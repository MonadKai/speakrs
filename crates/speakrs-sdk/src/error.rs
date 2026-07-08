use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SdkErrorCategory {
    Configuration,
    UnsupportedRuntime,
    ModelPrepare,
    ModelManifest,
    AudioDecode,
    AudioResample,
    Pipeline,
    Queue,
    Cancelled,
    Internal,
}

impl SdkErrorCategory {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Configuration => "configuration",
            Self::UnsupportedRuntime => "unsupported_runtime",
            Self::ModelPrepare => "model_prepare",
            Self::ModelManifest => "model_manifest",
            Self::AudioDecode => "audio_decode",
            Self::AudioResample => "audio_resample",
            Self::Pipeline => "pipeline",
            Self::Queue => "queue",
            Self::Cancelled => "cancelled",
            Self::Internal => "internal",
        }
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[error("{code}: {message}")]
pub struct SdkError {
    pub category: SdkErrorCategory,
    pub code: String,
    pub message: String,
}

impl SdkError {
    pub fn new(
        category: SdkErrorCategory,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            category,
            code: code.into(),
            message: message.into(),
        }
    }

    pub fn cancelled() -> Self {
        Self::new(
            SdkErrorCategory::Cancelled,
            SdkErrorCategory::Cancelled.code(),
            "operation was cancelled",
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_categories_have_stable_codes() {
        let pairs = [
            (SdkErrorCategory::Configuration, "configuration"),
            (SdkErrorCategory::UnsupportedRuntime, "unsupported_runtime"),
            (SdkErrorCategory::ModelPrepare, "model_prepare"),
            (SdkErrorCategory::ModelManifest, "model_manifest"),
            (SdkErrorCategory::AudioDecode, "audio_decode"),
            (SdkErrorCategory::AudioResample, "audio_resample"),
            (SdkErrorCategory::Pipeline, "pipeline"),
            (SdkErrorCategory::Queue, "queue"),
            (SdkErrorCategory::Cancelled, "cancelled"),
            (SdkErrorCategory::Internal, "internal"),
        ];

        for (category, code) in pairs {
            assert_eq!(category.code(), code);
        }
    }

    #[test]
    fn cancelled_error_uses_cancelled_category() {
        let err = SdkError::cancelled();

        assert_eq!(err.category, SdkErrorCategory::Cancelled);
        assert_eq!(err.code, "cancelled");
        assert!(err.to_string().contains("operation was cancelled"));
    }
}
