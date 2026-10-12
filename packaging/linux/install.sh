#!/usr/bin/env bash
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
BIN_DIR="${MINIMIX_BIN_DIR:-$HOME/.local/bin}"
UNIT_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user"
INSTALL_UNIT="${MINIMIX_INSTALL_SYSTEMD:-ask}"

mkdir -p "$BIN_DIR"
install -m 755 "$HERE/minimix-daemon" "$BIN_DIR/minimix-daemon"
install -m 755 "$HERE/minimix" "$BIN_DIR/minimix"

case ":$PATH:" in
  *":$BIN_DIR:"*) ;;
  *)
    echo "Add $BIN_DIR to PATH if the binaries are not found."
    ;;
esac

install_unit() {
  mkdir -p "$UNIT_DIR"
  cat >"$UNIT_DIR/minimix-daemon.service" <<EOF
[Unit]
Description=Minimix virtual microphone daemon
After=pipewire.service

[Service]
ExecStart=$BIN_DIR/minimix-daemon
Restart=on-failure

[Install]
WantedBy=default.target
EOF
  if command -v systemctl >/dev/null 2>&1; then
    systemctl --user daemon-reload
    systemctl --user enable --now minimix-daemon.service
    echo "Enabled systemd user unit minimix-daemon.service"
  else
    echo "Wrote $UNIT_DIR/minimix-daemon.service (systemctl not available)"
  fi
}

case "$INSTALL_UNIT" in
  1|yes|true)
    install_unit
    ;;
  0|no|false)
    ;;
  *)
    if [[ -t 0 ]]; then
      read -r -p "Install systemd user unit for minimix-daemon? [y/N] " ans
      case "$ans" in
        y|Y|yes|YES) install_unit ;;
      esac
    fi
    ;;
esac

echo "Installed minimix and minimix-daemon to $BIN_DIR"
echo "Needs PipeWire 1.0.2+, WirePlumber, and pipewire-pulse. See NOTES.txt."
