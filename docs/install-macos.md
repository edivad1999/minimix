# Install on macOS

## App (monitor + UI)

```bash
cargo run -p minimix
```

This opens the parameter UI and monitors the default input to the default output. It does not yet install a system-wide virtual microphone.

## Virtual microphone (HAL plug-in)

A system input device requires an AudioServerPlugIn under `/Library/Audio/Plug-Ins/HAL/`.

1. Build the HAL driver crate from `platforms/macos/hal-driver` (see that PR when merged).
2. Assemble the `.driver` bundle.
3. Sign with **Developer ID Application** and the hardened runtime. Ad-hoc signatures fail AMFI on modern macOS (`AppleMobileFileIntegrityError -423`).
4. Copy the bundle to `/Library/Audio/Plug-Ins/HAL/` (root).
5. Restart Core Audio: `sudo killall -9 coreaudiod`.
6. Confirm Minimix appears in Sound Settings.
7. Run the daemon so the plug-in receives processed PCM over the declared mach service.

Notarize the installer if the package carries a quarantine flag.
