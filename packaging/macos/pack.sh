#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=../lib/common.sh
source "$SCRIPT_DIR/../lib/common.sh"

ROOT="$(packaging_root)"
cd "$ROOT"

case "$(uname -m)" in
  arm64|aarch64) PLATFORM="macos-aarch64" ;;
  x86_64) PLATFORM="macos-x86_64" ;;
  *)
    echo "unsupported macOS arch: $(uname -m)" >&2
    exit 1
    ;;
esac

VERSION="$(minimix_version)"
STAGE="$(stage_dir_for "$VERSION" "$PLATFORM")"
STAGE_NAME="$(basename "$STAGE")"
STAGE_PARENT="$(dirname "$STAGE")"
OUT_TGZ="$ROOT/dist/$(artifact_name_ext "$VERSION" "$PLATFORM" tar.gz)"
OUT_DMG="$ROOT/dist/$(artifact_name_ext "$VERSION" "$PLATFORM" dmg)"

cargo build --release -p minimix-daemon -p minimix -p minimix-hal-driver
"$ROOT/platforms/macos/hal-driver/scripts/assemble-driver.sh" release

DRIVER_BUNDLE="$ROOT/target/release/Minimix.driver"
if [[ ! -d "$DRIVER_BUNDLE" ]]; then
  echo "missing $DRIVER_BUNDLE" >&2
  exit 1
fi

rm -rf "$STAGE"
mkdir -p "$STAGE"
cp "$ROOT/target/release/minimix-daemon" "$STAGE/"
cp "$ROOT/target/release/minimix" "$STAGE/"
cp -R "$DRIVER_BUNDLE" "$STAGE/Minimix.driver"
cp "$SCRIPT_DIR/install.sh" "$STAGE/install.sh"
chmod +x "$STAGE/minimix-daemon" "$STAGE/minimix" "$STAGE/install.sh"
write_notes "$PLATFORM" "$STAGE/NOTES.txt"

make_archive_as tar.gz "$STAGE_PARENT" "$STAGE_NAME" "$OUT_TGZ"
echo "packed $OUT_TGZ"

DMG_ROOT="$ROOT/dist/dmg-root/$PLATFORM"
rm -rf "$DMG_ROOT"
mkdir -p "$DMG_ROOT"
"$SCRIPT_DIR/build-installer-app.sh" "$STAGE" "$DMG_ROOT/Install Minimix.app"
cp "$STAGE/NOTES.txt" "$DMG_ROOT/NOTES.txt"
cat >"$DMG_ROOT/README.txt" <<'EOF'
Minimix for macOS

1. Double-click "Install Minimix".
2. Enter your Mac password when prompted.
3. If macOS blocks the app (unsigned): right-click Install Minimix → Open → Open.
4. In Terminal run: minimix-daemon

Live Discord / Sound Settings usually needs Developer ID signing.
See NOTES.txt.
EOF

rm -f "$OUT_DMG"
hdiutil create \
  -volname "Minimix" \
  -srcfolder "$DMG_ROOT" \
  -ov \
  -format UDZO \
  "$OUT_DMG"
echo "packed $OUT_DMG"
