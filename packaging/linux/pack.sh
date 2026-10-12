#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=../lib/common.sh
source "$SCRIPT_DIR/../lib/common.sh"

ROOT="$(packaging_root)"
cd "$ROOT"

PLATFORM="linux-x86_64"
VERSION="$(minimix_version)"
STAGE="$(stage_dir_for "$VERSION" "$PLATFORM")"
STAGE_NAME="$(basename "$STAGE")"
STAGE_PARENT="$(dirname "$STAGE")"
OUT="$ROOT/dist/$(artifact_name "$VERSION" "$PLATFORM")"

cargo build --release -p minimix-daemon -p minimix

rm -rf "$STAGE"
mkdir -p "$STAGE"
cp "$ROOT/target/release/minimix-daemon" "$STAGE/"
cp "$ROOT/target/release/minimix" "$STAGE/"
cp "$SCRIPT_DIR/install.sh" "$STAGE/install.sh"
chmod +x "$STAGE/minimix-daemon" "$STAGE/minimix" "$STAGE/install.sh"
write_notes "$PLATFORM" "$STAGE/NOTES.txt"

make_archive "$PLATFORM" "$STAGE_PARENT" "$STAGE_NAME" "$OUT"
echo "packed $OUT"
