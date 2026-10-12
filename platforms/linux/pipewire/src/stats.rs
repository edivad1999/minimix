//! Counters shared across capture, DSP, and source paths.

use std::sync::atomic::AtomicU64;

/// Underrun / overrun counters for ring hops.
#[derive(Debug, Default)]
pub struct RingStats {
    /// Capture callback could not push every sample.
    pub capture_overruns: AtomicU64,
    /// Source callback found an empty processed ring.
    pub source_underruns: AtomicU64,
    /// DSP blocks processed.
    pub dsp_blocks: AtomicU64,
    /// Peak DSP block duration in nanoseconds.
    pub max_dsp_ns: AtomicU64,
}
