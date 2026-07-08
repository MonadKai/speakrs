#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PACKAGE_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
REPO_ROOT="$(cd "$PACKAGE_DIR/../.." && pwd)"

CRATE="speakrs-ffi"
FEATURES="online,ios-coreml"
IOS_DEPLOYMENT_TARGET="${IOS_DEPLOYMENT_TARGET:-15.0}"
LIB_NAME="libspeakrs_ffi.a"
HOST_DYLIB="$REPO_ROOT/target/release/libspeakrs_ffi.dylib"
BUILD_DIR="$PACKAGE_DIR/build"
GENERATED_DIR="$BUILD_DIR/generated"
SWIFT_SOURCE_DIR="$PACKAGE_DIR/Sources/Speakrs"
HEADERS_DIR="$BUILD_DIR/headers"
ARTIFACT_DIR="$PACKAGE_DIR/artifacts"
XCFRAMEWORK="$ARTIFACT_DIR/speakrs_ffiFFI.xcframework"

TARGETS=(
  "aarch64-apple-ios"
  "aarch64-apple-ios-sim"
  "x86_64-apple-ios"
)

cd "$REPO_ROOT"

cargo build -p "$CRATE" --release

rm -rf "$GENERATED_DIR" "$HEADERS_DIR" "$XCFRAMEWORK"
mkdir -p "$GENERATED_DIR" "$SWIFT_SOURCE_DIR" "$HEADERS_DIR" "$ARTIFACT_DIR"

cargo run -p "$CRATE" --features bindgen-cli --bin uniffi-bindgen -- \
  generate \
  --language swift \
  --metadata-no-deps \
  --out-dir "$GENERATED_DIR" \
  --library "$HOST_DYLIB"

cp "$GENERATED_DIR/speakrs_ffi.swift" "$SWIFT_SOURCE_DIR/speakrs_ffi.swift"
cp "$GENERATED_DIR/speakrs_ffiFFI.h" "$HEADERS_DIR/speakrs_ffiFFI.h"
cp "$GENERATED_DIR/speakrs_ffiFFI.modulemap" "$HEADERS_DIR/module.modulemap"

for target in "${TARGETS[@]}"; do
  export IPHONEOS_DEPLOYMENT_TARGET="$IOS_DEPLOYMENT_TARGET"
  cargo build \
    -p "$CRATE" \
    --release \
    --no-default-features \
    --features "$FEATURES" \
    --target "$target"
done

DEVICE_LIB="$REPO_ROOT/target/aarch64-apple-ios/release/$LIB_NAME"
SIM_ARM64_LIB="$REPO_ROOT/target/aarch64-apple-ios-sim/release/$LIB_NAME"
SIM_X86_64_LIB="$REPO_ROOT/target/x86_64-apple-ios/release/$LIB_NAME"
SIM_LIB="$BUILD_DIR/ios-simulator/$LIB_NAME"

mkdir -p "$(dirname "$SIM_LIB")"
lipo -create "$SIM_ARM64_LIB" "$SIM_X86_64_LIB" -output "$SIM_LIB"

xcodebuild -create-xcframework \
  -library "$DEVICE_LIB" \
  -headers "$HEADERS_DIR" \
  -library "$SIM_LIB" \
  -headers "$HEADERS_DIR" \
  -output "$XCFRAMEWORK"
