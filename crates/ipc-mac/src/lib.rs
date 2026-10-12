//! Mach-service PCM transport for the Minimix macOS HAL path.
//!
//! Portable pieces ([`frame`], [`ring`]) build and unit-test on every host.
//! The bootstrap / `mach_msg` transport lives behind `cfg(target_os = "macos")`.

#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

pub mod frame;
pub mod ring;

#[cfg(target_os = "macos")]
pub mod mach;

pub use frame::{decode_packet, encode_packet, PacketHeader, FRAME_MAGIC, FRAME_VERSION};
pub use ring::{SampleConsumer, SampleProducer, SampleRing};

/// Bootstrap / launchd mach service name shared by the daemon and HAL plug-in.
pub const SERVICE_NAME: &str = "com.minimix.audio";
