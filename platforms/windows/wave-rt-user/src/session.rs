//! Capture → DSP → staging-ring commit session (Windows).

use crate::feed::{RingFeed, WaveRtError};
use minimix_engine::Engine;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// Live feed statistics.
#[derive(Debug, Default)]
pub struct FeedStats {
    /// Capture callbacks observed.
    pub callbacks: AtomicU64,
    /// Frames committed to the ring.
    pub frames_committed: AtomicU64,
    /// Max DSP+commit duration in nanoseconds.
    pub max_callback_ns: AtomicU64,
}

/// Running Windows feed. Dropping stops capture when a stream is held.
pub struct FeedSession {
    /// Shared stop flag.
    pub stop: Arc<AtomicBool>,
    /// Callback statistics.
    pub stats: Arc<FeedStats>,
    #[cfg(windows)]
    capture: minimix_audio_io::CaptureSession,
    #[cfg(not(windows))]
    _marker: std::marker::PhantomData<()>,
}

impl FeedSession {
    /// Signal capture to stop.
    pub fn request_stop(&self) {
        self.stop.store(true, Ordering::Release);
        #[cfg(windows)]
        self.capture.request_stop();
    }
}

/// Start capture on the default mic, process with `engine`, commit to `feed`.
///
/// On non-Windows hosts this returns [`WaveRtError::UnsupportedPlatform`].
///
/// # Errors
///
/// Returns [`WaveRtError`] when the platform cannot open capture or the feed.
#[cfg(windows)]
pub fn start_feed(
    engine: Arc<Mutex<Engine>>,
    feed: Arc<dyn RingFeed>,
) -> Result<FeedSession, WaveRtError> {
    use minimix_audio_io::start_capture;

    let stop = Arc::new(AtomicBool::new(false));
    let stats = Arc::new(FeedStats::default());
    let stats_cb = Arc::clone(&stats);
    let stop_cb = Arc::clone(&stop);

    let capture = start_capture(move |buf| {
        if stop_cb.load(Ordering::Acquire) {
            return;
        }
        let t0 = std::time::Instant::now();
        if let Ok(mut eng) = engine.lock() {
            eng.process(buf);
        }
        let _ = feed.commit(buf);
        let ns = u64::try_from(t0.elapsed().as_nanos()).unwrap_or(u64::MAX);
        stats_cb.callbacks.fetch_add(1, Ordering::Relaxed);
        stats_cb
            .frames_committed
            .fetch_add(buf.len() as u64, Ordering::Relaxed);
        let _ = stats_cb.max_callback_ns.fetch_max(ns, Ordering::Relaxed);
    })
    .map_err(|e| WaveRtError::Capture(e.to_string()))?;

    Ok(FeedSession {
        stop,
        stats,
        capture,
    })
}

/// Non-Windows stub.
///
/// # Errors
///
/// Always [`WaveRtError::UnsupportedPlatform`].
#[cfg(not(windows))]
pub fn start_feed(
    _engine: Arc<Mutex<Engine>>,
    _feed: Arc<dyn RingFeed>,
) -> Result<FeedSession, WaveRtError> {
    Err(WaveRtError::UnsupportedPlatform)
}
