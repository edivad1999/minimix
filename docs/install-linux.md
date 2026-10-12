# Install on Linux

## Requirements

- PipeWire 1.0.2 or newer
- WirePlumber
- `pipewire-pulse` (for Discord, Teams, stock OBS)

## Build

```bash
sudo apt-get install -y libpipewire-0.3-dev pkg-config
cargo build -p minimix-pipewire --release
```

## Run the virtual mic

Start the PipeWire source binary from the Linux PR (node name `minimix-mic`). Then:

```bash
wpctl status
pactl list sources short
pw-record --target minimix-mic /tmp/minimix.wav
```

Select **Minimix** in Discord, Teams, or OBS. Keep `priority.session` low so it does not steal the default mic.
