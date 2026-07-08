# DiarizeFile iOS sample

This sample app depends on the local `Speakrs` Swift package, prepares models with the SDK-owned `prepareModels` API, and diarizes the bundled `test_short.wav` fixture.

Build the package artifact first:

```sh
./bindings/swift/scripts/build-xcframework.sh
./bindings/swift/scripts/package-xcframework.sh
```

Then generate and build the app:

```sh
cd bindings/swift/Samples/DiarizeFile
xcodegen generate
xcodebuild -project DiarizeFile.xcodeproj -scheme DiarizeFile -destination 'generic/platform=iOS Simulator' -sdk iphonesimulator build
```

For local CoreML runtime smoke, add the required CoreML model files to the app bundle under `Models/`, or let `prepareModels` use the platform cache/download path when those artifacts are available.
