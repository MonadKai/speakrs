use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};

use speakrs_sdk::{
    AhcConfigDto, BinarizeConfigDto, CancelToken, DiarizationResultDto, ExecutionModeDto,
    ModelPrepareError, ModelStore, PipelineConfigDto, PrepareModelsOptions, PreparedModels,
    ReconstructMethodDto, RuntimeConfigDto, SdkError, SdkPipeline, SdkQueue, SegmentDto,
    TimingStatsDto, VbxConfigDto, required_model_files,
};

uniffi::setup_scaffolding!();

#[derive(Debug, Clone, Copy, uniffi::Enum)]
pub enum ExecutionMode {
    Cpu,
    CoreMl,
    CoreMlFast,
    Cuda,
    CudaFast,
    MiGraphX,
}

#[derive(Debug, Clone, Copy, uniffi::Enum)]
pub enum ReconstructMethod {
    Standard,
    Smoothed,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct BinarizeConfig {
    pub onset: f32,
    pub offset: f32,
    pub min_duration_on: u64,
    pub min_duration_off: u64,
    pub pad_onset: u64,
    pub pad_offset: u64,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct AhcConfig {
    pub threshold: f32,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct VbxConfig {
    pub fa: f64,
    pub fb: f64,
    pub max_iters: u64,
    pub epsilon: f64,
    pub init_smoothing: f64,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct PipelineConfig {
    pub binarize: BinarizeConfig,
    pub ahc: AhcConfig,
    pub vbx: VbxConfig,
    pub merge_gap: f64,
    pub speaker_keep_threshold: f64,
    pub reconstruct_method: ReconstructMethod,
    pub reconstruct_epsilon: f32,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct RuntimeConfig {
    pub chunk_emb_workers: u64,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct Segment {
    pub start: f64,
    pub end: f64,
    pub speaker: String,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct TimingStats {
    pub model_prepare_ms: u64,
    pub audio_decode_ms: u64,
    pub audio_resample_ms: u64,
    pub pipeline_ms: u64,
    pub queue_wait_ms: u64,
    pub total_ms: u64,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct DiarizationResult {
    pub segments: Vec<Segment>,
    pub rttm: String,
    pub duration: f64,
    pub mode: ExecutionMode,
    pub model_revision: String,
    pub timing: TimingStats,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct ErrorInfo {
    pub category: String,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct QueueResult {
    pub job_id: u64,
    pub file_id: String,
    pub result: Option<DiarizationResult>,
    pub error: Option<ErrorInfo>,
}

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum SpeakrsError {
    #[error("{category}: {code}: {details}")]
    Failure {
        category: String,
        code: String,
        details: String,
    },
}

#[derive(uniffi::Object)]
pub struct SpeakrsCancelToken {
    inner: CancelToken,
}

#[uniffi::export]
impl SpeakrsCancelToken {
    #[uniffi::constructor]
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            inner: CancelToken::new(),
        })
    }

    pub fn cancel(&self) {
        self.inner.cancel();
    }

    pub fn is_cancelled(&self) -> bool {
        self.inner.is_cancelled()
    }
}

#[derive(uniffi::Object)]
pub struct PreparedModelsHandle {
    inner: PreparedModels,
}

#[uniffi::export]
impl PreparedModelsHandle {
    pub fn model_dir(&self) -> String {
        self.inner.model_dir.to_string_lossy().into_owned()
    }

    pub fn model_revision(&self) -> String {
        self.inner.manifest.revision.clone()
    }
}

#[derive(uniffi::Object)]
pub struct SpeakrsPipeline {
    inner: Mutex<Option<SdkPipeline>>,
}

#[uniffi::export]
impl SpeakrsPipeline {
    #[uniffi::constructor]
    pub fn from_prepared(
        prepared: Arc<PreparedModelsHandle>,
        mode: ExecutionMode,
        pipeline_config: Option<PipelineConfig>,
        runtime_config: Option<RuntimeConfig>,
    ) -> Result<Arc<Self>, SpeakrsError> {
        let pipeline = SdkPipeline::from_prepared(
            prepared.inner.clone(),
            mode.into(),
            pipeline_config.map(pipeline_config_dto).transpose()?,
            runtime_config.map(runtime_config_dto).transpose()?,
        )
        .map_err(SpeakrsError::from)?;

        Ok(Arc::new(Self {
            inner: Mutex::new(Some(pipeline)),
        }))
    }

    pub fn diarize_samples(
        &self,
        samples: Vec<f32>,
        file_id: String,
        pipeline_config: Option<PipelineConfig>,
        cancel_token: Option<Arc<SpeakrsCancelToken>>,
    ) -> Result<DiarizationResult, SpeakrsError> {
        let mut guard = self.pipeline_guard()?;
        let pipeline = guard.as_mut().ok_or_else(pipeline_consumed_error)?;
        pipeline
            .diarize_samples(
                &samples,
                speakrs_sdk::DiarizeSamplesOptions {
                    file_id: &file_id,
                    pipeline_config: pipeline_config.map(pipeline_config_dto).transpose()?,
                    cancel_token: cancel_token.as_ref().map(|token| &token.inner),
                    progress: None,
                },
            )
            .map(Into::into)
            .map_err(SpeakrsError::from)
    }

    pub fn diarize_file(
        &self,
        path: String,
        file_id: String,
        pipeline_config: Option<PipelineConfig>,
        cancel_token: Option<Arc<SpeakrsCancelToken>>,
    ) -> Result<DiarizationResult, SpeakrsError> {
        let mut guard = self.pipeline_guard()?;
        let pipeline = guard.as_mut().ok_or_else(pipeline_consumed_error)?;
        pipeline
            .diarize_file(
                PathBuf::from(path),
                speakrs_sdk::DiarizeFileOptions {
                    file_id: &file_id,
                    pipeline_config: pipeline_config.map(pipeline_config_dto).transpose()?,
                    cancel_token: cancel_token.as_ref().map(|token| &token.inner),
                    progress: None,
                },
            )
            .map(Into::into)
            .map_err(SpeakrsError::from)
    }

    pub fn into_queue(
        &self,
        pipeline_config: Option<PipelineConfig>,
    ) -> Result<Arc<SpeakrsQueue>, SpeakrsError> {
        let pipeline = self
            .inner
            .lock()
            .map_err(|_| lock_error("pipeline"))?
            .take()
            .ok_or_else(pipeline_consumed_error)?;
        let queue = pipeline
            .into_queue(pipeline_config.map(pipeline_config_dto).transpose()?)
            .map_err(SpeakrsError::from)?;

        Ok(Arc::new(SpeakrsQueue {
            inner: Mutex::new(queue),
        }))
    }
}

impl SpeakrsPipeline {
    fn pipeline_guard(&self) -> Result<MutexGuard<'_, Option<SdkPipeline>>, SpeakrsError> {
        let guard = self.inner.lock().map_err(|_| lock_error("pipeline"))?;
        if guard.is_none() {
            return Err(pipeline_consumed_error());
        }

        Ok(guard)
    }
}

#[derive(uniffi::Object)]
pub struct SpeakrsQueue {
    inner: Mutex<SdkQueue>,
}

#[uniffi::export]
impl SpeakrsQueue {
    pub fn push_samples(&self, file_id: String, samples: Vec<f32>) -> Result<u64, SpeakrsError> {
        self.inner
            .lock()
            .map_err(|_| lock_error("queue"))?
            .push_samples(file_id, samples)
            .map_err(SpeakrsError::from)
    }

    pub fn push_file(&self, file_id: String, path: String) -> Result<u64, SpeakrsError> {
        self.inner
            .lock()
            .map_err(|_| lock_error("queue"))?
            .push_file(file_id, PathBuf::from(path))
            .map_err(SpeakrsError::from)
    }

    pub fn recv(&self) -> Result<QueueResult, SpeakrsError> {
        self.inner
            .lock()
            .map_err(|_| lock_error("queue"))?
            .recv()
            .map(Into::into)
            .map_err(SpeakrsError::from)
    }

    pub fn try_recv(&self) -> Result<Option<QueueResult>, SpeakrsError> {
        self.inner
            .lock()
            .map_err(|_| lock_error("queue"))?
            .try_recv()
            .map(|result| result.map(Into::into))
            .map_err(SpeakrsError::from)
    }
}

#[uniffi::export]
pub fn default_model_revision() -> String {
    speakrs_sdk::DEFAULT_MODEL_REVISION.to_string()
}

#[uniffi::export]
pub fn default_cache_dir() -> String {
    ModelStore::default_cache_dir()
        .to_string_lossy()
        .into_owned()
}

#[uniffi::export]
pub fn default_pipeline_config(mode: ExecutionMode) -> PipelineConfig {
    PipelineConfigDto::for_mode(mode.into()).into()
}

#[uniffi::export]
pub fn default_runtime_config() -> RuntimeConfig {
    RuntimeConfigDto::default().into()
}

#[uniffi::export]
pub fn required_model_file_paths(mode: ExecutionMode) -> Vec<String> {
    required_model_files(mode.into())
}

#[uniffi::export]
pub fn prepare_models(
    mode: ExecutionMode,
    cache_dir: Option<String>,
    model_dir: Option<String>,
) -> Result<Arc<PreparedModelsHandle>, SpeakrsError> {
    let store = cache_dir.as_ref().map(ModelStore::new).unwrap_or_default();
    let mut options = PrepareModelsOptions::new(mode.into());
    options.cache_dir = cache_dir.map(PathBuf::from);
    options.model_dir = model_dir.map(PathBuf::from);
    options.manifest = options
        .model_dir
        .as_ref()
        .map(|path| store.generate_manifest(path, mode.into()))
        .transpose()
        .map_err(SpeakrsError::from)?;

    store
        .prepare(options)
        .map(|inner| Arc::new(PreparedModelsHandle { inner }))
        .map_err(SpeakrsError::from)
}

impl From<ExecutionMode> for ExecutionModeDto {
    fn from(value: ExecutionMode) -> Self {
        match value {
            ExecutionMode::Cpu => Self::Cpu,
            ExecutionMode::CoreMl => Self::CoreMl,
            ExecutionMode::CoreMlFast => Self::CoreMlFast,
            ExecutionMode::Cuda => Self::Cuda,
            ExecutionMode::CudaFast => Self::CudaFast,
            ExecutionMode::MiGraphX => Self::MiGraphX,
        }
    }
}

impl From<ExecutionModeDto> for ExecutionMode {
    fn from(value: ExecutionModeDto) -> Self {
        match value {
            ExecutionModeDto::Cpu => Self::Cpu,
            ExecutionModeDto::CoreMl => Self::CoreMl,
            ExecutionModeDto::CoreMlFast => Self::CoreMlFast,
            ExecutionModeDto::Cuda => Self::Cuda,
            ExecutionModeDto::CudaFast => Self::CudaFast,
            ExecutionModeDto::MiGraphX => Self::MiGraphX,
        }
    }
}

impl From<BinarizeConfigDto> for BinarizeConfig {
    fn from(value: BinarizeConfigDto) -> Self {
        Self {
            onset: value.onset,
            offset: value.offset,
            min_duration_on: value.min_duration_on as u64,
            min_duration_off: value.min_duration_off as u64,
            pad_onset: value.pad_onset as u64,
            pad_offset: value.pad_offset as u64,
        }
    }
}

impl TryFrom<BinarizeConfig> for BinarizeConfigDto {
    type Error = SpeakrsError;

    fn try_from(value: BinarizeConfig) -> Result<Self, Self::Error> {
        Ok(Self {
            onset: value.onset,
            offset: value.offset,
            min_duration_on: checked_usize(value.min_duration_on, "min_duration_on")?,
            min_duration_off: checked_usize(value.min_duration_off, "min_duration_off")?,
            pad_onset: checked_usize(value.pad_onset, "pad_onset")?,
            pad_offset: checked_usize(value.pad_offset, "pad_offset")?,
        })
    }
}

impl From<AhcConfigDto> for AhcConfig {
    fn from(value: AhcConfigDto) -> Self {
        Self {
            threshold: value.threshold,
        }
    }
}

impl From<AhcConfig> for AhcConfigDto {
    fn from(value: AhcConfig) -> Self {
        Self {
            threshold: value.threshold,
        }
    }
}

impl From<VbxConfigDto> for VbxConfig {
    fn from(value: VbxConfigDto) -> Self {
        Self {
            fa: value.fa,
            fb: value.fb,
            max_iters: value.max_iters as u64,
            epsilon: value.epsilon,
            init_smoothing: value.init_smoothing,
        }
    }
}

impl TryFrom<VbxConfig> for VbxConfigDto {
    type Error = SpeakrsError;

    fn try_from(value: VbxConfig) -> Result<Self, Self::Error> {
        Ok(Self {
            fa: value.fa,
            fb: value.fb,
            max_iters: checked_usize(value.max_iters, "max_iters")?,
            epsilon: value.epsilon,
            init_smoothing: value.init_smoothing,
        })
    }
}

impl From<PipelineConfigDto> for PipelineConfig {
    fn from(value: PipelineConfigDto) -> Self {
        let (reconstruct_method, reconstruct_epsilon) = match value.reconstruct_method {
            ReconstructMethodDto::Standard => (ReconstructMethod::Standard, 0.0),
            ReconstructMethodDto::Smoothed { epsilon } => (ReconstructMethod::Smoothed, epsilon),
        };
        Self {
            binarize: value.binarize.into(),
            ahc: value.ahc.into(),
            vbx: value.vbx.into(),
            merge_gap: value.merge_gap,
            speaker_keep_threshold: value.speaker_keep_threshold,
            reconstruct_method,
            reconstruct_epsilon,
        }
    }
}

impl From<RuntimeConfigDto> for RuntimeConfig {
    fn from(value: RuntimeConfigDto) -> Self {
        Self {
            chunk_emb_workers: value.chunk_emb_workers as u64,
        }
    }
}

impl From<SegmentDto> for Segment {
    fn from(value: SegmentDto) -> Self {
        Self {
            start: value.start,
            end: value.end,
            speaker: value.speaker,
        }
    }
}

impl From<TimingStatsDto> for TimingStats {
    fn from(value: TimingStatsDto) -> Self {
        Self {
            model_prepare_ms: value.model_prepare_ms,
            audio_decode_ms: value.audio_decode_ms,
            audio_resample_ms: value.audio_resample_ms,
            pipeline_ms: value.pipeline_ms,
            queue_wait_ms: value.queue_wait_ms,
            total_ms: value.total_ms,
        }
    }
}

impl From<DiarizationResultDto> for DiarizationResult {
    fn from(value: DiarizationResultDto) -> Self {
        Self {
            segments: value.segments.into_iter().map(Into::into).collect(),
            rttm: value.rttm,
            duration: value.duration,
            mode: value.mode.into(),
            model_revision: value.model_revision,
            timing: value.timing.into(),
        }
    }
}

impl From<speakrs_sdk::SdkQueueResult> for QueueResult {
    fn from(value: speakrs_sdk::SdkQueueResult) -> Self {
        match value.result {
            Ok(result) => Self {
                job_id: value.job_id,
                file_id: value.file_id,
                result: Some(result.into()),
                error: None,
            },
            Err(error) => Self {
                job_id: value.job_id,
                file_id: value.file_id,
                result: None,
                error: Some(error.into()),
            },
        }
    }
}

impl From<SdkError> for ErrorInfo {
    fn from(value: SdkError) -> Self {
        Self {
            category: value.category.code().to_string(),
            code: value.code,
            message: value.message,
        }
    }
}

impl From<SdkError> for SpeakrsError {
    fn from(value: SdkError) -> Self {
        let info: ErrorInfo = value.into();
        Self::Failure {
            category: info.category,
            code: info.code,
            details: info.message,
        }
    }
}

impl From<ModelPrepareError> for SpeakrsError {
    fn from(value: ModelPrepareError) -> Self {
        Self::Failure {
            category: "model_prepare".to_string(),
            code: "model_prepare".to_string(),
            details: value.to_string(),
        }
    }
}

impl From<speakrs_sdk::ModelManifestError> for SpeakrsError {
    fn from(value: speakrs_sdk::ModelManifestError) -> Self {
        Self::Failure {
            category: "model_manifest".to_string(),
            code: "model_manifest".to_string(),
            details: value.to_string(),
        }
    }
}

fn checked_usize(value: u64, field: &str) -> Result<usize, SpeakrsError> {
    value.try_into().map_err(|_| SpeakrsError::Failure {
        category: "configuration".to_string(),
        code: "configuration".to_string(),
        details: format!("{field} is too large for this platform"),
    })
}

fn pipeline_config_dto(value: PipelineConfig) -> Result<PipelineConfigDto, SpeakrsError> {
    Ok(PipelineConfigDto {
        binarize: value.binarize.try_into()?,
        ahc: value.ahc.into(),
        vbx: value.vbx.try_into()?,
        merge_gap: value.merge_gap,
        speaker_keep_threshold: value.speaker_keep_threshold,
        reconstruct_method: match value.reconstruct_method {
            ReconstructMethod::Standard => ReconstructMethodDto::Standard,
            ReconstructMethod::Smoothed => ReconstructMethodDto::Smoothed {
                epsilon: value.reconstruct_epsilon,
            },
        },
    })
}

fn runtime_config_dto(value: RuntimeConfig) -> Result<RuntimeConfigDto, SpeakrsError> {
    Ok(RuntimeConfigDto {
        chunk_emb_workers: checked_usize(value.chunk_emb_workers, "chunk_emb_workers")?,
    })
}

fn lock_error(name: &str) -> SpeakrsError {
    SpeakrsError::Failure {
        category: "internal".to_string(),
        code: "internal".to_string(),
        details: format!("{name} lock was poisoned"),
    }
}

fn pipeline_consumed_error() -> SpeakrsError {
    SpeakrsError::Failure {
        category: "pipeline".to_string(),
        code: "pipeline_consumed".to_string(),
        details: "pipeline was converted to a queue".to_string(),
    }
}
