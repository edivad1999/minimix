# Install on Windows

## Test-signed driver (developers)

Until an EV certificate and Hardware Dev Center attestation are available, Minimix ships source and a test-signed WaveRT package only.

1. Enable test signing (may require Secure Boot off): `bcdedit /set testsigning on` then reboot.
2. Build the capture-only driver under `platforms/windows/wave-rt` with the WDK.
3. Install with `pnputil` / `devcon` using the INF from that tree.
4. Confirm a Minimix microphone appears in Sound Settings.
5. Run the Rust daemon to feed the driver's passive-level IOCTL ring.

## Production signing (blocked without EV)

Microsoft requires an EV certificate on the Hardware Dev Center account for attestation or WHCP signing. Without that, stock Windows 10/11 with Secure Boot will not load the `.sys`.

Attestation is for client testing scenarios. It is not retail Windows Update certification.
