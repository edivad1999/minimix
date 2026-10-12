# Status

| Item | State |
| --- | --- |
| SimpleAudioSample fork | pending (needs Windows + WDK checkout) |
| Capture-only INF | pending |
| Passive-level IOCTL ring (kernel) | pending — no `.sys` yet |
| Userspace IOCTL framing + fake ring | landed in `platforms/windows/wave-rt-user` (`minimix-wavert`) |
| Daemon WaveRT feed path | landed in `apps/daemon` (Windows `cfg`; fake ring via `win-fake-ring`) |
| Test-sign package | pending |
| EV / attestation | **blocked** without Hardware Dev Center EV cert |
| Live mic → virtual mic verify | **blocked** on missing `.sys` + EV/attestation |

CI on this repository does not compile kernel drivers. Fake-ring unit tests cover commit and underrun silence. Live verify needs a Windows VM with a WDK-built, test-signed driver; production load needs EV + attestation.
