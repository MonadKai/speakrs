#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionModeDto {
    Cpu,
    CoreMl,
    CoreMlFast,
    Cuda,
    CudaFast,
    MiGraphX,
}

impl ExecutionModeDto {
    pub const fn wire_name(self) -> &'static str {
        match self {
            Self::Cpu => "CPU",
            Self::CoreMl => "CoreML",
            Self::CoreMlFast => "CoreMLFast",
            Self::Cuda => "CUDA",
            Self::CudaFast => "CUDAFast",
            Self::MiGraphX => "MIGraphX",
        }
    }
}

#[cfg(any(
    feature = "pipeline",
    feature = "android-pipeline",
    feature = "ios-coreml"
))]
impl From<ExecutionModeDto> for speakrs::inference::ExecutionMode {
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

#[derive(Debug, Clone, PartialEq)]
pub struct BinarizeConfigDto {
    pub onset: f32,
    pub offset: f32,
    pub min_duration_on: usize,
    pub min_duration_off: usize,
    pub pad_onset: usize,
    pub pad_offset: usize,
}

impl Default for BinarizeConfigDto {
    fn default() -> Self {
        Self {
            onset: 0.5,
            offset: 0.5,
            min_duration_on: 0,
            min_duration_off: 0,
            pad_onset: 0,
            pad_offset: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AhcConfigDto {
    pub threshold: f32,
}

impl Default for AhcConfigDto {
    fn default() -> Self {
        Self { threshold: 0.6 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VbxConfigDto {
    pub fa: f64,
    pub fb: f64,
    pub max_iters: usize,
    pub epsilon: f64,
    pub init_smoothing: f64,
}

impl Default for VbxConfigDto {
    fn default() -> Self {
        Self {
            fa: 0.07,
            fb: 0.8,
            max_iters: 20,
            epsilon: 1e-4,
            init_smoothing: 7.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ReconstructMethodDto {
    Standard,
    Smoothed { epsilon: f32 },
}

impl Default for ReconstructMethodDto {
    fn default() -> Self {
        Self::Smoothed { epsilon: 0.1 }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PipelineConfigDto {
    pub binarize: BinarizeConfigDto,
    pub ahc: AhcConfigDto,
    pub vbx: VbxConfigDto,
    pub merge_gap: f64,
    pub speaker_keep_threshold: f64,
    pub reconstruct_method: ReconstructMethodDto,
}

impl Default for PipelineConfigDto {
    fn default() -> Self {
        Self {
            binarize: BinarizeConfigDto::default(),
            ahc: AhcConfigDto::default(),
            vbx: VbxConfigDto::default(),
            merge_gap: 0.0,
            speaker_keep_threshold: 1e-7,
            reconstruct_method: ReconstructMethodDto::default(),
        }
    }
}

impl PipelineConfigDto {
    pub fn for_mode(mode: ExecutionModeDto) -> Self {
        match mode {
            ExecutionModeDto::CoreMlFast | ExecutionModeDto::CudaFast => Self {
                binarize: BinarizeConfigDto {
                    min_duration_on: 3,
                    min_duration_off: 3,
                    ..BinarizeConfigDto::default()
                },
                vbx: VbxConfigDto {
                    max_iters: 3,
                    ..VbxConfigDto::default()
                },
                ..Self::default()
            },
            _ => Self::default(),
        }
    }
}

#[cfg(any(
    feature = "pipeline",
    feature = "android-pipeline",
    feature = "ios-coreml"
))]
impl From<PipelineConfigDto> for speakrs::pipeline::PipelineConfig {
    fn from(value: PipelineConfigDto) -> Self {
        Self {
            binarize: value.binarize.into(),
            ahc: value.ahc.into(),
            vbx: value.vbx.into(),
            merge_gap: value.merge_gap,
            speaker_keep_threshold: value.speaker_keep_threshold,
            reconstruct_method: value.reconstruct_method.into(),
        }
    }
}

#[cfg(any(
    feature = "pipeline",
    feature = "android-pipeline",
    feature = "ios-coreml"
))]
impl From<BinarizeConfigDto> for speakrs::pipeline::BinarizeConfig {
    fn from(value: BinarizeConfigDto) -> Self {
        Self {
            onset: value.onset,
            offset: value.offset,
            min_duration_on: value.min_duration_on,
            min_duration_off: value.min_duration_off,
            pad_onset: value.pad_onset,
            pad_offset: value.pad_offset,
        }
    }
}

#[cfg(any(
    feature = "pipeline",
    feature = "android-pipeline",
    feature = "ios-coreml"
))]
impl From<AhcConfigDto> for speakrs::pipeline::AhcConfig {
    fn from(value: AhcConfigDto) -> Self {
        Self {
            threshold: value.threshold,
        }
    }
}

#[cfg(any(
    feature = "pipeline",
    feature = "android-pipeline",
    feature = "ios-coreml"
))]
impl From<VbxConfigDto> for speakrs::pipeline::VbxConfig {
    fn from(value: VbxConfigDto) -> Self {
        Self {
            fa: value.fa,
            fb: value.fb,
            max_iters: value.max_iters,
            epsilon: value.epsilon,
            init_smoothing: value.init_smoothing,
        }
    }
}

