use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

use pyo3::exceptions::{PyException, PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use speakrs_sdk::{
    AhcConfigDto, BinarizeConfigDto, CacheEntry, CancelToken, DiarizationResultDto,
    ExecutionModeDto, ModelManifest, ModelManifestEntry, ModelPrepareError, ModelStore,
    PipelineConfigDto, PrepareModelsOptions, PreparedModels, ReconstructMethodDto,
    RuntimeConfigDto, SdkError, SdkErrorCategory, SdkPipeline, SdkQueue, SegmentDto,
    TimingStatsDto, VbxConfigDto, required_model_files,
};

pyo3::create_exception!(_native, SpeakrsError, PyException);

#[pyclass(name = "_CancelToken", from_py_object)]
#[derive(Clone)]
struct PyCancelToken {
    inner: CancelToken,
}

#[pymethods]
impl PyCancelToken {
    #[new]
    fn new() -> Self {
        Self {
            inner: CancelToken::new(),
        }
    }

    fn cancel(&self) {
        self.inner.cancel();
    }

    #[getter]
    fn cancelled(&self) -> bool {
        self.inner.is_cancelled()
    }
}

#[pyclass(name = "_BinarizeConfig", frozen, from_py_object)]
#[derive(Clone)]
struct PyBinarizeConfig {
    inner: BinarizeConfigDto,
}

#[pymethods]
impl PyBinarizeConfig {
    #[new]
    #[pyo3(signature = (onset=None, offset=None, min_duration_on=None, min_duration_off=None, pad_onset=None, pad_offset=None))]
    fn new(
        onset: Option<f32>,
        offset: Option<f32>,
        min_duration_on: Option<usize>,
        min_duration_off: Option<usize>,
        pad_onset: Option<usize>,
        pad_offset: Option<usize>,
    ) -> Self {
        let defaults = BinarizeConfigDto::default();
        Self {
            inner: BinarizeConfigDto {
                onset: onset.unwrap_or(defaults.onset),
                offset: offset.unwrap_or(defaults.offset),
                min_duration_on: min_duration_on.unwrap_or(defaults.min_duration_on),
                min_duration_off: min_duration_off.unwrap_or(defaults.min_duration_off),
                pad_onset: pad_onset.unwrap_or(defaults.pad_onset),
                pad_offset: pad_offset.unwrap_or(defaults.pad_offset),
            },
        }
    }

    #[getter]
    fn onset(&self) -> f32 {
        self.inner.onset
    }

    #[getter]
    fn offset(&self) -> f32 {
        self.inner.offset
    }

    #[getter]
    fn min_duration_on(&self) -> usize {
        self.inner.min_duration_on
    }

    #[getter]
    fn min_duration_off(&self) -> usize {
        self.inner.min_duration_off
    }

    #[getter]
    fn pad_onset(&self) -> usize {
        self.inner.pad_onset
    }

    #[getter]
    fn pad_offset(&self) -> usize {
        self.inner.pad_offset
    }
}

impl From<PyBinarizeConfig> for BinarizeConfigDto {
    fn from(value: PyBinarizeConfig) -> Self {
        value.inner
    }
}

#[pyclass(name = "_AhcConfig", frozen, from_py_object)]
#[derive(Clone)]
struct PyAhcConfig {
    inner: AhcConfigDto,
}

#[pymethods]
impl PyAhcConfig {
    #[new]
    #[pyo3(signature = (threshold=None))]
    fn new(threshold: Option<f32>) -> Self {
        let defaults = AhcConfigDto::default();
        Self {
            inner: AhcConfigDto {
                threshold: threshold.unwrap_or(defaults.threshold),
            },
        }
    }

    #[getter]
    fn threshold(&self) -> f32 {
        self.inner.threshold
    }
}

impl From<PyAhcConfig> for AhcConfigDto {
    fn from(value: PyAhcConfig) -> Self {
        value.inner
    }
}

#[pyclass(name = "_VbxConfig", frozen, from_py_object)]
#[derive(Clone)]
struct PyVbxConfig {
    inner: VbxConfigDto,
}

#[pymethods]
impl PyVbxConfig {
    #[new]
    #[pyo3(signature = (fa=None, fb=None, max_iters=None, epsilon=None, init_smoothing=None))]
    fn new(
        fa: Option<f64>,
        fb: Option<f64>,
        max_iters: Option<usize>,
        epsilon: Option<f64>,
        init_smoothing: Option<f64>,
    ) -> Self {
        let defaults = VbxConfigDto::default();
        Self {
            inner: VbxConfigDto {
                fa: fa.unwrap_or(defaults.fa),
                fb: fb.unwrap_or(defaults.fb),
                max_iters: max_iters.unwrap_or(defaults.max_iters),
                epsilon: epsilon.unwrap_or(defaults.epsilon),
                init_smoothing: init_smoothing.unwrap_or(defaults.init_smoothing),
            },
        }
    }

    #[getter]
    fn fa(&self) -> f64 {
        self.inner.fa
    }

    #[getter]
    fn fb(&self) -> f64 {
        self.inner.fb
    }

    #[getter]
    fn max_iters(&self) -> usize {
        self.inner.max_iters
    }

    #[getter]
    fn epsilon(&self) -> f64 {
        self.inner.epsilon
    }

    #[getter]
    fn init_smoothing(&self) -> f64 {
        self.inner.init_smoothing
    }
}

impl From<PyVbxConfig> for VbxConfigDto {
    fn from(value: PyVbxConfig) -> Self {
        value.inner
    }
}

#[pyclass(name = "_PipelineConfig", frozen, from_py_object)]
#[derive(Clone)]
struct PyPipelineConfig {
    inner: PipelineConfigDto,
}

#[pymethods]
impl PyPipelineConfig {
    #[new]
    #[pyo3(signature = (binarize=None, ahc=None, vbx=None, merge_gap=None, speaker_keep_threshold=None, reconstruct_method=None, reconstruct_epsilon=None))]
    fn new(
        binarize: Option<PyBinarizeConfig>,
        ahc: Option<PyAhcConfig>,
        vbx: Option<PyVbxConfig>,
        merge_gap: Option<f64>,
        speaker_keep_threshold: Option<f64>,
        reconstruct_method: Option<&str>,
        reconstruct_epsilon: Option<f32>,
    ) -> PyResult<Self> {
        let defaults = PipelineConfigDto::default();
        Ok(Self {
            inner: PipelineConfigDto {
                binarize: binarize.map(Into::into).unwrap_or(defaults.binarize),
                ahc: ahc.map(Into::into).unwrap_or(defaults.ahc),
                vbx: vbx.map(Into::into).unwrap_or(defaults.vbx),
                merge_gap: merge_gap.unwrap_or(defaults.merge_gap),
                speaker_keep_threshold: speaker_keep_threshold
                    .unwrap_or(defaults.speaker_keep_threshold),
                reconstruct_method: parse_reconstruct_method(
                    reconstruct_method,
                    reconstruct_epsilon,
                )?,
            },
        })
    }

    #[staticmethod]
    fn default() -> Self {
        Self {
            inner: PipelineConfigDto::default(),
        }
    }

    #[staticmethod]
    fn for_mode(mode: &str) -> PyResult<Self> {
        Ok(Self {
            inner: PipelineConfigDto::for_mode(parse_mode(mode)?),
        })
    }

    #[getter]
    fn binarize(&self) -> PyBinarizeConfig {
        PyBinarizeConfig {
            inner: self.inner.binarize.clone(),
        }
    }

    #[getter]
    fn ahc(&self) -> PyAhcConfig {
        PyAhcConfig {
            inner: self.inner.ahc,
        }
    }

    #[getter]
    fn vbx(&self) -> PyVbxConfig {
        PyVbxConfig {
            inner: self.inner.vbx,
        }
    }

    #[getter]
    fn merge_gap(&self) -> f64 {
        self.inner.merge_gap
    }

    #[getter]
    fn speaker_keep_threshold(&self) -> f64 {
        self.inner.speaker_keep_threshold
    }

    #[getter]
    fn reconstruct_method(&self) -> String {
        match self.inner.reconstruct_method {
            ReconstructMethodDto::Standard => "standard".to_string(),
            ReconstructMethodDto::Smoothed { .. } => "smoothed".to_string(),
        }
    }

    #[getter]
    fn reconstruct_epsilon(&self) -> Option<f32> {
        match self.inner.reconstruct_method {
            ReconstructMethodDto::Standard => None,
            ReconstructMethodDto::Smoothed { epsilon } => Some(epsilon),
        }
    }
}

#[pyclass(name = "_RuntimeConfig", frozen, from_py_object)]
#[derive(Clone)]
struct PyRuntimeConfig {
    inner: RuntimeConfigDto,
}

#[pymethods]
impl PyRuntimeConfig {
    #[new]
    #[pyo3(signature = (chunk_emb_workers=None))]
    fn new(chunk_emb_workers: Option<usize>) -> Self {
        let defaults = RuntimeConfigDto::default();
        Self {
            inner: RuntimeConfigDto {
                chunk_emb_workers: chunk_emb_workers.unwrap_or(defaults.chunk_emb_workers),
            },
        }
    }

    #[getter]
    fn chunk_emb_workers(&self) -> usize {
        self.inner.chunk_emb_workers
    }
}

#[pyclass(name = "_ModelManifestEntry", frozen, from_py_object)]
#[derive(Clone)]
struct PyModelManifestEntry {
    inner: ModelManifestEntry,
}

#[pymethods]
impl PyModelManifestEntry {
    #[getter]
    fn path(&self) -> String {
        self.inner.path.clone()
    }

    #[getter]
    fn size_bytes(&self) -> u64 {
        self.inner.size_bytes
    }

    #[getter]
    fn sha256(&self) -> String {
        self.inner.sha256.clone()
    }
}

#[pyclass(name = "_ModelManifest", frozen, from_py_object)]
#[derive(Clone)]
struct PyModelManifest {
    inner: ModelManifest,
}

#[pymethods]
impl PyModelManifest {
    #[getter]
    fn repository(&self) -> String {
        self.inner.repository.clone()
    }

    #[getter]
    fn revision(&self) -> String {
        self.inner.revision.clone()
    }

    #[getter]
    fn files(&self) -> Vec<PyModelManifestEntry> {
        self.inner
            .files
            .iter()
            .cloned()
            .map(|inner| PyModelManifestEntry { inner })
            .collect()
    }
}

#[pyclass(name = "_PreparedModels", frozen, from_py_object)]
#[derive(Clone)]
struct PyPreparedModels {
    inner: PreparedModels,
}

#[pymethods]
impl PyPreparedModels {
    #[getter]
    fn model_dir(&self) -> PathBuf {
        self.inner.model_dir.clone()
    }

    #[getter]
    fn manifest(&self) -> PyModelManifest {
        PyModelManifest {
            inner: self.inner.manifest.clone(),
        }
    }
}

#[pyclass(name = "_CacheEntry", frozen, from_py_object)]
#[derive(Clone)]
struct PyCacheEntry {
    inner: CacheEntry,
}

#[pymethods]
impl PyCacheEntry {
    #[getter]
    fn repository(&self) -> String {
        self.inner.repository.clone()
    }

    #[getter]
    fn revision(&self) -> String {
        self.inner.revision.clone()
    }

    #[getter]
    fn path(&self) -> PathBuf {
        self.inner.path.clone()
    }
}

#[pyclass(name = "_Segment", frozen, from_py_object)]
#[derive(Clone)]
struct PySegment {
    inner: SegmentDto,
}

#[pymethods]
impl PySegment {
    #[getter]
    fn start(&self) -> f64 {
        self.inner.start
    }

    #[getter]
    fn end(&self) -> f64 {
        self.inner.end
    }

    #[getter]
    fn speaker(&self) -> String {
        self.inner.speaker.clone()
    }

    #[getter]
    fn duration(&self) -> f64 {
        self.inner.duration()
    }
}

#[pyclass(name = "_TimingStats", frozen, from_py_object)]
#[derive(Clone)]
struct PyTimingStats {
    inner: TimingStatsDto,
}

#[pymethods]
impl PyTimingStats {
    #[getter]
    fn model_prepare_ms(&self) -> u64 {
        self.inner.model_prepare_ms
    }

    #[getter]
    fn audio_decode_ms(&self) -> u64 {
        self.inner.audio_decode_ms
    }

    #[getter]
    fn audio_resample_ms(&self) -> u64 {
        self.inner.audio_resample_ms
    }

    #[getter]
    fn pipeline_ms(&self) -> u64 {
        self.inner.pipeline_ms
    }

    #[getter]
    fn queue_wait_ms(&self) -> u64 {
        self.inner.queue_wait_ms
    }

    #[getter]
    fn total_ms(&self) -> u64 {
        self.inner.total_ms
    }
}

#[pyclass(name = "_DiarizationResult", frozen, from_py_object)]
#[derive(Clone)]
struct PyDiarizationResult {
    inner: DiarizationResultDto,
}

#[pymethods]
impl PyDiarizationResult {
    #[getter]
    fn segments(&self) -> Vec<PySegment> {
        self.inner
            .segments
            .iter()
            .cloned()
            .map(|inner| PySegment { inner })
            .collect()
    }

    #[getter]
    fn rttm(&self) -> String {
        self.inner.rttm.clone()
    }

    #[getter]
    fn duration(&self) -> f64 {
        self.inner.duration
    }

    #[getter]
    fn mode(&self) -> String {
        self.inner.mode.wire_name().to_string()
    }

    #[getter]
    fn model_revision(&self) -> String {
        self.inner.model_revision.clone()
    }

    #[getter]
    fn timing(&self) -> PyTimingStats {
        PyTimingStats {
            inner: self.inner.timing.clone(),
        }
    }
}

#[pyclass(name = "_QueueResult", frozen, from_py_object)]
#[derive(Clone)]
struct PyQueueResult {
    job_id: u64,
    file_id: String,
    result: Option<PyDiarizationResult>,
    error: Option<PyErrorInfo>,
}

#[pymethods]
impl PyQueueResult {
    #[getter]
    fn job_id(&self) -> u64 {
        self.job_id
    }

    #[getter]
    fn file_id(&self) -> String {
        self.file_id.clone()
    }

    #[getter]
    fn ok(&self) -> bool {
        self.error.is_none()
    }

    #[getter]
    fn result(&self) -> Option<PyDiarizationResult> {
        self.result.clone()
    }

    #[getter]
    fn error(&self) -> Option<PyErrorInfo> {
        self.error.clone()
    }

    fn unwrap(&self) -> PyResult<PyDiarizationResult> {
        if let Some(result) = &self.result {
            return Ok(result.clone());
        }

        let error = self
            .error
            .as_ref()
            .ok_or_else(|| PyRuntimeError::new_err("queue result has no result or error"))?;
        Err(speakrs_error(
            &error.category,
            &error.code,
            error.message.clone(),
        ))
    }
}

#[pyclass(name = "_ErrorInfo", frozen, from_py_object)]
#[derive(Clone)]
struct PyErrorInfo {
    category: String,
    code: String,
    message: String,
}

#[pymethods]
impl PyErrorInfo {
    #[getter]
    fn category(&self) -> String {
        self.category.clone()
    }

    #[getter]
    fn code(&self) -> String {
        self.code.clone()
    }

    #[getter]
    fn message(&self) -> String {
        self.message.clone()
    }
}

#[pyclass(name = "_Pipeline")]
struct PyPipeline {
    inner: Mutex<Option<SdkPipeline>>,
}

#[pymethods]
impl PyPipeline {
    fn diarize_samples(
        &mut self,
        samples: Vec<f32>,
        file_id: Option<String>,
        pipeline_config: Option<PyPipelineConfig>,
        cancel_token: Option<PyCancelToken>,
    ) -> PyResult<PyDiarizationResult> {
        let options = speakrs_sdk::DiarizeSamplesOptions {
            file_id: file_id.as_deref().unwrap_or("file1"),
            pipeline_config: pipeline_config.map(|config| config.inner),
            cancel_token: cancel_token.as_ref().map(|token| &token.inner),
            progress: None,
        };
        let mut inner = self.inner_mut()?;
        let pipeline = inner
            .as_mut()
            .ok_or_else(|| PyRuntimeError::new_err("pipeline was converted to a queue"))?;
        pipeline
            .diarize_samples(&samples, options)
            .map(py_result)
            .map_err(sdk_py_err)
    }

    fn diarize_file(
        &mut self,
        path: PathBuf,
        file_id: Option<String>,
        pipeline_config: Option<PyPipelineConfig>,
        cancel_token: Option<PyCancelToken>,
    ) -> PyResult<PyDiarizationResult> {
        let options = speakrs_sdk::DiarizeFileOptions {
            file_id: file_id.as_deref().unwrap_or("file1"),
            pipeline_config: pipeline_config.map(|config| config.inner),
            cancel_token: cancel_token.as_ref().map(|token| &token.inner),
            progress: None,
        };
        let mut inner = self.inner_mut()?;
        let pipeline = inner
            .as_mut()
            .ok_or_else(|| PyRuntimeError::new_err("pipeline was converted to a queue"))?;
        pipeline
            .diarize_file(path, options)
            .map(py_result)
            .map_err(sdk_py_err)
    }

    #[pyo3(name = "into_queue")]
    fn make_queue(&mut self, pipeline_config: Option<PyPipelineConfig>) -> PyResult<PyQueue> {
        let pipeline = self
            .inner
            .lock()
            .map_err(|_| PyRuntimeError::new_err("pipeline lock was poisoned"))?
            .take()
            .ok_or_else(|| PyRuntimeError::new_err("pipeline was already converted to a queue"))?;
        pipeline
            .into_queue(pipeline_config.map(|config| config.inner))
            .map(|inner| PyQueue { inner })
            .map_err(sdk_py_err)
    }
}

impl PyPipeline {
    fn inner_mut(&self) -> PyResult<MutexGuard<'_, Option<SdkPipeline>>> {
        let guard = self
            .inner
            .lock()
            .map_err(|_| PyRuntimeError::new_err("pipeline lock was poisoned"))?;
        if guard.is_none() {
            return Err(PyRuntimeError::new_err("pipeline was converted to a queue"));
        }

        Ok(guard)
    }
}

