# Speakrs binding release checklist

This checklist is the release operator path for the public Python, Android, and iOS binding artifacts.

## Required evidence

- Run the package workflow from `.github/workflows/package.yml` on the release commit.
- Confirm the Rust crate job packages `speakrs` and checks the packaged crate as a dependency.
- Confirm the Rust API docs artifact is uploaded.
- Confirm the Python wheel job uploads wheels for Linux x86_64, macOS arm64, and macOS x86_64 for CPython 3.10 through 3.14.
- Confirm clean install/import smoke passes for every built CPU wheel.
- Confirm Android lean and bundled-ORT Maven-local publication, sample APK builds, native ABI inspection, APK fixture asset inspection, and bundled ONNX Runtime APK inspection pass.
- Confirm the Swift XCFramework, Swift package, sample app, zipped binary artifact, and checksum steps pass.
- Run the CUDA and MIGraphX workflow-dispatch jobs on matching self-hosted Linux x86_64 hardware before publishing either GPU package.

## Registry ownership gates

- PyPI: verify maintainer access for `speakrs`, `speakrs-cuda`, and `speakrs-migraphx`.
- Maven Central: verify publishing access for the `com.avencera` namespace.
- SwiftPM: publish the zipped `speakrs_ffiFFI.xcframework.zip` to the release host and update the public package manifest with its URL and checksum.

These gates require registry accounts or release-host credentials and cannot be completed from a local unauthenticated checkout.

## Signing and notarization

- Python wheels: publish with trusted publishing or an API token; no local code-signing step is required for the Rust extension wheels.
- Android AARs: publish signed Maven artifacts using the Maven Central signing key configured in the release environment.
- Swift binary artifact: use the workflow-produced zip and checksum. App signing is handled by downstream iOS apps; the sample project disables code signing for CI builds.
- macOS notarization is not part of this release path because the Swift artifact targets iOS and the Python wheels are extension modules distributed through PyPI.

## Publish blockers

- Do not publish `com.avencera:speakrs-bundled-ort` until the bundled sample APK contains ONNX Runtime native libraries for both required ABIs and runtime smoke passes on Android.
- Do not publish `speakrs-cuda` or `speakrs-migraphx` until the real-hardware fixture smoke passes for the matching package.
- Do not claim CoreML runtime smoke complete until CoreML model assets are available and the iOS/macOS CoreML fixture smoke passes.