#[cfg(any(
    feature = "pipeline",
    feature = "android-pipeline",
    feature = "ios-coreml"
))]
impl From<ReconstructMethodDto> for speakrs::pipeline::ReconstructMethod {
    fn from(value: ReconstructMethodDto) -> Self {
        match value {
            ReconstructMethodDto::Standard => Self::Standard,
            ReconstructMethodDto::Smoothed { epsilon } => Self::Smoothed { epsilon },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeConfigDto {
    pub chunk_emb_workers: usize,
}

impl Default for RuntimeConfigDto {
    fn default() -> Self {
        Self {
            chunk_emb_workers: 1,
        }
    }
}

#[cfg(any(
    feature = "pipeline",
    feature = "android-pipeline",
    feature = "ios-coreml"
))]
impl From<RuntimeConfigDto> for speakrs::pipeline::RuntimeConfig {
    fn from(value: RuntimeConfigDto) -> Self {
        #[cfg(any(feature = "coreml", feature = "ios-coreml"))]
        {
            Self {
                chunk_emb_workers: value.chunk_emb_workers,
                ..Default::default()
            }
        }

        #[cfg(not(any(feature = "coreml", feature = "ios-coreml")))]
        {
            Self {
                chunk_emb_workers: value.chunk_emb_workers,
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SegmentDto {
    pub start: f64,
    pub end: f64,
    pub speaker: String,
}

impl SegmentDto {
    pub fn duration(&self) -> f64 {
        self.end - self.start
    }
}

#[cfg(any(
    feature = "pipeline",
    feature = "android-pipeline",
    feature = "ios-coreml"
))]
impl From<speakrs::segment::Segment> for SegmentDto {
    fn from(value: speakrs::segment::Segment) -> Self {
        Self {
            start: value.start,
            end: value.end,
            speaker: value.speaker,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TimingStatsDto {
    pub model_prepare_ms: u64,
    pub audio_decode_ms: u64,
    pub audio_resample_ms: u64,
    pub pipeline_ms: u64,
    pub queue_wait_ms: u64,
    pub total_ms: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DiarizationResultDto {
    pub segments: Vec<SegmentDto>,
    pub rttm: String,
    pub duration: f64,
    pub mode: ExecutionModeDto,
    pub model_revision: String,
    pub timing: TimingStatsDto,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProgressEvent {
    PreparingModels,
    DownloadingModel { path: String },
    VerifyingModelManifest,
    DecodingAudio,
    ResamplingAudio,
    RunningPipeline,
    QueueWait { job_id: u64 },
    Completed,
}

#[cfg(test)]
mod tests {
    #[cfg(any(
        feature = "pipeline",
        feature = "android-pipeline",
        feature = "ios-coreml"
    ))]
    use speakrs::inference::ExecutionMode;
    #[cfg(any(
        feature = "pipeline",
        feature = "android-pipeline",
        feature = "ios-coreml"
    ))]
    use speakrs::pipeline::{
        PipelineConfig, ReconstructMethod, RuntimeConfig, segmentation_step_seconds,
    };

    use super::*;

    #[test]
    fn execution_modes_have_required_wire_names() {
        let names: Vec<_> = [
            ExecutionModeDto::Cpu,
            ExecutionModeDto::CoreMl,
            ExecutionModeDto::CoreMlFast,
            ExecutionModeDto::Cuda,
            ExecutionModeDto::CudaFast,
            ExecutionModeDto::MiGraphX,
        ]
        .into_iter()
        .map(ExecutionModeDto::wire_name)
        .collect();

        assert_eq!(
            names,
            [
                "CPU",
                "CoreML",
                "CoreMLFast",
                "CUDA",
                "CUDAFast",
                "MIGraphX"
            ]
        );
    }