#[pyclass(name = "_Queue")]
struct PyQueue {
    inner: SdkQueue,
}

#[pymethods]
impl PyQueue {
    fn push_samples(&self, py: Python<'_>, file_id: String, samples: Vec<f32>) -> PyResult<u64> {
        py.detach(|| self.inner.push_samples(file_id, samples))
            .map_err(sdk_py_err)
    }

    fn push_file(&self, py: Python<'_>, file_id: String, path: PathBuf) -> PyResult<u64> {
        py.detach(|| self.inner.push_file(file_id, path))
            .map_err(sdk_py_err)
    }

    fn recv(&self, py: Python<'_>) -> PyResult<PyQueueResult> {
        py.detach(|| self.inner.recv())
            .map(py_queue_result)
            .map_err(sdk_py_err)
    }

    fn try_recv(&self) -> PyResult<Option<PyQueueResult>> {
        self.inner
            .try_recv()
            .map(|result| result.map(py_queue_result))
            .map_err(sdk_py_err)
    }
}

#[pyfunction]
fn _default_model_revision() -> &'static str {
    speakrs_sdk::DEFAULT_MODEL_REVISION
}

#[pyfunction]
fn _default_cache_dir() -> PathBuf {
    ModelStore::default_cache_dir()
}

#[pyfunction]
fn _required_model_files(mode: &str) -> PyResult<Vec<String>> {
    Ok(required_model_files(parse_mode(mode)?))
}

