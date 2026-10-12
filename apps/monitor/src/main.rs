//! Live mic monitor. Captures, runs DSP, plays to the default output.

use minimix_audio_io::{list_input_devices, start_monitor};
use minimix_engine::Engine;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

fn main() {
    match list_input_devices() {
        Ok(devices) => {
            for (i, d) in devices.iter().enumerate() {
                eprintln!("input_device[{i}]={}", d.name);
            }
        }
        Err(err) => eprintln!("list_input_devices error: {err}"),
    }

    let engine = Arc::new(Mutex::new(Engine::new(48_000.0)));
    {
        let mut eng = engine.lock().expect("engine lock");
        eng.processor_mut()
            .equalizer_mut()
            .set_highpass(80.0, 0.707);
    }

    let engine_cb = Arc::clone(&engine);
    let session = match start_monitor(move |buf| {
        if let Ok(mut eng) = engine_cb.lock() {
            eng.process(buf);
        }
    }) {
        Ok(s) => s,
        Err(err) => {
            eprintln!("monitor failed: {err}");
            std::process::exit(1);
        }
    };

    // Demo: toggle bypass once so logs show both states without a GUI.
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

    let duration_s: u64 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(8);
    eprintln!("monitor_running_s={duration_s}");
    thread::sleep(Duration::from_secs(duration_s));

    session.request_stop();
    let callbacks = session.stats.callbacks.load(Ordering::Relaxed);
    let max_ns = session.stats.max_callback_ns.load(Ordering::Relaxed);
    let frames = session.stats.frames.load(Ordering::Relaxed);
    let underruns = session.stats.underruns.load(Ordering::Relaxed);
    eprintln!("monitor_stats callbacks={callbacks} frames={frames} max_callback_ns={max_ns} underruns={underruns}");
    eprintln!("monitor_stopped");
}
