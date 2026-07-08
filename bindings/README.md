# Speakrs Bindings

This directory contains the public bindings for the `speakrs` Rust diarization engine.

## Packages

| Platform | Package | Runtime policy |
| --- | --- | --- |
| Python | `speakrs` | CPU package with bundled ONNX Runtime |
| Python | `speakrs-cuda` | CUDA package for Linux x86_64 systems with compatible NVIDIA runtime |
| Python | `speakrs-migraphx` | MIGraphX package for Linux x86_64 systems with compatible ROCm/MIGraphX runtime |
| Android | `com.avencera:speakrs-lean` | AAR where the app supplies ONNX Runtime |
| Android | `com.avencera:speakrs-bundled-ort` | AAR that supplies ONNX Runtime through the Microsoft Android AAR |
| iOS | `Speakrs` | Swift Package binary XCFramework for iOS 15+ |

All public artifacts use the same version as the root Rust crate.

## Guides

- Python: [`python/README.md`](python/README.md)
- Android: [`android/README.md`](android/README.md)
- Swift: [`swift/README.md`](swift/README.md)
- Release checklist: [`RELEASE.md`](RELEASE.md)

## Release Checks

Release candidates should pass the package workflow and the platform-specific smoke checks before publishing:

- Rust crate packaging and downstream dependency checks
- Rust API docs generation
- Python wheel build, clean install, and fixture smoke for the supported CPython and platform matrix
- Android lean and bundled-ORT AAR Maven-local publication, sample APK builds, native ABI inspection, and bundled ONNX Runtime APK inspection
- Swift XCFramework build, zipped binary artifact generation, Swift Package build, sample app build, and SwiftPM checksum generation
- CUDA and MIGraphX package build, install, and runtime smoke on matching Linux x86_64 hardware

GPU packages are not publishable until their hardware smoke checks pass.
