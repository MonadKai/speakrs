# Speakrs Android

The Android package is a Gradle project with a UniFFI-backed Android library and a sample app.

## Artifacts

| Gradle variant | Intended Maven artifact | Runtime policy |
| --- | --- | --- |
| `bundledOrtRelease` | `com.avencera:speakrs-bundled-ort` | depends on the Microsoft ONNX Runtime Android AAR and packages its native libraries into apps |
| `leanRelease` | `com.avencera:speakrs-lean` | app supplies ONNX Runtime native libraries |

Both variants target Android minSdk 24 and build native libraries for `arm64-v8a` and `x86_64`.

## Build

Set `ANDROID_HOME` or `ANDROID_SDK_ROOT` to the Android SDK. The project uses NDK `29.0.14206865`.

Generate Kotlin bindings:

```sh
cd bindings/android
./gradlew :speakrs:generateUniFfiKotlin
```

Build the lean AAR and sample APK:

```sh
cd bindings/android
./gradlew :speakrs:assembleLeanRelease :sample:assembleLeanRelease
```

Build the bundled-ORT AAR:

```sh
cd bindings/android
./gradlew :speakrs:assembleBundledOrtRelease
```

Build both Maven-local artifacts and sample APKs:

```sh
cd bindings/android
./gradlew :speakrs:publishLeanReleasePublicationToMavenLocal :speakrs:publishBundledOrtReleasePublicationToMavenLocal :sample:assembleLeanRelease :sample:assembleBundledOrtRelease
```

## Sample

The sample app prepares models through the SDK API and runs `test_short.wav` from its bundled assets through the diarization pipeline. Build the lean sample with:

```sh
cd bindings/android
./gradlew :sample:assembleLeanRelease
```

Use the `bundledOrt` flavor when the bundled AAR has both required ABIs. Use the `lean` flavor when the host app packages ONNX Runtime itself. To diarize another local file, launch the activity with `com.avencera.speakrs.sample.AUDIO_PATH` set to that file path.
