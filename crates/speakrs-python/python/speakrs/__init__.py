from __future__ import annotations

import asyncio
from dataclasses import dataclass
from enum import Enum
from pathlib import Path
from typing import Callable, Iterable

from . import _native

__version__ = "0.5.0"
DEFAULT_MODEL_REVISION = _native._default_model_revision()
SpeakrsError = _native.SpeakrsError


class ExecutionMode(str, Enum):
    CPU = "CPU"
    CoreML = "CoreML"
    CoreMLFast = "CoreMLFast"
    CUDA = "CUDA"
    CUDAFast = "CUDAFast"
    MIGraphX = "MIGraphX"


class ProgressEvent(str, Enum):
    PreparingModels = "preparing_models"
    DownloadingModel = "downloading_model"
    VerifyingModelManifest = "verifying_model_manifest"
    DecodingAudio = "decoding_audio"
    ResamplingAudio = "resampling_audio"
    RunningPipeline = "running_pipeline"
    QueueWait = "queue_wait"
    Completed = "completed"


@dataclass(frozen=True)
class BinarizeConfig:
    onset: float | None = None
    offset: float | None = None
    min_duration_on: int | None = None
    min_duration_off: int | None = None
    pad_onset: int | None = None
    pad_offset: int | None = None

    def _native(self):
        return _native._BinarizeConfig(
            self.onset,
            self.offset,
            self.min_duration_on,
            self.min_duration_off,
            self.pad_onset,
            self.pad_offset,
        )


@dataclass(frozen=True)
class AhcConfig:
    threshold: float | None = None

    def _native(self):
        return _native._AhcConfig(self.threshold)


@dataclass(frozen=True)
class VbxConfig:
    fa: float | None = None
    fb: float | None = None
    max_iters: int | None = None
    epsilon: float | None = None
    init_smoothing: float | None = None

    def _native(self):
        return _native._VbxConfig(
            self.fa,
            self.fb,
            self.max_iters,
            self.epsilon,
            self.init_smoothing,
        )


@dataclass(frozen=True)
class PipelineConfig:
    binarize: BinarizeConfig | None = None
    ahc: AhcConfig | None = None
    vbx: VbxConfig | None = None
    merge_gap: float | None = None
    speaker_keep_threshold: float | None = None
    reconstruct_method: str | None = None
    reconstruct_epsilon: float | None = None

    @classmethod
    def for_mode(cls, mode: ExecutionMode | str) -> _native._PipelineConfig:
        return _native._PipelineConfig.for_mode(_mode_value(mode))

    def _native(self):
        return _native._PipelineConfig(
            self.binarize._native() if self.binarize else None,
            self.ahc._native() if self.ahc else None,
            self.vbx._native() if self.vbx else None,
            self.merge_gap,
            self.speaker_keep_threshold,
            self.reconstruct_method,
            self.reconstruct_epsilon,
        )


@dataclass(frozen=True)
class RuntimeConfig:
    chunk_emb_workers: int | None = None

    def _native(self):
        return _native._RuntimeConfig(self.chunk_emb_workers)


CancelToken = _native._CancelToken
ModelManifest = _native._ModelManifest
ModelManifestEntry = _native._ModelManifestEntry
PreparedModels = _native._PreparedModels
CacheEntry = _native._CacheEntry
Segment = _native._Segment
TimingStats = _native._TimingStats
DiarizationResult = _native._DiarizationResult
QueueResult = _native._QueueResult


def default_cache_dir() -> Path:
    return Path(_native._default_cache_dir())


def required_model_files(mode: ExecutionMode | str = ExecutionMode.CPU) -> list[str]:
    return _native._required_model_files(_mode_value(mode))


def generate_manifest(
    model_dir: str | Path,
    mode: ExecutionMode | str = ExecutionMode.CPU,
) -> ModelManifest:
    return _native._generate_manifest(Path(model_dir), _mode_value(mode))


def prepare(
    mode: ExecutionMode | str = ExecutionMode.CPU,
    *,
    cache_dir: str | Path | None = None,
    model_dir: str | Path | None = None,
    manifest: ModelManifest | None = None,
    progress: Callable[[ProgressEvent], None] | None = None,
) -> PreparedModels:
    if progress:
        progress(ProgressEvent.PreparingModels)
    model_path = Path(model_dir) if model_dir is not None else None
    if manifest is None and model_path is not None:
        if progress:
            progress(ProgressEvent.VerifyingModelManifest)
        manifest = generate_manifest(model_path, mode)
    prepared = _native._prepare(
        _mode_value(mode),
        Path(cache_dir) if cache_dir is not None else None,
        model_path,
        manifest,
    )
    if progress:
        progress(ProgressEvent.Completed)
    return prepared


