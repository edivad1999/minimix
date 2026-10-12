# macOS HAL virtual microphone

Minimix exposes a system input device through a thin Core Audio
`AudioServerPlugIn`. The plug-in does not run DSP. A userspace daemon
captures from a real mic, processes with `minimix-engine`, and pushes
PCM to the plug-in over the mach bootstrap service `com.minimix.audio`.

## Layout

| Path | Role |
| ---- | ---- |
| `platforms/macos/hal-driver/` | `tympan-aspl` cdylib + `.driver` bundle scripts |
| `crates/ipc-mac/` | Portable PCM framing + macOS mach transport |
| `apps/daemon/` | Capture → DSP → mach client (macOS) |

## Live install (Developer ID required)

1. Build the plug-in and daemon:

```sh
cargo build --release -p minimix-hal-driver -p minimix-daemon
```

2. Assemble the bundle:

```sh
./platforms/macos/hal-driver/scripts/assemble-driver.sh release
```

3. Sign with a **Developer ID Application** identity and install:

```sh
export MINIMIX_CODESIGN_IDENTITY="Developer ID Application: Your Name (TEAMID)"
./platforms/macos/hal-driver/scripts/sign-and-install.sh release
```

4. Confirm the device enumerates:

```sh
system_profiler SPAudioDataType | grep -i Minimix
```

5. Start the daemon (keeps feeding the plug-in while it runs):

```sh
./target/release/minimix-daemon
```

6. Select **Minimix** as the input device in Discord, Zoom, Teams, or OBS.

### Ad-hoc signing warning

`codesign --sign -` is enough for `coreaudiod` to *discover* the bundle.
On modern macOS the out-of-process Core Audio Driver Service helper still
rejects ad-hoc binaries with `AppleMobileFileIntegrityError -423` before
the device appears in System Settings. Do not claim a Discord-visible mic
without a Developer ID signed install.

## Mach service

The plug-in registers `com.minimix.audio` with `bootstrap_check_in` from a
non-realtime worker thread. The daemon calls `bootstrap_look_up` and sends
framed float32 packets (`crates/ipc-mac`). If the plug-in is not loaded,
the daemon exits with a bootstrap error. If the daemon is not running, the
plug-in serves silence.

This path does **not** use a home-directory mmap file.

## Panic policy

The plug-in crate is built with `panic = "abort"` in release so a panic
cannot unwind across the C ABI into `coreaudiod`.
