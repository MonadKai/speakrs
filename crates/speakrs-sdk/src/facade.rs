use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};
use std::time::Instant;

use speakrs::pipeline::{
    OwnedDiarizationPipeline, PipelineBuilder, QueueReceiver, QueueSender, QueuedDiarizationRequest,
};

use crate::audio::{decode_file_to_mono, decode_file_to_mono_16khz, resample_mono_to_16khz};
use crate::cancel::CancelToken;
use crate::dto::{
    DiarizationResultDto, ExecutionModeDto, PipelineConfigDto, ProgressEvent, RuntimeConfigDto,
    TimingStatsDto,
};
use crate::error::{SdkError, SdkErrorCategory};
use crate::models::PreparedModels;

pub type ProgressCallback<'a> = dyn Fn(ProgressEvent) + Send + Sync + 'a;

pub struct SdkPipeline {
    pipeline: OwnedDiarizationPipeline,
    mode: ExecutionModeDto,
    model_revision: String,
}

pub struct SdkQueue {
    sender: QueueSender,
    receiver: Mutex<QueueReceiver>,
    mode: ExecutionModeDto,
    model_revision: String,
}

pub struct SdkQueueResult {
    pub job_id: u64,
    pub file_id: String,
    pub result: Result<DiarizationResultDto, SdkError>,
}

pub struct DiarizeSamplesOptions<'a> {
    pub file_id: &'a str,
    pub pipeline_config: Option<PipelineConfigDto>,
    pub cancel_token: Option<&'a CancelToken>,
    pub progress: Option<&'a ProgressCallback<'a>>,
}

impl Default for DiarizeSamplesOptions<'_> {
    fn default() -> Self {
        Self {
            file_id: "file1",
            pipeline_config: None,
            cancel_token: None,
            progress: None,
        }
    }
}

pub struct DiarizeFileOptions<'a> {
    pub file_id: &'a str,
    pub pipeline_config: Option<PipelineConfigDto>,
    pub cancel_token: Option<&'a CancelToken>,
    pub progress: Option<&'a ProgressCallback<'a>>,
}

impl Default for DiarizeFileOptions<'_> {
    fn default() -> Self {
        Self {
            file_id: "file1",
            pipeline_config: None,
            cancel_token: None,
            progress: None,
        }
    }
}

impl SdkPipeline {
    pub fn from_prepared(
        prepared: PreparedModels,
        mode: ExecutionModeDto,
        pipeline_config: Option<PipelineConfigDto>,
        runtime_config: Option<RuntimeConfigDto>,
    ) -> Result<Self, SdkError> {
        let mut builder = PipelineBuilder::from_dir(&prepared.model_dir, mode.into());
        if let Some(config) = pipeline_config {
            builder = builder.pipeline(config.into());
        }
        if let Some(config) = runtime_config {
            builder = builder.runtime(config.into());
        }

        let pipeline = builder.build().map_err(SdkError::from_pipeline_error)?;
        Ok(Self {
            pipeline,
            mode,
            model_revision: prepared.manifest.revision,
        })
    }

    pub fn diarize_samples(
        &mut self,
        samples: &[f32],
        options: DiarizeSamplesOptions<'_>,
    ) -> Result<DiarizationResultDto, SdkError> {
        check_cancelled(options.cancel_token)?;
        emit(options.progress, ProgressEvent::RunningPipeline);
        let pipeline_start = Instant::now();
        let config = options
            .pipeline_config
            .unwrap_or_else(|| PipelineConfigDto::for_mode(self.mode));
        let result = self
            .pipeline
            .run_with_config(samples, options.file_id, &config.into())
            .map_err(SdkError::from_pipeline_error)?;
        check_cancelled(options.cancel_token)?;
        let pipeline_ms = elapsed_ms(pipeline_start);
        emit(options.progress, ProgressEvent::Completed);

        Ok(map_result(
            result,
            options.file_id,
            samples.len() as f64 / 16_000.0,
            self.mode,
            &self.model_revision,
            TimingStatsDto {
                pipeline_ms,
                total_ms: pipeline_ms,
                ..TimingStatsDto::default()
            },
        ))
    }

