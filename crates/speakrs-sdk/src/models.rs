use std::io::Read;
use std::path::{Component, Path, PathBuf};

use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::dto::ExecutionModeDto;

pub const DEFAULT_MODEL_REPOSITORY: &str = "avencera/speakrs-models";
pub const DEFAULT_MODEL_REVISION: &str = "5d24ffee75f13fb061fa6d10944a64e2dc1d5e6f";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelManifestPlan {
    pub repository: &'static str,
    pub revision: &'static str,
    pub manifest_file: &'static str,
}

impl Default for ModelManifestPlan {
    fn default() -> Self {
        Self {
            repository: DEFAULT_MODEL_REPOSITORY,
            revision: DEFAULT_MODEL_REVISION,
            manifest_file: "model-manifest.json",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrepareModelsOptions {
    pub mode: ExecutionModeDto,
    pub cache_dir: Option<PathBuf>,
    pub model_dir: Option<PathBuf>,
    pub manifest: Option<ModelManifest>,
}

impl PrepareModelsOptions {
    pub fn new(mode: ExecutionModeDto) -> Self {
        Self {
            mode,
            cache_dir: None,
            model_dir: None,
            manifest: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedModels {
    pub model_dir: PathBuf,
    pub manifest: ModelManifest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheEntry {
    pub repository: String,
    pub revision: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct ModelStore {
    cache_dir: PathBuf,
    plan: ModelManifestPlan,
}

impl ModelStore {
    pub fn new(cache_dir: impl Into<PathBuf>) -> Self {
        Self {
            cache_dir: cache_dir.into(),
            plan: ModelManifestPlan::default(),
        }
    }

    pub fn with_plan(cache_dir: impl Into<PathBuf>, plan: ModelManifestPlan) -> Self {
        Self {
            cache_dir: cache_dir.into(),
            plan,
        }
    }

    pub fn default_cache_dir() -> PathBuf {
        if let Some(path) = std::env::var_os("SPEAKRS_CACHE_DIR") {
            return PathBuf::from(path);
        }

        if let Some(path) = std::env::var_os("XDG_CACHE_HOME") {
            return PathBuf::from(path).join("speakrs");
        }

        if let Some(path) = std::env::var_os("HOME") {
            return PathBuf::from(path).join(".cache").join("speakrs");
        }

        std::env::temp_dir().join("speakrs-cache")
    }

    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    pub fn snapshot_dir(&self) -> PathBuf {
        self.snapshot_dir_for_cache(&self.cache_dir)
    }

    fn snapshot_dir_for_cache(&self, cache_dir: &Path) -> PathBuf {
        cache_dir
            .join(cache_repo_dir(self.plan.repository))
            .join(self.plan.revision)
    }

    pub fn prepare(
        &self,
        options: PrepareModelsOptions,
    ) -> Result<PreparedModels, ModelPrepareError> {
        let mode = options.mode;
        let cache_dir = options.cache_dir.as_deref().unwrap_or(&self.cache_dir);
        let uses_sdk_snapshot = options.model_dir.is_none();
        let model_dir = if let Some(model_dir) = options.model_dir {
            model_dir
        } else {
            let model_dir = self.snapshot_dir_for_cache(cache_dir);
            #[cfg(feature = "online")]
            {
                self.download_required_files(cache_dir, &model_dir, mode)?;
            }
            #[cfg(not(feature = "online"))]
            if !model_dir.exists() {
                return Err(ModelPrepareError::MissingPreparedModelDir { path: model_dir });
            }
            model_dir
        };
        let manifest = match options.manifest {
            Some(manifest) => manifest,
            None if uses_sdk_snapshot => self.generate_manifest(&model_dir, mode)?,
            None => return Err(ModelPrepareError::MissingManifest),
        };
        manifest.verify(&model_dir)?;

        Ok(PreparedModels {
            model_dir,
            manifest,
        })
    }

    pub fn generate_manifest(
        &self,
        model_dir: impl AsRef<Path>,
        mode: ExecutionModeDto,
    ) -> Result<ModelManifest, ModelManifestError> {
        ModelManifest::generate(model_dir, &self.plan, required_model_files(mode))
    }

    #[cfg(feature = "online")]
    fn download_required_files(
        &self,
        cache_dir: &Path,
        model_dir: &Path,
        mode: ExecutionModeDto,
    ) -> Result<(), ModelPrepareError> {
        let repo = hf_hub::Repo::with_revision(
            self.plan.repository.to_string(),
            hf_hub::RepoType::Model,
            self.plan.revision.to_string(),
        );
        let api =
            hf_hub::api::sync::ApiBuilder::from_cache(hf_hub::Cache::new(cache_dir.join("hf-hub")))
                .build()
                .map_err(|err| ModelPrepareError::Download {
                    message: err.to_string(),
                })?;
        let api_repo = api.repo(repo);

        for file in required_model_files(mode) {
            let source = api_repo
                .get(&file)
                .map_err(|err| ModelPrepareError::Download {
                    message: err.to_string(),
                })?;
            let destination = model_dir.join(&file);
            if let Some(parent) = destination.parent() {
                std::fs::create_dir_all(parent).map_err(|err| ModelPrepareError::CopyFile {
                    path: destination.clone(),
                    message: err.to_string(),
                })?;
            }
            std::fs::copy(&source, &destination).map_err(|err| ModelPrepareError::CopyFile {
                path: destination,
                message: err.to_string(),
            })?;
        }

        Ok(())
    }

    pub fn list_cache(&self) -> Result<Vec<CacheEntry>, ModelPrepareError> {
        let repo_dir = self.cache_dir.join(cache_repo_dir(self.plan.repository));
        let Ok(entries) = std::fs::read_dir(&repo_dir) else {
            return Ok(Vec::new());
        };

        let mut cache_entries = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|err| ModelPrepareError::CacheList {
                path: repo_dir.clone(),
                message: err.to_string(),
            })?;
            let file_type = entry
                .file_type()
                .map_err(|err| ModelPrepareError::CacheList {
                    path: entry.path(),
                    message: err.to_string(),
                })?;
            if !file_type.is_dir() {
                continue;
            }

            cache_entries.push(CacheEntry {
                repository: self.plan.repository.to_string(),
                revision: entry.file_name().to_string_lossy().into_owned(),
                path: entry.path(),
            });
        }

        cache_entries.sort_by(|lhs, rhs| lhs.revision.cmp(&rhs.revision));
        Ok(cache_entries)
    }

    pub fn cleanup_revision(&self, revision: &str) -> Result<bool, ModelPrepareError> {
        if !is_safe_cache_revision(revision) {
            return Err(ModelPrepareError::CacheCleanup {
                path: PathBuf::from(revision),
                message: "invalid revision: contains path separators or traversal components"
                    .to_string(),
            });
        }

        let path = self
            .cache_dir
            .join(cache_repo_dir(self.plan.repository))
            .join(revision);
        match std::fs::remove_dir_all(&path) {
            Ok(()) => Ok(true),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(err) => Err(ModelPrepareError::CacheCleanup {
                path,
                message: err.to_string(),
            }),
        }
    }
}

impl Default for ModelStore {
    fn default() -> Self {
        Self::new(Self::default_cache_dir())
    }
}

#[derive(Debug, Error)]
pub enum ModelPrepareError {
    #[error("model prepare requires a checksum manifest")]
    MissingManifest,

    #[error("prepared model directory `{path}` does not exist")]
    MissingPreparedModelDir { path: PathBuf },

    #[error(transparent)]
    Manifest(#[from] ModelManifestError),

    #[error("failed to download prepared models: {message}")]
    Download { message: String },

    #[error("failed to copy downloaded model file `{path}`: {message}")]
    CopyFile { path: PathBuf, message: String },

    #[error("failed to list model cache `{path}`: {message}")]
    CacheList { path: PathBuf, message: String },

    #[error("failed to clean model cache `{path}`: {message}")]
    CacheCleanup { path: PathBuf, message: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelManifest {
    pub repository: String,
    pub revision: String,
    pub files: Vec<ModelManifestEntry>,
}

impl ModelManifest {
    pub fn generate(
        model_dir: impl AsRef<Path>,
        plan: &ModelManifestPlan,
        files: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<Self, ModelManifestError> {
        let model_dir = model_dir.as_ref();
        let mut entries = Vec::new();
        for file in files {
            let path = file.into();
            let absolute_path = model_dir.join(&path);
            entries.push(ModelManifestEntry::from_file(path, &absolute_path)?);
        }

        entries.sort_by(|lhs, rhs| lhs.path.cmp(&rhs.path));
        Ok(Self {
            repository: plan.repository.to_string(),
            revision: plan.revision.to_string(),
            files: entries,
        })
    }

    pub fn verify(&self, model_dir: impl AsRef<Path>) -> Result<(), ModelManifestError> {
        let model_dir = model_dir.as_ref();
        for expected in &self.files {
            let actual = ModelManifestEntry::from_file(
                expected.path.clone(),
                &model_dir.join(&expected.path),
            )?;
            if actual.size_bytes != expected.size_bytes {
                return Err(ModelManifestError::SizeMismatch {
                    path: expected.path.clone(),
                    expected: expected.size_bytes,
                    actual: actual.size_bytes,
                });
            }
            if actual.sha256 != expected.sha256 {
                return Err(ModelManifestError::ChecksumMismatch {
                    path: expected.path.clone(),
                    expected: expected.sha256.clone(),
                    actual: actual.sha256,
                });
            }
        }

        Ok(())
    }
}

pub fn required_model_files(mode: ExecutionModeDto) -> Vec<String> {
    let mut files: Vec<String> = PLDA_FILES.iter().map(|file| (*file).to_string()).collect();

    match mode {
        ExecutionModeDto::Cpu => {
            files.extend(ONNX_FILES.iter().map(|file| (*file).to_string()));
        }
        ExecutionModeDto::Cuda | ExecutionModeDto::CudaFast | ExecutionModeDto::MiGraphX => {
            files.extend(ONNX_FILES.iter().map(|file| (*file).to_string()));
            files.extend(
                ACCELERATED_ONNX_FILES
                    .iter()
                    .map(|file| (*file).to_string()),
            );
        }
        ExecutionModeDto::CoreMl => {
            files.extend(COREML_ONNX_FILES.iter().map(|file| (*file).to_string()));
            extend_mlmodelc_files(&mut files, COREML_COMMON_MODEL_STEMS);
            extend_mlmodelc_files(&mut files, COREML_CHUNK_MODEL_STEMS);
        }
        ExecutionModeDto::CoreMlFast => {
            files.extend(COREML_ONNX_FILES.iter().map(|file| (*file).to_string()));
            extend_mlmodelc_files(&mut files, COREML_COMMON_MODEL_STEMS);
            extend_mlmodelc_files(&mut files, COREML_FAST_SEGMENTATION_MODEL_STEMS);
            extend_mlmodelc_files(&mut files, COREML_FAST_CHUNK_MODEL_STEMS);
        }
    }

    files
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelManifestEntry {
    pub path: String,
    pub size_bytes: u64,
    pub sha256: String,
}

fn cache_repo_dir(repository: &str) -> String {
    repository.replace('/', "--")
}

fn is_safe_cache_revision(revision: &str) -> bool {
    if revision.contains('\\') {
        return false;
    }

    let mut components = Path::new(revision).components();
    matches!(components.next(), Some(Component::Normal(_))) && components.next().is_none()
}

fn mlmodelc_files(name: &str) -> Vec<String> {
    vec![
        format!("{name}/model.mil"),
        format!("{name}/coremldata.bin"),
        format!("{name}/weights/weight.bin"),
        format!("{name}/analytics/coremldata.bin"),
    ]
}

fn extend_mlmodelc_files(files: &mut Vec<String>, names: &[&str]) {
    for name in names {
        files.extend(mlmodelc_files(name));
    }
}

impl ModelManifestEntry {
    fn from_file(path: String, absolute_path: &Path) -> Result<Self, ModelManifestError> {
        let mut file = std::fs::File::open(absolute_path).map_err(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
                ModelManifestError::MissingFile { path: path.clone() }
            } else {
                ModelManifestError::ReadFile {
                    path: path.clone(),
                    message: err.to_string(),
                }
            }
        })?;
        let metadata = file
            .metadata()
            .map_err(|err| ModelManifestError::ReadFile {
                path: path.clone(),
                message: err.to_string(),
            })?;
        let mut hasher = Sha256::new();
        let mut buffer = [0; 16 * 1024];
        loop {
            let read = file
                .read(&mut buffer)
                .map_err(|err| ModelManifestError::ReadFile {
                    path: path.clone(),
                    message: err.to_string(),
                })?;
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
        }

        Ok(Self {
            path,
            size_bytes: metadata.len(),
            sha256: hex_lower(&hasher.finalize()),
        })
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ModelManifestError {
    #[error("model manifest is missing `{path}`")]
    MissingFile { path: String },

    #[error("failed to read model manifest file `{path}`: {message}")]
    ReadFile { path: String, message: String },

    #[error("model manifest size mismatch for `{path}`: expected {expected}, got {actual}")]
    SizeMismatch {
        path: String,
        expected: u64,
        actual: u64,
    },

    #[error("model manifest checksum mismatch for `{path}`: expected {expected}, got {actual}")]
    ChecksumMismatch {
        path: String,
        expected: String,
        actual: String,
    },
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

const PLDA_FILES: &[&str] = &[
    "plda_lda.npy",
    "plda_tr.npy",
    "plda_mu.npy",
    "plda_psi.npy",
    "plda_mean1.npy",
    "plda_mean2.npy",
    "wespeaker-voxceleb-resnet34.min_num_samples.txt",
];

const ONNX_FILES: &[&str] = &[
    "segmentation-3.0.onnx",
    "wespeaker-voxceleb-resnet34.onnx",
    "wespeaker-voxceleb-resnet34.onnx.data",
];

const ACCELERATED_ONNX_FILES: &[&str] = &[
    "wespeaker-fbank.onnx",
    "wespeaker-fbank-b32.onnx",
    "wespeaker-multimask-tail.onnx",
    "wespeaker-multimask-tail-b32.onnx",
    "segmentation-3.0-b32.onnx",
    "wespeaker-voxceleb-resnet34-b64.onnx",
];

const COREML_ONNX_FILES: &[&str] = &[
    "segmentation-3.0.onnx",
    "wespeaker-voxceleb-resnet34.onnx",
    "wespeaker-voxceleb-resnet34.onnx.data",
    "segmentation-3.0-b32.onnx",
    "wespeaker-fbank.onnx",
    "wespeaker-fbank-b32.onnx",
    "wespeaker-voxceleb-resnet34-tail.onnx",
    "wespeaker-voxceleb-resnet34-tail-b3.onnx",
    "wespeaker-voxceleb-resnet34-tail-b32.onnx",
];

const COREML_COMMON_MODEL_STEMS: &[&str] = &[
    "segmentation-3.0.mlmodelc",
    "segmentation-3.0-b32.mlmodelc",
    "segmentation-3.0-b64.mlmodelc",
    "wespeaker-fbank.mlmodelc",
    "wespeaker-fbank-b32.mlmodelc",
    "wespeaker-fbank-30s.mlmodelc",
    "wespeaker-multimask-tail-b32.mlmodelc",
    "wespeaker-voxceleb-resnet34-tail.mlmodelc",
    "wespeaker-voxceleb-resnet34-tail-b3.mlmodelc",
    "wespeaker-voxceleb-resnet34-tail-b32.mlmodelc",
];

const COREML_CHUNK_MODEL_STEMS: &[&str] = &[
    "wespeaker-chunk-emb-s12-w22.mlmodelc",
    "wespeaker-chunk-emb-s12-w37.mlmodelc",
    "wespeaker-chunk-emb-s12-w53.mlmodelc",
    "wespeaker-chunk-emb-s12-w84.mlmodelc",
    "wespeaker-chunk-emb-s12-w116.mlmodelc",
];

const COREML_FAST_SEGMENTATION_MODEL_STEMS: &[&str] = &[
    "segmentation-3.0-w8a16.mlmodelc",
    "segmentation-3.0-b32-w8a16.mlmodelc",
    "segmentation-3.0-b64-w8a16.mlmodelc",
];

const COREML_FAST_CHUNK_MODEL_STEMS: &[&str] = &[
    "wespeaker-chunk-emb-s25-w11.mlmodelc",
    "wespeaker-chunk-emb-s25-w16.mlmodelc",
    "wespeaker-chunk-emb-s25-w21.mlmodelc",
    "wespeaker-chunk-emb-s25-w26.mlmodelc",
    "wespeaker-chunk-emb-s25-w36.mlmodelc",
    "wespeaker-chunk-emb-s25-w46.mlmodelc",
    "wespeaker-chunk-emb-s25-w56.mlmodelc",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_manifest_plan_pins_current_fixture_revision() {
        let plan = ModelManifestPlan::default();

        assert_eq!(plan.repository, "avencera/speakrs-models");
        assert_eq!(plan.revision.len(), 40);
        assert_eq!(plan.manifest_file, "model-manifest.json");
    }

    #[test]
    fn generated_manifest_verifies_file_sizes_and_checksums() {
        let dir = temp_model_dir("verify-ok");
        write_model_file(&dir, "a.bin", b"alpha");
        write_model_file(&dir, "nested/b.bin", b"beta");

        let manifest = ModelManifest::generate(
            &dir,
            &ModelManifestPlan::default(),
            ["nested/b.bin", "a.bin"],
        )
        .unwrap();

        assert_eq!(manifest.repository, DEFAULT_MODEL_REPOSITORY);
        assert_eq!(manifest.revision, DEFAULT_MODEL_REVISION);
        assert_eq!(manifest.files[0].path, "a.bin");
        assert_eq!(manifest.files[0].size_bytes, 5);
        assert_eq!(manifest.files[0].sha256.len(), 64);
        manifest.verify(&dir).unwrap();

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn manifest_verification_rejects_corrupt_file() {
        let dir = temp_model_dir("verify-corrupt");
        write_model_file(&dir, "model.onnx", b"original");
        let manifest =
            ModelManifest::generate(&dir, &ModelManifestPlan::default(), ["model.onnx"]).unwrap();

        write_model_file(&dir, "model.onnx", b"corrupt");
        let err = manifest.verify(&dir).unwrap_err();

        assert!(matches!(err, ModelManifestError::SizeMismatch { .. }));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn manifest_verification_rejects_missing_file() {
        let dir = temp_model_dir("verify-missing");
        write_model_file(&dir, "model.onnx", b"original");
        let manifest =
            ModelManifest::generate(&dir, &ModelManifestPlan::default(), ["model.onnx"]).unwrap();

        std::fs::remove_file(dir.join("model.onnx")).unwrap();
        let err = manifest.verify(&dir).unwrap_err();

        assert!(matches!(err, ModelManifestError::MissingFile { .. }));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn required_model_files_cover_all_public_execution_modes() {
        let cpu = required_model_files(ExecutionModeDto::Cpu);
        assert!(cpu.contains(&"segmentation-3.0.onnx".to_string()));
        assert!(cpu.contains(&"plda_lda.npy".to_string()));

        let cuda = required_model_files(ExecutionModeDto::Cuda);
        assert!(cuda.contains(&"wespeaker-multimask-tail-b32.onnx".to_string()));
        assert!(cuda.contains(&"wespeaker-voxceleb-resnet34-b64.onnx".to_string()));

        let migraphx = required_model_files(ExecutionModeDto::MiGraphX);
        assert_eq!(cuda, migraphx);

        let coreml = required_model_files(ExecutionModeDto::CoreMl);
        assert!(coreml.contains(&"segmentation-3.0-b64.mlmodelc/model.mil".to_string()));
        assert!(coreml.contains(&"wespeaker-chunk-emb-s12-w116.mlmodelc/model.mil".to_string()));

        let coreml_fast = required_model_files(ExecutionModeDto::CoreMlFast);
        assert!(coreml_fast.contains(&"segmentation-3.0-b64-w8a16.mlmodelc/model.mil".to_string()));
        assert!(
            coreml_fast.contains(&"wespeaker-chunk-emb-s25-w56.mlmodelc/model.mil".to_string())
        );
    }

    #[test]
    fn model_store_prepares_manifest_verified_model_dir() {
        let dir = temp_model_dir("prepare-ok");
        write_model_file(&dir, "model.onnx", b"model");
        let store = ModelStore::new(temp_model_dir("prepare-cache"));
        let manifest =
            ModelManifest::generate(&dir, &ModelManifestPlan::default(), ["model.onnx"]).unwrap();

        let prepared = store
            .prepare(PrepareModelsOptions {
                mode: ExecutionModeDto::Cpu,
                cache_dir: None,
                model_dir: Some(dir.clone()),
                manifest: Some(manifest.clone()),
            })
            .unwrap();

        assert_eq!(prepared.model_dir, dir);
        assert_eq!(prepared.manifest, manifest);
        let _ = std::fs::remove_dir_all(store.cache_dir());
        let _ = std::fs::remove_dir_all(prepared.model_dir);
    }

    #[test]
    fn model_store_requires_manifest_for_prepare() {
        let dir = temp_model_dir("prepare-missing-manifest-models");
        let store = ModelStore::new(temp_model_dir("prepare-missing-manifest-cache"));
        let err = store
            .prepare(PrepareModelsOptions {
                mode: ExecutionModeDto::Cpu,
                cache_dir: None,
                model_dir: Some(dir.clone()),
                manifest: None,
            })
            .unwrap_err();

        assert!(matches!(err, ModelPrepareError::MissingManifest));
        let _ = std::fs::remove_dir_all(dir);
        let _ = std::fs::remove_dir_all(store.cache_dir());
    }

    #[test]
    fn model_store_uses_cache_override_for_prepared_snapshot() {
        let cache_dir = temp_model_dir("prepare-cache-override");
        let store = ModelStore::new(temp_model_dir("prepare-cache-unused"));
        let model_dir = store.snapshot_dir_for_cache(&cache_dir);
        write_model_file(&model_dir, "model.onnx", b"model");
        let manifest =
            ModelManifest::generate(&model_dir, &ModelManifestPlan::default(), ["model.onnx"])
                .unwrap();

        let prepared = store
            .prepare(PrepareModelsOptions {
                mode: ExecutionModeDto::Cpu,
                cache_dir: Some(cache_dir.clone()),
                model_dir: None,
                manifest: Some(manifest),
            })
            .unwrap();

        assert_eq!(prepared.model_dir, model_dir);
        let _ = std::fs::remove_dir_all(cache_dir);
        let _ = std::fs::remove_dir_all(store.cache_dir());
    }

    #[test]
    fn model_store_generates_manifest_for_sdk_owned_snapshot() {
        let cache_dir = temp_model_dir("prepare-generated-manifest-cache");
        let store = ModelStore::new(&cache_dir);
        let model_dir = store.snapshot_dir();
        for file in required_model_files(ExecutionModeDto::Cpu) {
            write_model_file(&model_dir, &file, file.as_bytes());
        }

        let prepared = store
            .prepare(PrepareModelsOptions {
                mode: ExecutionModeDto::Cpu,
                cache_dir: None,
                model_dir: None,
                manifest: None,
            })
            .unwrap();

        assert_eq!(prepared.model_dir, model_dir);
        assert_eq!(prepared.manifest.revision, DEFAULT_MODEL_REVISION);
        assert_eq!(
            prepared.manifest.files.len(),
            required_model_files(ExecutionModeDto::Cpu).len()
        );
        prepared.manifest.verify(&prepared.model_dir).unwrap();

        let _ = std::fs::remove_dir_all(cache_dir);
    }

    #[test]
    #[cfg(not(feature = "online"))]
    fn model_store_without_online_requires_prepopulated_cache() {
        let store = ModelStore::new(temp_model_dir("prepare-empty-cache"));
        let manifest = ModelManifest {
            repository: DEFAULT_MODEL_REPOSITORY.to_string(),
            revision: DEFAULT_MODEL_REVISION.to_string(),
            files: vec![],
        };

        let err = store
            .prepare(PrepareModelsOptions {
                mode: ExecutionModeDto::Cpu,
                cache_dir: None,
                model_dir: None,
                manifest: Some(manifest),
            })
            .unwrap_err();

        assert!(matches!(
            err,
            ModelPrepareError::MissingPreparedModelDir { .. }
        ));
        let _ = std::fs::remove_dir_all(store.cache_dir());
    }

    #[test]
    fn model_store_lists_and_cleans_cache_revisions() {
        let cache_dir = temp_model_dir("cache-list");
        let store = ModelStore::new(&cache_dir);
        std::fs::create_dir_all(store.snapshot_dir()).unwrap();
        std::fs::create_dir_all(
            cache_dir
                .join(cache_repo_dir(DEFAULT_MODEL_REPOSITORY))
                .join("other-revision"),
        )
        .unwrap();

        let entries = store.list_cache().unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].revision, DEFAULT_MODEL_REVISION);
        assert_eq!(entries[1].revision, "other-revision");
        assert!(store.cleanup_revision("other-revision").unwrap());
        assert!(!store.cleanup_revision("missing").unwrap());

        let remaining = store.list_cache().unwrap();
        assert_eq!(remaining.len(), 1);
        let _ = std::fs::remove_dir_all(cache_dir);
    }

    #[test]
    fn model_store_rejects_cleanup_revision_traversal() {
        let cache_dir = temp_model_dir("cleanup-traversal");
        let store = ModelStore::new(&cache_dir);
        let protected_dir = cache_dir.join("protected");
        std::fs::create_dir_all(&protected_dir).unwrap();

        let err = store.cleanup_revision("../protected").unwrap_err();

        assert!(matches!(err, ModelPrepareError::CacheCleanup { .. }));
        assert!(protected_dir.is_dir());

        let err = store.cleanup_revision(r"..\protected").unwrap_err();
        assert!(matches!(err, ModelPrepareError::CacheCleanup { .. }));
        assert!(protected_dir.is_dir());
        let _ = std::fs::remove_dir_all(cache_dir);
    }

    fn temp_model_dir(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("speakrs-sdk-models-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_model_file(dir: &Path, relative_path: &str, bytes: &[u8]) {
        let path = dir.join(relative_path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, bytes).unwrap();
    }
}