#[pyfunction]
fn _generate_manifest(model_dir: PathBuf, mode: &str) -> PyResult<PyModelManifest> {
    let store = ModelStore::default();
    store
        .generate_manifest(model_dir, parse_mode(mode)?)
        .map(|inner| PyModelManifest { inner })
        .map_err(manifest_py_err)
}

#[pyfunction]
#[pyo3(signature = (mode, cache_dir=None, model_dir=None, manifest=None))]
fn _prepare(
    mode: &str,
    cache_dir: Option<PathBuf>,
    model_dir: Option<PathBuf>,
    manifest: Option<PyModelManifest>,
) -> PyResult<PyPreparedModels> {
    let store = cache_dir.clone().map(ModelStore::new).unwrap_or_default();
    let mut options = PrepareModelsOptions::new(parse_mode(mode)?);
    options.cache_dir = cache_dir;
    options.model_dir = model_dir;
    options.manifest = manifest.map(|manifest| manifest.inner);

    store
        .prepare(options)
        .map(|inner| PyPreparedModels { inner })
        .map_err(model_prepare_py_err)
}

#[pyfunction]
fn _list_cache(cache_dir: Option<PathBuf>) -> PyResult<Vec<PyCacheEntry>> {
    let store = cache_dir.map(ModelStore::new).unwrap_or_default();
    store
        .list_cache()
        .map(|entries| {
            entries
                .into_iter()
                .map(|inner| PyCacheEntry { inner })
                .collect()
        })
        .map_err(model_prepare_py_err)
}

