#!/usr/bin/env bash
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
HAL_DIR="/Library/Audio/Plug-Ins/HAL"
BIN_DIR="${MINIMIX_BIN_DIR:-/usr/local/bin}"
DRIVER_SRC="$HERE/Minimix.driver"

if [[ ! -d "$DRIVER_SRC" ]]; then
  echo "missing Minimix.driver in archive" >&2
  exit 1
fi

echo "Installing HAL plug-in to $HAL_DIR (sudo)..."
sudo mkdir -p "$HAL_DIR"
sudo rm -rf "$HAL_DIR/Minimix.driver"
sudo cp -R "$DRIVER_SRC" "$HAL_DIR/Minimix.driver"

echo "Restarting coreaudiod..."
sudo killall coreaudiod 2>/dev/null || true
sleep 1

sudo mkdir -p "$BIN_DIR"
sudo install -m 755 "$HERE/minimix-daemon" "$BIN_DIR/minimix-daemon"
sudo install -m 755 "$HERE/minimix" "$BIN_DIR/minimix"

APP_HINT="$HOME/Applications"
mkdir -p "$APP_HINT"
echo "GUI binary is at $BIN_DIR/minimix. You can also keep a copy under $APP_HINT."

cat <<'EOF'

Installed unsigned Minimix.driver + binaries.

Live Discord / Sound Settings input needs a Developer ID signed plug-in.
Ad-hoc codesign often fails with AppleMobileFileIntegrityError -423 (AMFI).
See docs/macos-hal.md and NOTES.txt.

Start the daemon after install:
  minimix-daemon
EOF
