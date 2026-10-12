# minimix-wavert

Userspace framing for the Minimix WaveRT staging ring.

- `protocol` — private IOCTL codes and shared ring header (`MXRT`)
- `StagingRing` — commit PCM; consumer pull inserts silence on underrun
- `win-fake-ring` — in-process feed for `cargo test` without a loaded `.sys`
- `open_device_ring` — returns `DriverNotLoaded` until the WDK driver is installed

Live capture → DSP → commit runs from `apps/daemon` on Windows. Kernel timer and INF remain under `../wave-rt/` and are not built in CI.