#[pyfunction]
fn _cleanup_revision(revision: &str, cache_dir: Option<PathBuf>) -> PyResult<bool> {
    let store = cache_dir.map(ModelStore::new).unwrap_or_default();
    store
        .cleanup_revision(revision)
        .map_err(model_prepare_py_err)
}

#[pyfunction]
#[pyo3(signature = (prepared, mode, pipeline_config=None, runtime_config=None))]
fn _build_pipeline(
    prepared: PyPreparedModels,
    mode: &str,
    pipeline_config: Option<PyPipelineConfig>,
    runtime_config: Option<PyRuntimeConfig>,
) -> PyResult<PyPipeline> {
    SdkPipeline::from_prepared(
        prepared.inner,
        parse_mode(mode)?,
        pipeline_config.map(|config| config.inner),
        runtime_config.map(|config| config.inner),
    )
    .map(|inner| PyPipeline {
        inner: Mutex::new(Some(inner)),
    })
    .map_err(sdk_py_err)
}

#[pymodule]
fn _native(py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("SpeakrsError", py.get_type::<SpeakrsError>())?;
    m.add_class::<PyCancelToken>()?;
    m.add_class::<PyBinarizeConfig>()?;
    m.add_class::<PyAhcConfig>()?;
    m.add_class::<PyVbxConfig>()?;
    m.add_class::<PyPipelineConfig>()?;
    m.add_class::<PyRuntimeConfig>()?;
    m.add_class::<PyModelManifestEntry>()?;
    m.add_class::<PyModelManifest>()?;
    m.add_class::<PyPreparedModels>()?;
    m.add_class::<PyCacheEntry>()?;
    m.add_class::<PySegment>()?;
    m.add_class::<PyTimingStats>()?;
    m.add_class::<PyDiarizationResult>()?;
    m.add_class::<PyQueueResult>()?;
    m.add_class::<PyErrorInfo>()?;
    m.add_class::<PyPipeline>()?;
    m.add_class::<PyQueue>()?;
    m.add_function(wrap_pyfunction!(_default_model_revision, m)?)?;
    m.add_function(wrap_pyfunction!(_default_cache_dir, m)?)?;
    m.add_function(wrap_pyfunction!(_required_model_files, m)?)?;
    m.add_function(wrap_pyfunction!(_generate_manifest, m)?)?;
    m.add_function(wrap_pyfunction!(_prepare, m)?)?;
    m.add_function(wrap_pyfunction!(_list_cache, m)?)?;
    m.add_function(wrap_pyfunction!(_cleanup_revision, m)?)?;
    m.add_function(wrap_pyfunction!(_build_pipeline, m)?)?;
    Ok(())
}

