# Install on macOS

## From a GitHub Release (preferred)

1. Download the matching **DMG** from
   [Releases](https://github.com/edivad1999/minimix/releases):
   `minimix-*-macos-aarch64.dmg` (Apple silicon) or
   `minimix-*-macos-x86_64.dmg` (Intel).
2. Open the DMG and double-click **Install Minimix**.
3. Enter your Mac password when prompted.
4. If macOS blocks the app (unsigned build), right-click
   **Install Minimix** → **Open** → **Open**.

This copies `Minimix.driver` to `/Library/Audio/Plug-Ins/HAL`, restarts
`coreaudiod`, and installs `minimix-daemon` / `minimix` under `/usr/local/bin`.

5. Start the daemon, then open the GUI if you want the parameter UI:

```bash
minimix-daemon
minimix
```

### Script install (tar.gz)

Releases also ship `minimix-*-macos-*.tar.gz` for automation:

```bash
tar -xzf minimix-*-macos-aarch64.tar.gz
cd minimix-*-macos-aarch64
./install.sh
```

### Signing limit

Release builds are **unsigned**. Live Discord / Sound Settings input needs a
**Developer ID Application** signed plug-in. Ad-hoc signatures often fail AMFI
on modern macOS with `AppleMobileFileIntegrityError -423`. Full signing and
mach-service notes live in [macos-hal.md](macos-hal.md).

## Build from source (advanced)

```bash
cargo build --release -p minimix-hal-driver -p minimix-daemon -p minimix
./platforms/macos/hal-driver/scripts/assemble-driver.sh release
./packaging/macos/pack.sh   # writes .tar.gz and .dmg under dist/
```

To sign and install with Developer ID:

```bash
export MINIMIX_CODESIGN_IDENTITY="Developer ID Application: Your Name (TEAMID)"
./platforms/macos/hal-driver/scripts/sign-and-install.sh release
./target/release/minimix-daemon
```

Confirm Minimix in Sound Settings or `system_profiler SPAudioDataType`.
