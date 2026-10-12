# Minimix WaveRT capture driver (scaffold)

This tree will hold a **capture-only** PortCls WaveRT virtual microphone derived from Microsoft's [SimpleAudioSample](https://github.com/microsoft/Windows-driver-samples/tree/main/audio/simpleaudiosample), not the full SysVAD solution.

## Design

- Root-enumerated software device + INF
- One capture bridge pin published as a microphone by AudioEndpointBuilder
- Driver timer owns `GetPosition` and never waits on user mode
- Passive-level private IOCTL fills a staging ring; underrun plays silence
- All DSP stays in the Rust daemon (see `minimix-06`)

## License

Microsoft sample derivatives must keep **MS-PL** notices. When sources are vendored here, add `LICENSE-MS-PL` beside them and do not relicense that subtree as MIT.

## Build (Windows + WDK)

Not runnable from macOS/Linux CI. On a Windows machine with the WDK:

1. Open the driver solution under this directory (added in a follow-up commit once the SimpleAudioSample fork lands).
2. Build the Release x64 package.
3. Test-sign and install with Secure Boot off / testsigning on.

## Status

Scaffold only. No `.sys` is built in this PR. The INF and C++ miniport land next once the MS-PL tree is copied with notices.
