# Install on Windows

## From a GitHub Release (preferred)

1. Download `minimix-*-windows-x86_64.zip` from
   [Releases](https://github.com/edivad1999/minimix/releases).
2. Unpack and run the install script:

```powershell
Expand-Archive .\minimix-*-windows-x86_64.zip
cd .\minimix-*-windows-x86_64
.\install.ps1
```

This installs `minimix.exe` and `minimix-daemon.exe` under
`%LOCALAPPDATA%\Minimix\bin`, adds that folder to the user PATH, and can create
a Start Menu shortcut.

### Virtual microphone status

The release zip is **userspace only**. It does **not** install a WaveRT `.sys`.
Production kernel load needs an EV certificate and Hardware Dev Center
attestation. Until then, a live system mic stays test-sign / EV-blocked. See
`NOTES.txt` in the archive and
[platforms/windows/wave-rt/STATUS.md](../platforms/windows/wave-rt/STATUS.md).

## Test-signed driver (developers)

1. Enable test signing (may require Secure Boot off): `bcdedit /set testsigning on`
   then reboot.
2. Build the capture-only driver under `platforms/windows/wave-rt` with the WDK.
3. Install with `pnputil` / `devcon` using the INF from that tree.
4. Confirm a Minimix microphone appears in Sound Settings.
5. Run `minimix-daemon` to feed the driver's passive-level IOCTL ring.

## Build from source (advanced)

```powershell
cargo build --release -p minimix-daemon -p minimix
.\packaging\windows\pack.ps1
```
