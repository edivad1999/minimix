#!/usr/bin/env bash
# Sign Minimix.driver and install into /Library/Audio/Plug-Ins/HAL/.
#
# Developer ID Application signing is required for the device to appear in
# System Settings on modern macOS. Ad-hoc signing (--sign -) may discover the
# bundle but AMFI rejects load with AppleMobileFileIntegrityError -423.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../../.." && pwd)"
PROFILE="${1:-release}"
TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
BUNDLE="$TARGET_DIR/$PROFILE/Minimix.driver"
IDENTITY="${MINIMIX_CODESIGN_IDENTITY:--}"
HAL_DIR="/Library/Audio/Plug-Ins/HAL"

if [[ ! -d "$BUNDLE" ]]; then
  echo "missing $BUNDLE — run scripts/assemble-driver.sh first" >&2
  exit 1
fi

echo "codesign identity: $IDENTITY"
codesign --force --sign "$IDENTITY" --timestamp=none "$BUNDLE"
codesign --verify --verbose=2 "$BUNDLE"

echo "installing into $HAL_DIR (sudo)"
sudo mkdir -p "$HAL_DIR"
sudo rm -rf "$HAL_DIR/Minimix.driver"
sudo cp -R "$BUNDLE" "$HAL_DIR/Minimix.driver"
sudo killall coreaudiod || true

if [[ "$IDENTITY" == "-" ]]; then
  cat <<'EOF'

Ad-hoc signature applied. On macOS 15+ the Core Audio Driver Service helper
often rejects ad-hoc HAL plug-ins with AMFI error -423 before the device
enumerates. A Developer ID Application identity is required for a Discord-
visible mic. Set MINIMIX_CODESIGN_IDENTITY to that identity and re-run.
EOF
fi
