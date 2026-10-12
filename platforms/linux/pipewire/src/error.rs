//! Errors from the `PipeWire` virtual-mic adapter.

/// Failures starting or running the virtual mic.
#[derive(Debug, thiserror::Error)]
pub enum PipewireError {
    /// This build target has no `PipeWire` backend.
    #[error("PipeWire virtual mic is only available on Linux")]
    UnsupportedPlatform,
    /// `PipeWire` client or stream setup failed.
    #[error("pipewire error: {0}")]
    Pipewire(String),
    /// Background session thread stopped unexpectedly.
    #[error("virtual mic session ended: {0}")]
    Session(String),
}
