#!/usr/bin/env bash
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
HAL_DIR="/Library/Audio/Plug-Ins/HAL"
BIN_DIR="${MINIMIX_BIN_DIR:-/usr/local/bin}"
DRIVER_SRC="$HERE/Minimix.driver"

as_root() {
  if [[ "$(id -u)" -eq 0 ]]; then
    "$@"
  else
    sudo "$@"
  fi
}

if [[ ! -d "$DRIVER_SRC" ]]; then
  echo "missing Minimix.driver in package" >&2
  exit 1
fi

echo "Installing HAL plug-in to $HAL_DIR..."
as_root mkdir -p "$HAL_DIR"
as_root rm -rf "$HAL_DIR/Minimix.driver"
as_root cp -R "$DRIVER_SRC" "$HAL_DIR/Minimix.driver"

echo "Restarting coreaudiod..."
as_root killall coreaudiod 2>/dev/null || true
sleep 1

as_root mkdir -p "$BIN_DIR"
as_root install -m 755 "$HERE/minimix-daemon" "$BIN_DIR/minimix-daemon"
as_root install -m 755 "$HERE/minimix" "$BIN_DIR/minimix"

cat <<'EOF'

Installed unsigned Minimix.driver + binaries.

Live Discord / Sound Settings input needs a Developer ID signed plug-in.
Ad-hoc codesign often fails with AppleMobileFileIntegrityError -423 (AMFI).
See docs/macos-hal.md and NOTES.txt.

Start the daemon after install:
  minimix-daemon
EOF
