//! Desktop entry: live monitor audio + egui parameter surface.

use minimix_audio_io::start_monitor;
use minimix_engine::Engine;
use std::sync::{Arc, Mutex};
use std::thread;

fn main() -> eframe::Result {
    let engine = Arc::new(Mutex::new(Engine::new(48_000.0)));

    let engine_audio = Arc::clone(&engine);
    let session = match start_monitor(move |buf| {
        if let Ok(mut eng) = engine_audio.lock() {
            eng.process(buf);
        }
    }) {
        Ok(s) => {
            eprintln!("monitor_started");
            Some(s)
        }
        Err(err) => {
            eprintln!("monitor failed (UI still opens): {err}");
            None
        }
    };

    let result = minimix_ui::run(Arc::clone(&engine));

    if let Some(session) = session {
        session.request_stop();
        thread::sleep(std::time::Duration::from_millis(50));
    }
    result
}
