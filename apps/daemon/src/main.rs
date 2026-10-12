//! Capture → DSP → platform virtual-mic feed (mach on macOS, `WaveRT` on Windows).

fn main() {
    #[cfg(target_os = "macos")]
    macos::run();

    #[cfg(target_os = "windows")]
    {
        if let Err(code) = windows::run() {
            std::process::exit(code);
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        eprintln!("minimix-daemon unsupported on this platform");
    }
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

#[cfg(target_os = "windows")]
mod windows {
    use minimix_engine::Engine;
    use minimix_wavert::{open_device_ring, open_fake_ring, start_feed, RingFeed};
    use std::sync::{Arc, Mutex};

    pub fn run() -> Result<(), i32> {
        use std::env;
        use std::sync::atomic::Ordering;
        use std::thread;
        use std::time::Duration;

        let use_fake = env::var_os("MINIMIX_FAKE_RING").is_some();
        let feed: Arc<dyn RingFeed> = if use_fake {
            match open_fake_ring(48_000, 4096) {
                Ok(f) => {
                    eprintln!("daemon_ring=fake");
                    Arc::new(f)
                }
                Err(err) => {
                    eprintln!("daemon_failed error={err}");
                    return Err(1);
                }
            }
        } else {
            match open_device_ring() {
                Ok(f) => {
                    eprintln!("daemon_ring=device");
                    Arc::new(f)
                }
                Err(err) => {
                    eprintln!("daemon_failed error={err}");
                    eprintln!(
                        "hint: set MINIMIX_FAKE_RING=1 for in-process ring (requires win-fake-ring feature)"
                    );
                    return Err(1);
                }
            }
        };

        let engine = Arc::new(Mutex::new(Engine::new(48_000.0)));
        {
            let mut eng = engine.lock().expect("engine lock");
            eng.processor_mut()
                .equalizer_mut()
                .set_highpass(80.0, 0.707);
        }

        let session = match start_feed(Arc::clone(&engine), feed) {
            Ok(s) => s,
            Err(err) => {
                eprintln!("daemon_failed error={err}");
                return Err(1);
            }
        };
        eprintln!("daemon_started");

        thread::sleep(Duration::from_secs(2));
        if let Ok(eng) = engine.lock() {
            eng.params().bypass.store(true, Ordering::Relaxed);
            eprintln!("bypass=on");
        }
        thread::sleep(Duration::from_secs(2));
        if let Ok(eng) = engine.lock() {
            eng.params().bypass.store(false, Ordering::Relaxed);
            eprintln!("bypass=off");
        }

        let duration_s: u64 = env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(0);
        if duration_s > 0 {
            eprintln!("daemon_running_s={duration_s}");
            thread::sleep(Duration::from_secs(duration_s));
            session.request_stop();
        } else {
            eprintln!("daemon_running (kill to stop)");
            loop {
                thread::sleep(Duration::from_secs(3600));
            }
        }

        eprintln!(
            "daemon_stats callbacks={} frames={} max_callback_ns={}",
            session.stats.callbacks.load(Ordering::Relaxed),
            session.stats.frames_committed.load(Ordering::Relaxed),
            session.stats.max_callback_ns.load(Ordering::Relaxed)
        );
        eprintln!("daemon_stopped");
        Ok(())
    }
}
