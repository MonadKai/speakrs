#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PACKAGE_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
ARTIFACT_DIR="$PACKAGE_DIR/artifacts"
XCFRAMEWORK="$ARTIFACT_DIR/speakrs_ffiFFI.xcframework"
ZIP="$ARTIFACT_DIR/speakrs_ffiFFI.xcframework.zip"
CHECKSUM_FILE="$ZIP.checksum"

if [[ ! -d "$XCFRAMEWORK" ]]; then
  echo "missing $XCFRAMEWORK; run bindings/swift/scripts/build-xcframework.sh first" >&2
  exit 1
fi

rm -f "$ZIP" "$CHECKSUM_FILE"

(
  cd "$ARTIFACT_DIR"
  ditto -c -k --sequesterRsrc --keepParent "$(basename "$XCFRAMEWORK")" "$(basename "$ZIP")"
)

checksum="$(swift package --package-path "$PACKAGE_DIR" compute-checksum "$ZIP")"
printf '%s\n' "$checksum" > "$CHECKSUM_FILE"
printf 'Swift binary artifact: %s\nChecksum: %s\n' "$ZIP" "$checksum"
