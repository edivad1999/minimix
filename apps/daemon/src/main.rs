//! Capture → DSP → mach client that feeds the Minimix HAL plug-in.

fn main() {
    #[cfg(not(target_os = "macos"))]
    {
        eprintln!("minimix-daemon is macOS-only (mach service client)");
    }

    #[cfg(target_os = "macos")]
    macos::run();
}

#[cfg(target_os = "macos")]
mod macos {
    use minimix_audio_io::{list_input_devices, start_capture};
    use minimix_engine::Engine;
    use minimix_ipc_mac::frame::PacketHeader;
    use minimix_ipc_mac::mach::DaemonClient;
    use minimix_ipc_mac::SERVICE_NAME;
    use std::sync::atomic::Ordering;
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Duration;

    pub fn run() {
        match list_input_devices() {
            Ok(devices) => {
                for (i, d) in devices.iter().enumerate() {
                    eprintln!("input_device[{i}]={}", d.name);
                }
            }
            Err(err) => eprintln!("list_input_devices error: {err}"),
        }

        let client = match DaemonClient::connect() {
            Ok(client) => Arc::new(Mutex::new(client)),
            Err(err) => {
                eprintln!(
                    "mach look_up failed ({err}). Is Minimix.driver installed and coreaudiod loaded?"
                );
                std::process::exit(1);
            }
        };

        let engine = Arc::new(Mutex::new(Engine::new(48_000.0)));
        {
            let mut eng = engine.lock().expect("engine lock");
            eng.processor_mut()
                .equalizer_mut()
                .set_highpass(80.0, 0.707);
        }

        let engine_cb = Arc::clone(&engine);
        let client_cb = Arc::clone(&client);
        let seq = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let seq_cb = Arc::clone(&seq);

        let session = match start_capture(move |buf| {
            if let Ok(mut eng) = engine_cb.lock() {
                eng.process(buf);
            }
            if let Ok(client) = client_cb.lock() {
                for chunk in buf.chunks(minimix_ipc_mac::frame::MAX_SAMPLES) {
                    let Ok(count) = u32::try_from(chunk.len()) else {
                        continue;
                    };
                    let sequence = seq_cb.fetch_add(1, Ordering::Relaxed);
                    let header = PacketHeader::new(1, 48_000, count, sequence);
                    if let Err(err) = client.send_packet(header, chunk) {
                        eprintln!("mach send error: {err}");
                    }
                }
            }
        }) {
            Ok(s) => s,
            Err(err) => {
                eprintln!("capture failed: {err}");
                std::process::exit(1);
            }
        };

        let duration_s: u64 = std::env::args()
            .nth(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        eprintln!(
            "daemon_running service={SERVICE_NAME} sample_rate={}",
            session.sample_rate
        );
        if duration_s == 0 {
            loop {
                thread::sleep(Duration::from_secs(3600));
            }
        }

        thread::sleep(Duration::from_secs(duration_s));
        session.request_stop();
        let callbacks = session.stats.callbacks.load(Ordering::Relaxed);
        let frames = session.stats.frames.load(Ordering::Relaxed);
        eprintln!("daemon_stats callbacks={callbacks} frames={frames}");
    }
}
