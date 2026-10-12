# Packaging

Release archives are built from the **PackageSpec** registry in
[`package-spec.json`](package-spec.json). Pack scripts read that file. Do not
fork platform rules into scattered conditionals.

## PackageSpec

| Field | Meaning |
| ----- | ------- |
| `version` | From `MINIMIX_VERSION`, tag `v*` (`v0.1.0` → `0.1.0`), or `0.0.0-dev+<shortsha>` |
| `platform` | `linux-x86_64`, `macos-aarch64`, `macos-x86_64`, `windows-x86_64` |
| `components` | `daemon`, `gui`, optional `hal-driver`, plus `notes` |
| `signing` | `unsigned`, `developer-id-pending`, `ev-pending`, or `test-sign-pending` |
| `artifact_name` | `minimix-{version}-{platform}.tar.gz` or `.zip` |

## Build locally

```bash
# Linux (on Linux host)
./packaging/linux/pack.sh

# macOS (native arch)
./packaging/macos/pack.sh

# Windows (PowerShell)
./packaging/windows/pack.ps1
```

Archives land under `dist/`.

## Install from a release archive

Download the matching asset from GitHub Releases, unpack, and run the
embedded install script (`install.sh` or `install.ps1`).

## Follow-ups (out of this tree)

- Developer ID signing + notarization for macOS HAL
- Homebrew tap
- EV / attestation WaveRT installer (WiX/MSI) once a `.sys` exists
