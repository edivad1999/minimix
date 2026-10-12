#!/usr/bin/env bash
# Assemble Minimix.driver from the release cdylib + committed Info.plist.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../../.." && pwd)"
CRATE_DIR="$(cd "$(dirname "$0")/.." && pwd)"
PROFILE="${1:-release}"
TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
DYLIB="$TARGET_DIR/$PROFILE/libminimix_hal_driver.dylib"
BUNDLE="$TARGET_DIR/$PROFILE/Minimix.driver"

if [[ ! -f "$DYLIB" ]]; then
  echo "missing $DYLIB — run: cargo build --$PROFILE -p minimix-hal-driver" >&2
  exit 1
fi

rm -rf "$BUNDLE"
mkdir -p "$BUNDLE/Contents/MacOS"
cp "$CRATE_DIR/Info.plist" "$BUNDLE/Contents/Info.plist"
cp "$DYLIB" "$BUNDLE/Contents/MacOS/Minimix"

echo "assembled $BUNDLE"