def list_cache(cache_dir: str | Path | None = None) -> list[CacheEntry]:
    return _native._list_cache(Path(cache_dir) if cache_dir is not None else None)


def cleanup_revision(
    revision: str = DEFAULT_MODEL_REVISION,
    *,
    cache_dir: str | Path | None = None,
) -> bool:
    return _native._cleanup_revision(
        revision,
        Path(cache_dir) if cache_dir is not None else None,
    )


class Pipeline:
    def __init__(self, native):
        self._native = native

    @classmethod
    def from_prepared(
        cls,
        prepared: PreparedModels,
        mode: ExecutionMode | str = ExecutionMode.CPU,
        *,
        pipeline_config: PipelineConfig | _native._PipelineConfig | None = None,
        runtime_config: RuntimeConfig | _native._RuntimeConfig | None = None,
    ) -> Pipeline:
        return cls(
            _native._build_pipeline(
                prepared,
                _mode_value(mode),
                _native_pipeline_config(pipeline_config),
                _native_runtime_config(runtime_config),
            )
        )

    def diarize_samples(
        self,
        samples: Iterable[float],
        *,
        file_id: str = "file1",
        pipeline_config: PipelineConfig | _native._PipelineConfig | None = None,
        cancel_token: CancelToken | None = None,
        progress: Callable[[ProgressEvent], None] | None = None,
    ) -> DiarizationResult:
        if progress:
            progress(ProgressEvent.RunningPipeline)
        result = self._native.diarize_samples(
            list(samples),
            file_id,
            _native_pipeline_config(pipeline_config),
            cancel_token,
        )
        if progress:
            progress(ProgressEvent.Completed)
        return result

    async def diarize_samples_async(self, *args, **kwargs) -> DiarizationResult:
        return await asyncio.to_thread(self.diarize_samples, *args, **kwargs)

    def diarize_file(
        self,
        path: str | Path,
        *,
        file_id: str = "file1",
        pipeline_config: PipelineConfig | _native._PipelineConfig | None = None,
        cancel_token: CancelToken | None = None,
        progress: Callable[[ProgressEvent], None] | None = None,
    ) -> DiarizationResult:
        if progress:
            progress(ProgressEvent.DecodingAudio)
            progress(ProgressEvent.ResamplingAudio)
        result = self._native.diarize_file(
            Path(path),
            file_id,
            _native_pipeline_config(pipeline_config),
            cancel_token,
        )
        if progress:
            progress(ProgressEvent.Completed)
        return result

    async def diarize_file_async(self, *args, **kwargs) -> DiarizationResult:
        return await asyncio.to_thread(self.diarize_file, *args, **kwargs)

    def into_queue(
        self,
        pipeline_config: PipelineConfig | _native._PipelineConfig | None = None,
    ) -> Queue:
        return Queue(self._native.into_queue(_native_pipeline_config(pipeline_config)))


class Queue:
    def __init__(self, native):
        self._native = native

    def push_samples(self, file_id: str, samples: Iterable[float]) -> int:
        return self._native.push_samples(file_id, list(samples))

    def push_file(self, file_id: str, path: str | Path) -> int:
        return self._native.push_file(file_id, Path(path))

    def recv(self) -> QueueResult:
        return self._native.recv()

    async def recv_async(self) -> QueueResult:
        return await asyncio.to_thread(self.recv)

    def try_recv(self) -> QueueResult | None:
        return self._native.try_recv()


def _mode_value(mode: ExecutionMode | str) -> str:
    return mode.value if isinstance(mode, ExecutionMode) else str(mode)


def _native_pipeline_config(config):
    if config is None:
        return None
    if isinstance(config, PipelineConfig):
        return config._native()
    return config


def _native_runtime_config(config):
    if config is None:
        return None
    if isinstance(config, RuntimeConfig):
        return config._native()
    return config


__all__ = [
    "AhcConfig",
    "BinarizeConfig",
    "CacheEntry",
    "CancelToken",
    "DEFAULT_MODEL_REVISION",
    "DiarizationResult",
    "ExecutionMode",
    "ModelManifest",
    "ModelManifestEntry",
    "Pipeline",
    "PipelineConfig",
    "PreparedModels",
    "ProgressEvent",
    "Queue",
    "QueueResult",
    "RuntimeConfig",
    "Segment",
    "SpeakrsError",
    "TimingStats",
    "VbxConfig",
    "cleanup_revision",
    "default_cache_dir",
    "generate_manifest",
    "list_cache",
    "prepare",
    "required_model_files",
]
