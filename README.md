# Minimix

[![CI](https://github.com/edivad1999/minimix/actions/workflows/ci.yml/badge.svg)](https://github.com/edivad1999/minimix/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

Cross-platform virtual microphone with a shared Rust DSP engine.

Minimix captures from a real input device, applies real-time effects (EQ, compressor, gain, bypass), and exposes a system-selectable virtual microphone for apps such as Discord, Teams, Zoom, and OBS.

| Platform | Virtual device |
| -------- | -------------- |
| Linux | PipeWire `Audio/Source` |
| macOS | Core Audio AudioServerPlugIn |
| Windows | Minimal WaveRT capture driver |

No third-party virtual cables (BlackHole, VB-Cable, and similar) are required.

## Install

Download a platform archive from
[GitHub Releases](https://github.com/edivad1999/minimix/releases).
Every push to `main` refreshes the **Continuous build** prerelease. Tag `v*`
for a numbered release.

On macOS, open the `.dmg` and double-click **Install Minimix**. On Linux and
Windows, unpack the archive and run `install.sh` or `install.ps1`.

| Platform | Guide |
| -------- | ----- |
| Linux | [docs/install-linux.md](docs/install-linux.md) |
| macOS | [docs/install-macos.md](docs/install-macos.md) |
| Windows | [docs/install-windows.md](docs/install-windows.md) |

Packaging layout and the PackageSpec registry are documented in
[packaging/README.md](packaging/README.md).

## Status

DSP, monitor, GUI, and platform scaffolds are in tree. The macOS HAL virtual
microphone (thin `AudioServerPlugIn` + mach-fed daemon) builds here; live load
needs a Developer ID signed install. See [docs/macos-hal.md](docs/macos-hal.md)
and [docs/install-macos.md](docs/install-macos.md).
CI checks formatting, Clippy, and tests on Linux, macOS, and Windows.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Please follow the [Code of Conduct](CODE_OF_CONDUCT.md).

Security reports use [SECURITY.md](SECURITY.md), not public issues.

## License

MIT. See [LICENSE](LICENSE).

Windows driver sources derived from Microsoft samples, when added, will keep their MS-PL notices in that tree.
