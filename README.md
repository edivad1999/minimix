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
