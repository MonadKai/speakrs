# speakrs-cuda

`speakrs-cuda` is the Linux x86_64 CUDA Python distribution for `speakrs`.

It installs the same `speakrs` Python module as the CPU package and enables the `CUDA` and `CUDAFast` execution modes. Install only one `speakrs` distribution in an environment.

Runtime requirements:

- Linux x86_64
- CUDA 12
- cuDNN 9
- NVIDIA driver compatible with the selected CUDA runtime

This package is not release-ready until a real NVIDIA hardware fixture smoke has passed for the built artifact.