    pub fn diarize_file(
        &mut self,
        path: impl Into<PathBuf>,
        options: DiarizeFileOptions<'_>,
    ) -> Result<DiarizationResultDto, SdkError> {
        check_cancelled(options.cancel_token)?;
        emit(options.progress, ProgressEvent::DecodingAudio);
        let decode_start = Instant::now();
        let audio = decode_file_to_mono(path.into()).map_err(SdkError::from_audio_error)?;
        let audio_decode_ms = elapsed_ms(decode_start);
        check_cancelled(options.cancel_token)?;
        emit(options.progress, ProgressEvent::ResamplingAudio);
        let resample_start = Instant::now();
        let samples = resample_mono_to_16khz(&audio.samples, audio.sample_rate_hz)
            .map_err(SdkError::from_audio_error)?;
        let audio_resample_ms = elapsed_ms(resample_start);

        self.diarize_samples(
            &samples,
            DiarizeSamplesOptions {
                file_id: options.file_id,
                pipeline_config: options.pipeline_config,
                cancel_token: options.cancel_token,
                progress: options.progress,
            },
        )
        .map(|mut result| {
            result.timing.audio_decode_ms = audio_decode_ms;
            result.timing.audio_resample_ms = audio_resample_ms;
            result.timing.total_ms += audio_decode_ms;
            result.timing.total_ms += audio_resample_ms;
            result
        })
    }

    pub fn into_queue(
        self,
        pipeline_config: Option<PipelineConfigDto>,
    ) -> Result<SdkQueue, SdkError> {
        let config = pipeline_config.unwrap_or_else(|| PipelineConfigDto::for_mode(self.mode));
        let (sender, receiver) = self
            .pipeline
            .into_queued_with_config(config.into())
            .map_err(SdkError::from_queue_error)?;

        Ok(SdkQueue {
            sender,
            receiver: Mutex::new(receiver),
            mode: self.mode,
            model_revision: self.model_revision,
        })
    }
}

impl SdkQueue {
    pub fn push_samples(
        &self,
        file_id: impl Into<String>,
        samples: Vec<f32>,
    ) -> Result<u64, SdkError> {
        let job_id = self
            .sender
            .push(QueuedDiarizationRequest::new(file_id, samples))
            .map_err(SdkError::from_queue_error)?;
        Ok(job_id.as_u64())
    }

    pub fn push_file(
        &self,
        file_id: impl Into<String>,
        path: impl Into<PathBuf>,
    ) -> Result<u64, SdkError> {
        let audio = decode_file_to_mono_16khz(path.into()).map_err(SdkError::from_audio_error)?;
        self.push_samples(file_id, audio.samples)
    }

    pub fn recv(&self) -> Result<SdkQueueResult, SdkError> {
        let result = self
            .receiver_guard()?
            .recv()
            .map_err(SdkError::from_queue_error)?;
        Ok(self.map_queue_result(result))
    }

    pub fn try_recv(&self) -> Result<Option<SdkQueueResult>, SdkError> {
        self.receiver_guard()?
            .try_recv()
            .map_err(SdkError::from_queue_error)?
            .map(|result| Ok(self.map_queue_result(result)))
            .transpose()
    }

    fn receiver_guard(&self) -> Result<MutexGuard<'_, QueueReceiver>, SdkError> {
        self.receiver.lock().map_err(|_| {
            SdkError::new(
                SdkErrorCategory::Internal,
                SdkErrorCategory::Internal.code(),
                "queue receiver lock was poisoned",
            )
        })
    }

    fn map_queue_result(
        &self,
        result: speakrs::pipeline::QueuedDiarizationResult,
    ) -> SdkQueueResult {
        let file_id = result.file_id;
        let job_id = result.job_id.as_u64();
        let duration = result.duration;
        let result = result
            .result
            .map(|result| {
                map_result(
                    result,
                    &file_id,
                    duration,
                    self.mode,
                    &self.model_revision,
                    TimingStatsDto::default(),
                )
            })
            .map_err(SdkError::from_pipeline_error);

        SdkQueueResult {
            job_id,
            file_id,
            result,
        }
    }
}

impl SdkError {
    pub(crate) fn from_pipeline_error(error: speakrs::pipeline::PipelineError) -> Self {
        let message = error.to_string();
        let category = if message.contains("requires the `") {
            SdkErrorCategory::UnsupportedRuntime
        } else {
            SdkErrorCategory::Pipeline
        };

        Self::new(category, category.code(), message)
    }

    pub(crate) fn from_audio_error(error: crate::audio::AudioDecodeError) -> Self {
        Self::new(
            SdkErrorCategory::AudioDecode,
            SdkErrorCategory::AudioDecode.code(),
            error.to_string(),
        )
    }

    pub(crate) fn from_queue_error(error: speakrs::pipeline::QueueError) -> Self {
        Self::new(
            SdkErrorCategory::Queue,
            SdkErrorCategory::Queue.code(),
            error.to_string(),
        )
    }
}

fn map_result(
    result: speakrs::pipeline::DiarizationResult,
    file_id: &str,
    duration: f64,
    mode: ExecutionModeDto,
    model_revision: &str,
    timing: TimingStatsDto,
) -> DiarizationResultDto {
    let rttm = result.rttm(file_id);
    let segments = result.segments.into_iter().map(Into::into).collect();
    DiarizationResultDto {
        segments,
        rttm,
        duration,
        mode,
        model_revision: model_revision.to_string(),
        timing,
    }
}

