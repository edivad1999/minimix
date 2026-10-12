//! `PipeWire` virtual microphone adapter for Minimix.
//!
//! On Linux this crate captures from a hardware source, runs [`minimix_engine::Engine`]
//! on a normal thread between preallocated rings, and publishes an `Audio/Source`
//! named Minimix. Other targets compile a stub that returns
//! [`PipewireError::UnsupportedPlatform`].

mod config;
mod error;
mod ring;
mod stats;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(not(target_os = "linux"))]
mod stub;

pub use config::VirtualMicConfig;
pub use error::PipewireError;
pub use ring::SampleRing;
pub use stats::RingStats;

#[cfg(target_os = "linux")]
pub use linux::{start_virtual_mic, VirtualMicSession};
#[cfg(not(target_os = "linux"))]
pub use stub::{start_virtual_mic, VirtualMicSession};
