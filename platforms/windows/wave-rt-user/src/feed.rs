//! Trait for submitting processed PCM into a `WaveRT` staging ring.

use crate::protocol::{RingHeader, RingQueryResponse};
use crate::ring::StagingRing;
use thiserror::Error;

/// Errors opening or writing a `WaveRT` feed target.
#[derive(Debug, Error)]
pub enum WaveRtError {
    /// No loaded Minimix `WaveRT` `.sys` (WDK build / install / EV still pending).
    #[error("WaveRT driver not loaded: {0}")]
    DriverNotLoaded(&'static str),
    /// Capture device open or stream failed.
    #[error("capture error: {0}")]
    Capture(String),
    /// Platform has no `WaveRT` path (non-Windows builds).
    #[error("WaveRT feed unsupported on this platform")]
    UnsupportedPlatform,
    /// Fake ring feature was required but not enabled.
    #[error("win-fake-ring feature required for in-process staging ring")]
    FakeRingDisabled,
}

/// Userspace producer face of the staging ring.
pub trait RingFeed: Send + Sync {
    /// Header negotiated at open.
    fn header(&self) -> &RingHeader;

    /// Commit processed mono f32 frames.
    ///
    /// # Errors
    ///
    /// Returns [`WaveRtError`] when the feed cannot accept frames.
    fn commit(&self, frames: &[f32]) -> Result<usize, WaveRtError>;

    /// Snapshot cursors (fake ring and query IOCTL).
    fn query(&self) -> RingQueryResponse;
}

/// Feed backed by an in-process [`StagingRing`] (CI / no `.sys`).
#[derive(Debug)]
pub struct FakeRingFeed {
    ring_id: u64,
    ring: StagingRing,
}

impl FakeRingFeed {
    /// Open a mono f32 fake ring.
    #[must_use]
    pub fn open(sample_rate: u32, frame_capacity: u32) -> Self {
        Self {
            ring_id: 1,
            ring: StagingRing::mono_f32(sample_rate, frame_capacity),
        }
    }

    /// Borrow the underlying ring (tests pull as the driver timer).
    #[must_use]
    pub fn ring(&self) -> &StagingRing {
        &self.ring
    }
}

impl RingFeed for FakeRingFeed {
    fn header(&self) -> &RingHeader {
        self.ring.header()
    }

    fn commit(&self, frames: &[f32]) -> Result<usize, WaveRtError> {
        Ok(self.ring.commit(frames))
    }

    fn query(&self) -> RingQueryResponse {
        RingQueryResponse {
            ring_id: self.ring_id,
            write_cursor: self.ring.write_cursor(),
            read_cursor: self.ring.read_cursor(),
            frame_capacity: self.ring.header().frame_capacity,
            underrun_frames: self.ring.underrun_frames(),
        }
    }
}

/// Attempt to open the real kernel device. Always fails until a `.sys` lands.
///
/// # Errors
///
/// Returns [`WaveRtError::DriverNotLoaded`] because this tree has no WDK
/// driver binary and live `DeviceIoControl` needs that install.
pub fn open_device_ring() -> Result<FakeRingFeed, WaveRtError> {
    Err(WaveRtError::DriverNotLoaded(
        "no Minimix WaveRT .sys installed; passive IOCTL ring needs WDK build, test-sign, and EV/attestation for production",
    ))
}

/// Open the in-process fake ring when the `win-fake-ring` feature is enabled.
///
/// # Errors
///
/// Returns [`WaveRtError::FakeRingDisabled`] without the feature.
#[cfg(feature = "win-fake-ring")]
pub fn open_fake_ring(sample_rate: u32, frame_capacity: u32) -> Result<FakeRingFeed, WaveRtError> {
    Ok(FakeRingFeed::open(sample_rate, frame_capacity))
}

/// Open the in-process fake ring when the `win-fake-ring` feature is enabled.
///
/// # Errors
///
/// Returns [`WaveRtError::FakeRingDisabled`] without the feature.
#[cfg(not(feature = "win-fake-ring"))]
pub fn open_fake_ring(
    _sample_rate: u32,
    _frame_capacity: u32,
) -> Result<FakeRingFeed, WaveRtError> {
    Err(WaveRtError::FakeRingDisabled)
}
