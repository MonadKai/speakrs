# speakrs-migraphx

`speakrs-migraphx` is the Linux x86_64 ROCm/MIGraphX Python distribution for `speakrs`.

It installs the same `speakrs` Python module as the CPU package and enables the `MIGraphX` execution mode. Install only one `speakrs` distribution in an environment.

Runtime requirements:

- Linux x86_64
- ROCm with MIGraphX
- AMD GPU supported by the installed ROCm/MIGraphX runtime

This package is not release-ready until a real AMD/ROCm hardware fixture smoke has passed for the built artifact.