fn parse_mode(mode: &str) -> PyResult<ExecutionModeDto> {
    match mode {
        "CPU" | "cpu" => Ok(ExecutionModeDto::Cpu),
        "CoreML" | "coreml" => Ok(ExecutionModeDto::CoreMl),
        "CoreMLFast" | "coreml_fast" | "coremlfast" => Ok(ExecutionModeDto::CoreMlFast),
        "CUDA" | "cuda" => Ok(ExecutionModeDto::Cuda),
        "CUDAFast" | "cuda_fast" | "cudafast" => Ok(ExecutionModeDto::CudaFast),
        "MIGraphX" | "migraphx" => Ok(ExecutionModeDto::MiGraphX),
        _ => Err(PyValueError::new_err(format!(
            "unsupported execution mode `{mode}`"
        ))),
    }
}

fn parse_reconstruct_method(
    method: Option<&str>,
    epsilon: Option<f32>,
) -> PyResult<ReconstructMethodDto> {
    match method.unwrap_or("smoothed") {
        "standard" => Ok(ReconstructMethodDto::Standard),
        "smoothed" => Ok(ReconstructMethodDto::Smoothed {
            epsilon: epsilon.unwrap_or(0.1),
        }),
        value => Err(PyValueError::new_err(format!(
            "unsupported reconstruct method `{value}`"
        ))),
    }
}

