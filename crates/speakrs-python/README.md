# speakrs

Fast speaker diarization with native Rust execution.

## Install

CPU package:

Build a local wheel from the repository:

```sh
uvx maturin build -m crates/speakrs-python/Cargo.toml -i python3.12
```

Install the generated wheel into your Python environment:

```sh
python -m pip install target/wheels/speakrs-*.whl
```

GPU packages use the same `import speakrs` API and are separate distributions:

| Distribution | Runtime | Platform |
| --- | --- | --- |
| `speakrs` | CPU ONNX Runtime | macOS arm64/x86_64, Linux x86_64 |
| `speakrs-cuda` | CUDA / CUDAFast | Linux x86_64 with CUDA 12 and cuDNN 9 |
| `speakrs-migraphx` | MIGraphX | Linux x86_64 with ROCm/MIGraphX |

Install only one `speakrs` distribution in an environment. GPU packages are release-ready only after the matching real-hardware fixture smoke has passed.

## Use

```python
import speakrs

prepared = speakrs.prepare(model_dir="fixtures/models")
pipeline = speakrs.Pipeline.from_prepared(prepared)
result = pipeline.diarize_file("fixtures/test.wav")
print(result.rttm)
```

## CLI

```sh
speakrs fixtures/test.wav --model-dir fixtures/models
```

## Queue

```python
prepared = speakrs.prepare(model_dir="fixtures/models")
pipeline = speakrs.Pipeline.from_prepared(prepared)
queue = pipeline.into_queue()

queue.push_file("file1", "fixtures/test.wav")
result = queue.recv().unwrap()
print(result.rttm)
```
