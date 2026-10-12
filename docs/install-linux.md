# Install on Linux

## Requirements

- PipeWire 1.0.2 or newer
- WirePlumber
- `pipewire-pulse` (for Discord, Teams, stock OBS)

## From a GitHub Release (preferred)

1. Download `minimix-*-linux-x86_64.tar.gz` from
   [Releases](https://github.com/edivad1999/minimix/releases).
2. Unpack and run the install script:

```bash
tar -xzf minimix-*-linux-x86_64.tar.gz
cd minimix-*-linux-x86_64
./install.sh
```

Bins go to `~/.local/bin`. The script can install an optional systemd user unit
for `minimix-daemon`.

3. Select **Minimix** in Discord, Teams, or OBS. Keep `priority.session` low so
   it does not steal the default mic.

## Build from source (advanced)

```bash
sudo apt-get install -y libpipewire-0.3-dev pkg-config libasound2-dev
cargo build --release -p minimix-daemon -p minimix
./packaging/linux/pack.sh   # optional local archive under dist/
```

Or run the binaries from `target/release/` directly. Confirm the virtual
source with:

```bash
wpctl status
pactl list sources short
pw-record --target minimix-mic /tmp/minimix.wav
```