fn py_result(inner: DiarizationResultDto) -> PyDiarizationResult {
    PyDiarizationResult { inner }
}

fn py_queue_result(result: speakrs_sdk::SdkQueueResult) -> PyQueueResult {
    match result.result {
        Ok(inner) => PyQueueResult {
            job_id: result.job_id,
            file_id: result.file_id,
            result: Some(py_result(inner)),
            error: None,
        },
        Err(error) => PyQueueResult {
            job_id: result.job_id,
            file_id: result.file_id,
            result: None,
            error: Some(py_error_info(error)),
        },
    }
}

fn py_error_info(error: SdkError) -> PyErrorInfo {
    PyErrorInfo {
        category: error.category.code().to_string(),
        code: error.code,
        message: error.message,
    }
}

fn sdk_py_err(error: SdkError) -> PyErr {
    speakrs_error(error.category.code(), &error.code, error.message)
}

fn model_prepare_py_err(error: ModelPrepareError) -> PyErr {
    let category = match error {
        ModelPrepareError::Manifest(_) => SdkErrorCategory::ModelManifest,
        _ => SdkErrorCategory::ModelPrepare,
    };
    speakrs_error(category.code(), category.code(), error.to_string())
}

fn manifest_py_err(error: speakrs_sdk::ModelManifestError) -> PyErr {
    speakrs_error(
        SdkErrorCategory::ModelManifest.code(),
        SdkErrorCategory::ModelManifest.code(),
        error.to_string(),
    )
}

fn speakrs_error(category: &str, code: &str, message: String) -> PyErr {
    SpeakrsError::new_err((category.to_string(), code.to_string(), message))
}
