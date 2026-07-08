//! Shared binding-safe SDK substrate for speakrs wrappers.

pub mod audio;
pub mod cancel;
pub mod dto;
pub mod error;
#[cfg(any(
    feature = "pipeline",
    feature = "android-pipeline",
    feature = "ios-coreml"
))]
pub mod facade;
pub mod models;

pub use audio::{AudioDecodeError, AudioFormat, DecodedAudio, ResamplePlan};
pub use cancel::CancelToken;
pub use dto::{
    AhcConfigDto, BinarizeConfigDto, DiarizationResultDto, ExecutionModeDto, PipelineConfigDto,
    ProgressEvent, ReconstructMethodDto, RuntimeConfigDto, SegmentDto, TimingStatsDto,
    VbxConfigDto,
};
pub use error::{SdkError, SdkErrorCategory};
#[cfg(any(
    feature = "pipeline",
    feature = "android-pipeline",
    feature = "ios-coreml"
))]
pub use facade::{
    DiarizeFileOptions, DiarizeSamplesOptions, ProgressCallback, SdkPipeline, SdkQueue,
    SdkQueueResult,
};
pub use models::{
    CacheEntry, DEFAULT_MODEL_REPOSITORY, DEFAULT_MODEL_REVISION, ModelManifest,
    ModelManifestEntry, ModelManifestError, ModelManifestPlan, ModelPrepareError, ModelStore,
    PrepareModelsOptions, PreparedModels, required_model_files,
};
