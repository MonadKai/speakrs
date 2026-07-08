# Python API

## Lifecycle

Use `speakrs.prepare(...)` before diarization. It returns `PreparedModels`, which is passed to `Pipeline.from_prepared(...)`.

The CPU distribution is named `speakrs`. GPU distributions are named `speakrs-cuda` and `speakrs-migraphx`; they install the same `speakrs` import package and expose the same API surface.

## Models and Cache

- `prepare(mode=ExecutionMode.CPU, cache_dir=None, model_dir=None, manifest=None, progress=None)`
- `generate_manifest(model_dir, mode=ExecutionMode.CPU)`
- `required_model_files(mode=ExecutionMode.CPU)`
- `default_cache_dir()`
- `list_cache(cache_dir=None)`
- `cleanup_revision(revision=DEFAULT_MODEL_REVISION, cache_dir=None)`

## Pipeline

- `Pipeline.from_prepared(prepared, mode=ExecutionMode.CPU, pipeline_config=None, runtime_config=None)`
- `Pipeline.diarize_samples(samples, file_id="file1", pipeline_config=None, cancel_token=None, progress=None)`
- `Pipeline.diarize_file(path, file_id="file1", pipeline_config=None, cancel_token=None, progress=None)`
- `Pipeline.diarize_samples_async(...)`
- `Pipeline.diarize_file_async(...)`
- `Pipeline.into_queue(pipeline_config=None)`

## Queue

- `Queue.push_samples(file_id, samples)`
- `Queue.push_file(file_id, path)`
- `Queue.recv()`
- `Queue.recv_async()`
- `Queue.try_recv()`

## Typed Objects

The package exposes `ExecutionMode`, `ProgressEvent`, `CancelToken`, `BinarizeConfig`, `AhcConfig`, `VbxConfig`, `PipelineConfig`, `RuntimeConfig`, `PreparedModels`, `ModelManifest`, `Segment`, `TimingStats`, `DiarizationResult`, `QueueResult`, and `SpeakrsError`.

## Runtime Packages

| Distribution | Enabled modes | Release gate |
| --- | --- | --- |
| `speakrs` | `CPU` | wheel build, install/import, and CPU fixture smoke |
| `speakrs-cuda` | `CUDA`, `CUDAFast` | Linux x86_64 CUDA hardware wheel build, install/import, and fixture smoke |
| `speakrs-migraphx` | `MIGraphX` | Linux x86_64 ROCm/MIGraphX hardware wheel build, install/import, and fixture smoke |

Unsupported mode/package/platform combinations raise `SpeakrsError` with a stable category and message.
