# Speakrs Swift Package

This package is generated from the Rust UniFFI wrapper crate and targets iOS 15+.

## Build

Build the local binary artifact before resolving or building the package:

```sh
./bindings/swift/scripts/build-xcframework.sh
./bindings/swift/scripts/package-xcframework.sh
xcodebuild -scheme Speakrs -destination 'generic/platform=iOS Simulator' -sdk iphonesimulator -derivedDataPath bindings/swift/build/xcode build
xcodebuild -scheme Speakrs -destination 'generic/platform=iOS' -sdk iphoneos -derivedDataPath bindings/swift/build/xcode-device build
```

The build script creates `artifacts/speakrs_ffiFFI.xcframework` and regenerates `Sources/Speakrs/speakrs_ffi.swift`.
The package script creates `artifacts/speakrs_ffiFFI.xcframework.zip` and writes its SwiftPM checksum to `artifacts/speakrs_ffiFFI.xcframework.zip.checksum`.

## Package

The Swift package product is `Speakrs`. It depends on the generated binary target `speakrs_ffiFFI`.

Add the package from this directory during local development:

```swift
.package(path: "bindings/swift")
```

Then depend on the product:

```swift
.product(name: "Speakrs", package: "Speakrs")
```

## Sample

The iOS sample app lives at `Samples/DiarizeFile`. It depends on the local Swift package, prepares models with `prepareModels`, builds a `SpeakrsPipeline` from the prepared handle, and diarizes the bundled `test_short.wav` fixture.

```sh
cd bindings/swift/Samples/DiarizeFile
xcodegen generate
xcodebuild -project DiarizeFile.xcodeproj -scheme DiarizeFile -destination 'generic/platform=iOS Simulator' -sdk iphonesimulator -derivedDataPath build/xcode build
```

CoreML runtime smoke requires the matching CoreML model assets to be present in the app bundle or available through the SDK model cache.

## Release

Public SwiftPM releases use the zipped XCFramework as a binary target. After uploading `speakrs_ffiFFI.xcframework.zip` to the release host, update the public package manifest to use the hosted URL and the checksum from `speakrs_ffiFFI.xcframework.zip.checksum`.

## Runtime Notes

The `CoreML` and `CoreMLFast` modes use the native CoreML path and do not require ONNX Runtime. CPU, CUDA, CUDAFast, and MIGraphX are visible in the public enum and fail with typed runtime errors when the package or platform does not support them.
