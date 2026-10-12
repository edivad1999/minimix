#!/usr/bin/env bash
# Assemble Install Minimix.app from stage payloads into DEST_APP.
set -euo pipefail

STAGE="${1:?stage dir with Minimix.driver, bins, install.sh, NOTES.txt}"
DEST_APP="${2:?destination Install Minimix.app path}"
TEMPLATE="$(cd "$(dirname "$0")" && pwd)/InstallMinimix"

rm -rf "$DEST_APP"
mkdir -p "$DEST_APP/Contents/MacOS" "$DEST_APP/Contents/Resources"

cp "$TEMPLATE/Contents/Info.plist" "$DEST_APP/Contents/Info.plist"
cp "$TEMPLATE/Contents/MacOS/Install Minimix" "$DEST_APP/Contents/MacOS/Install Minimix"
chmod +x "$DEST_APP/Contents/MacOS/Install Minimix"

cp -R "$STAGE/Minimix.driver" "$DEST_APP/Contents/Resources/Minimix.driver"
cp "$STAGE/minimix-daemon" "$DEST_APP/Contents/Resources/minimix-daemon"
cp "$STAGE/minimix" "$DEST_APP/Contents/Resources/minimix"
cp "$STAGE/install.sh" "$DEST_APP/Contents/Resources/install.sh"
cp "$STAGE/NOTES.txt" "$DEST_APP/Contents/Resources/NOTES.txt"
chmod +x \
  "$DEST_APP/Contents/Resources/minimix-daemon" \
  "$DEST_APP/Contents/Resources/minimix" \
  "$DEST_APP/Contents/Resources/install.sh"

printf '%s\n' "$DEST_APP"
