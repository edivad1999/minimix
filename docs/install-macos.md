# Install on macOS

## App (monitor + UI)

```bash
cargo run -p minimix
```

This opens the parameter UI and monitors the default input to the default output. It does not install a system-wide virtual microphone by itself.

## Virtual microphone (HAL plug-in)

Full steps, signing notes, and the mach service contract live in
[macos-hal.md](macos-hal.md). Short version:

1. `cargo build --release -p minimix-hal-driver -p minimix-daemon`
2. `./platforms/macos/hal-driver/scripts/assemble-driver.sh release`
3. Sign with **Developer ID Application** (`MINIMIX_CODESIGN_IDENTITY`) and run
   `./platforms/macos/hal-driver/scripts/sign-and-install.sh release`
4. Confirm Minimix in Sound Settings / `system_profiler SPAudioDataType`
5. Run `./target/release/minimix-daemon`

Ad-hoc signatures (`codesign --sign -`) often fail AMFI on modern macOS with
`AppleMobileFileIntegrityError -423`. A Discord-visible mic requires Developer ID.