fn check_cancelled(cancel_token: Option<&CancelToken>) -> Result<(), SdkError> {
    if let Some(token) = cancel_token
        && token.is_cancelled()
    {
        return Err(SdkError::cancelled());
    }

    Ok(())
}

fn emit(progress: Option<&ProgressCallback<'_>>, event: ProgressEvent) {
    if let Some(progress) = progress {
        progress(event);
    }
}

fn elapsed_ms(start: Instant) -> u64 {
    start.elapsed().as_millis().try_into().unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex, mpsc};

    use crate::audio::decode_file_to_mono_16khz;
    use crate::models::{DEFAULT_MODEL_REVISION, ModelManifest, ModelStore, PrepareModelsOptions};

    use super::*;

    #[test]
    fn cancelled_samples_run_returns_typed_error_before_pipeline_work() {
        let token = CancelToken::new();
        token.cancel();

        let err = check_cancelled(Some(&token)).unwrap_err();

        assert_eq!(err.category, SdkErrorCategory::Cancelled);
        assert_eq!(err.code, "cancelled");
    }

    #[test]
    fn result_mapping_keeps_only_stable_public_fields() {
        let result = speakrs::pipeline::DiarizationResult {
            segmentations: speakrs::pipeline::DecodedSegmentations(Default::default()),
            embeddings: speakrs::pipeline::ChunkEmbeddings(Default::default()),
            speaker_count: speakrs::pipeline::SpeakerCountTrack(vec![]),
            hard_clusters: speakrs::pipeline::ChunkSpeakerClusters(Default::default()),
            discrete_diarization: speakrs::pipeline::DiscreteDiarization(Default::default()),
            segments: vec![speakrs::segment::Segment::new(0.0, 1.5, "SPEAKER_00")],
        };

        let mapped = map_result(
            result,
            "file",
            1.5,
            ExecutionModeDto::Cpu,
            DEFAULT_MODEL_REVISION,
            TimingStatsDto::default(),
        );

        assert_eq!(mapped.segments.len(), 1);
        assert_eq!(
            mapped.rttm,
            "SPEAKER file 1 0.000000 1.500000 <NA> <NA> SPEAKER_00 <NA> <NA>\n"
        );
        assert_eq!(mapped.duration, 1.5);
        assert_eq!(mapped.model_revision, DEFAULT_MODEL_REVISION);
    }

    #[test]
    fn prepared_models_feed_pipeline_builder_path_contract() {
        let prepared = PreparedModels {
            model_dir: PathBuf::from("/tmp/speakrs-models"),
            manifest: ModelManifest {
                repository: "repo".to_string(),
                revision: "rev".to_string(),
                files: vec![],
            },
        };

        assert_eq!(prepared.model_dir, PathBuf::from("/tmp/speakrs-models"));
        assert_eq!(prepared.manifest.revision, "rev");
    }

    #[test]
    fn queue_error_maps_to_stable_queue_category() {
        let err = SdkError::from_queue_error(speakrs::pipeline::QueueError::Closed);

        assert_eq!(err.category, SdkErrorCategory::Queue);
        assert_eq!(err.code, "queue");
    }

    #[test]
    fn unsupported_execution_modes_map_to_stable_runtime_category() {
        let unsupported_modes = [
            (ExecutionModeDto::CoreMl, "coreml"),
            (ExecutionModeDto::CoreMlFast, "coreml-fast"),
            (ExecutionModeDto::Cuda, "cuda"),
            (ExecutionModeDto::CudaFast, "cuda-fast"),
            (ExecutionModeDto::MiGraphX, "migraphx"),
        ];

        for (mode, mode_name) in unsupported_modes {
            let err = match SdkPipeline::from_prepared(dummy_prepared_models(), mode, None, None) {
                Ok(_) => panic!("unsupported mode should fail before loading models"),
                Err(err) => err,
            };

            assert_eq!(err.category, SdkErrorCategory::UnsupportedRuntime);
            assert_eq!(err.code, "unsupported_runtime");
            assert!(
                err.message.contains(mode_name),
                "missing mode name in {err:?}"
            );
            assert!(
                err.message.contains("Cargo feature"),
                "missing feature guidance in {err:?}"
            );
        }
    }

    #[test]
    fn fixture_prepared_pipeline_diarizes_samples_and_files() {
        let prepared = fixture_prepared_models();
        let wav_path = fixture_path("test_short.wav");
        let decoded = decode_file_to_mono_16khz(&wav_path).unwrap();
        let progress_events = Arc::new(Mutex::new(Vec::new()));
        let callback_events = Arc::clone(&progress_events);
        let progress = move |event| callback_events.lock().unwrap().push(event);
        let mut pipeline =
            SdkPipeline::from_prepared(prepared, ExecutionModeDto::Cpu, None, None).unwrap();

        let samples_result = pipeline
            .diarize_samples(
                &decoded.samples,
                DiarizeSamplesOptions {
                    file_id: "samples",
                    progress: Some(&progress),
                    ..Default::default()
                },
            )
            .unwrap();
        let file_result = pipeline
            .diarize_file(
                &wav_path,
                DiarizeFileOptions {
                    file_id: "file",
                    progress: Some(&progress),
                    ..Default::default()
                },
            )
            .unwrap();

        assert_eq!(samples_result.mode, ExecutionModeDto::Cpu);
        assert_eq!(samples_result.model_revision, DEFAULT_MODEL_REVISION);
        assert!(samples_result.duration > 0.0);
        assert!(samples_result.timing.total_ms >= samples_result.timing.pipeline_ms);
        assert!(file_result.timing.total_ms >= file_result.timing.audio_decode_ms);
        assert!(file_result.duration > 0.0);
        let events = progress_events.lock().unwrap();
        assert!(events.contains(&ProgressEvent::DecodingAudio));
        assert!(events.contains(&ProgressEvent::ResamplingAudio));
        assert!(events.contains(&ProgressEvent::RunningPipeline));
        assert!(events.contains(&ProgressEvent::Completed));
    }

    #[test]
    fn fixture_queue_accepts_sample_and_path_jobs() {
        let prepared = fixture_prepared_models();
        let wav_path = fixture_path("test_short.wav");
        let decoded = decode_file_to_mono_16khz(&wav_path).unwrap();
        let pipeline =
            SdkPipeline::from_prepared(prepared, ExecutionModeDto::Cpu, None, None).unwrap();
        let queue = pipeline.into_queue(None).unwrap();

        let sample_job = queue
            .push_samples("sample-job", decoded.samples.clone())
            .unwrap();
        let path_job = queue.push_file("path-job", wav_path).unwrap();

        assert_eq!(sample_job, 0);
        assert_eq!(path_job, 1);

        let first = queue.recv().unwrap();
        let second = queue.recv().unwrap();
        let mut ids = [first.job_id, second.job_id];
        ids.sort();
        assert_eq!(ids, [0, 1]);
        assert!(first.result.is_ok());
        assert!(second.result.is_ok());
        assert!(first.result.as_ref().unwrap().duration > 0.0);
        assert!(second.result.as_ref().unwrap().duration > 0.0);
    }

    #[test]
    fn fixture_queue_accepts_push_while_recv_waits_on_another_thread() {
        let prepared = fixture_prepared_models();
        let wav_path = fixture_path("test_short.wav");
        let decoded = decode_file_to_mono_16khz(&wav_path).unwrap();
        let pipeline =
            SdkPipeline::from_prepared(prepared, ExecutionModeDto::Cpu, None, None).unwrap();
        let queue = Arc::new(pipeline.into_queue(None).unwrap());
        let receiver_queue = Arc::clone(&queue);
        let (waiting_tx, waiting_rx) = mpsc::channel();
        let receiver = std::thread::spawn(move || {
            waiting_tx.send(()).unwrap();
            receiver_queue.recv()
        });

        waiting_rx.recv().unwrap();
        let job_id = queue.push_samples("concurrent", decoded.samples).unwrap();
        let result = receiver.join().unwrap().unwrap();

        assert_eq!(result.job_id, job_id);
        assert_eq!(result.file_id, "concurrent");
        assert!(result.result.as_ref().unwrap().duration > 0.0);
    }

    fn fixture_prepared_models() -> PreparedModels {
        let model_dir = fixture_path("models");
        let store = ModelStore::new(std::env::temp_dir().join("speakrs-sdk-fixture-cache"));
        let manifest = store
            .generate_manifest(&model_dir, ExecutionModeDto::Cpu)
            .unwrap();

        store
            .prepare(PrepareModelsOptions {
                mode: ExecutionModeDto::Cpu,
                cache_dir: None,
                model_dir: Some(model_dir),
                manifest: Some(manifest),
            })
            .unwrap()
    }

    fn dummy_prepared_models() -> PreparedModels {
        PreparedModels {
            model_dir: PathBuf::from("/tmp/speakrs-sdk-unavailable-models"),
            manifest: ModelManifest {
                repository: "repo".to_string(),
                revision: "rev".to_string(),
                files: vec![],
            },
        }
    }

    fn fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("fixtures")
            .join(name)
    }
}
