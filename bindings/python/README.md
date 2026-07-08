# Python package variants

The default `speakrs` package is the CPU package and bundles CPU ONNX Runtime.

GPU package variants use the same `import speakrs` API and are distributed as separate, mutually exclusive Python distributions:

| Distribution | Import | Runtime | Platform |
| --- | --- | --- | --- |
| `speakrs` | `speakrs` | CPU ONNX Runtime | macOS arm64/x86_64, Linux x86_64 |
| `speakrs-cuda` | `speakrs` | CUDA / CUDAFast | Linux x86_64 with CUDA 12 and cuDNN 9 |
| `speakrs-migraphx` | `speakrs` | MIGraphX | Linux x86_64 with ROCm/MIGraphX |

Do not publish `speakrs-cuda` or `speakrs-migraphx` as release-ready until the matching real-hardware fixture smoke has passed.

Local metadata checks:

```sh
mkdir -p _scratch/speakrs-cuda-dist-info _scratch/speakrs-migraphx-dist-info
cd bindings/python/speakrs-cuda
uvx maturin pep517 write-dist-info --metadata-directory ../../../_scratch/speakrs-cuda-dist-info
cd ../speakrs-migraphx
uvx maturin pep517 write-dist-info --metadata-directory ../../../_scratch/speakrs-migraphx-dist-info
```

Linux wheel builds:

```sh
cd bindings/python/speakrs-cuda
uvx maturin build --release -i python3.12 --compatibility linux

cd ../speakrs-migraphx
uvx maturin build --release -i python3.12 --compatibility linux
```