    #[test]
    #[cfg(any(
        feature = "pipeline",
        feature = "android-pipeline",
        feature = "ios-coreml"
    ))]
    fn pipeline_config_default_matches_core_default() {
        assert_config_matches_core(&PipelineConfigDto::default(), &PipelineConfig::default());
    }

    #[test]
    #[cfg(any(
        feature = "pipeline",
        feature = "android-pipeline",
        feature = "ios-coreml"
    ))]
    fn fast_mode_defaults_match_core_defaults() {
        let pairs = [
            (
                ExecutionModeDto::CoreMlFast,
                PipelineConfig::for_mode(ExecutionMode::CoreMlFast),
            ),
            (
                ExecutionModeDto::CudaFast,
                PipelineConfig::for_mode(ExecutionMode::CudaFast),
            ),
        ];

        for (mode, core) in pairs {
            assert_config_matches_core(&PipelineConfigDto::for_mode(mode), &core);
        }
    }

    #[test]
    #[cfg(any(
        feature = "pipeline",
        feature = "android-pipeline",
        feature = "ios-coreml"
    ))]
    fn runtime_config_default_matches_core_default() {
        let dto = RuntimeConfigDto::default();
        let core = RuntimeConfig::default();

        assert_eq!(dto.chunk_emb_workers, core.chunk_emb_workers);
    }

    #[test]
    fn timing_stats_include_required_fields() {
        let timing = TimingStatsDto {
            model_prepare_ms: 1,
            audio_decode_ms: 2,
            audio_resample_ms: 3,
            pipeline_ms: 4,
            queue_wait_ms: 5,
            total_ms: 15,
        };

        assert_eq!(timing.total_ms, 15);
    }

    #[test]
    #[cfg(any(
        feature = "pipeline",
        feature = "android-pipeline",
        feature = "ios-coreml"
    ))]
    fn dto_modes_preserve_core_segmentation_step_groups() {
        assert_eq!(
            segmentation_step_seconds(ExecutionMode::Cpu),
            segmentation_step_seconds(ExecutionMode::MiGraphX)
        );
        assert_eq!(
            PipelineConfigDto::for_mode(ExecutionModeDto::CudaFast)
                .binarize
                .min_duration_on,
            PipelineConfigDto::for_mode(ExecutionModeDto::CoreMlFast)
                .binarize
                .min_duration_on
        );
    }

    #[cfg(any(
        feature = "pipeline",
        feature = "android-pipeline",
        feature = "ios-coreml"
    ))]
    fn assert_config_matches_core(dto: &PipelineConfigDto, core: &PipelineConfig) {
        assert_eq!(dto.binarize.onset, core.binarize.onset);
        assert_eq!(dto.binarize.offset, core.binarize.offset);
        assert_eq!(dto.binarize.min_duration_on, core.binarize.min_duration_on);
        assert_eq!(
            dto.binarize.min_duration_off,
            core.binarize.min_duration_off
        );
        assert_eq!(dto.binarize.pad_onset, core.binarize.pad_onset);
        assert_eq!(dto.binarize.pad_offset, core.binarize.pad_offset);
        assert_eq!(dto.ahc.threshold, core.ahc.threshold);
        assert_eq!(dto.vbx.fa, core.vbx.fa);
        assert_eq!(dto.vbx.fb, core.vbx.fb);
        assert_eq!(dto.vbx.max_iters, core.vbx.max_iters);
        assert_eq!(dto.vbx.epsilon, core.vbx.epsilon);
        assert_eq!(dto.vbx.init_smoothing, core.vbx.init_smoothing);
        assert_eq!(dto.merge_gap, core.merge_gap);
        assert_eq!(dto.speaker_keep_threshold, core.speaker_keep_threshold);
        assert_reconstruct_matches_core(dto.reconstruct_method, core.reconstruct_method);
    }

    #[cfg(any(
        feature = "pipeline",
        feature = "android-pipeline",
        feature = "ios-coreml"
    ))]
    fn assert_reconstruct_matches_core(dto: ReconstructMethodDto, core: ReconstructMethod) {
        match (dto, core) {
            (ReconstructMethodDto::Standard, ReconstructMethod::Standard) => {}
            (
                ReconstructMethodDto::Smoothed { epsilon: lhs },
                ReconstructMethod::Smoothed { epsilon: rhs },
            ) => assert_eq!(lhs, rhs),
            _ => panic!("reconstruct method mismatch"),
        }
    }
}
