# Minimix

Cross-platform virtual microphone with a shared Rust DSP engine.

Minimix captures from a real input device, applies real-time effects (EQ, compressor, gain, bypass), and exposes a system-selectable virtual microphone for apps such as Discord, Teams, Zoom, and OBS.

| Platform | Virtual device |
| -------- | -------------- |
| Linux | PipeWire `Audio/Source` |
| macOS | Core Audio AudioServerPlugIn |
| Windows | Minimal WaveRT capture driver |

No third-party virtual cables (BlackHole, VB-Cable, and similar) are required.

## Status

Early scaffold. DSP, platform adapters, GUI, and installers are not shipping yet. CI currently checks formatting, Clippy, and unit tests for the workspace crates that exist.

## License

MIT. See [LICENSE](LICENSE).

Windows driver sources derived from Microsoft samples, when added, will keep their MS-PL notices in that tree.
