//! Non-Linux stub. Compiles the public API without linking libpipewire.

use crate::config::VirtualMicConfig;
use crate::error::PipewireError;
use crate::stats::RingStats;
use minimix_engine::Engine;
use std::sync::{Arc, Mutex};

/// Placeholder session on platforms without `PipeWire`.
#[derive(Debug)]
pub struct VirtualMicSession {
    _private: (),
}

impl VirtualMicSession {
    /// Always errors on this target.
    pub fn request_stop(&self) {}

    /// Stub never becomes ready.
    #[must_use]
    pub fn is_ready(&self) -> bool {
        false
    }

    /// Empty stats for API parity.
    #[must_use]
    pub fn stats(&self) -> Arc<RingStats> {
        Arc::new(RingStats::default())
    }
}

/// Refuse to start. Real streams exist only on Linux.
///
/// # Errors
///
/// Always returns [`PipewireError::UnsupportedPlatform`].
pub fn start_virtual_mic(
    _config: VirtualMicConfig,
    _engine: Arc<Mutex<Engine>>,
) -> Result<VirtualMicSession, PipewireError> {
    Err(PipewireError::UnsupportedPlatform)
}
