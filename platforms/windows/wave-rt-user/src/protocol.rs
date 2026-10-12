//! Framing for the Minimix `WaveRT` private IOCTL staging ring.
//!
//! The kernel timer (passive-level completion of these IOCTLs) copies
//! committed frames into the `WaveRT` cyclic buffer. When the producer has
//! not committed enough frames, the timer writes silence and advances.

/// ASCII `MXRT` identifies a Minimix `WaveRT` staging mapping.
pub const RING_MAGIC: u32 = u32::from_le_bytes(*b"MXRT");

/// Protocol revision carried in [`RingHeader::version`].
pub const RING_VERSION: u32 = 1;

/// Mono f32 interleaved samples (daemon DSP output).
pub const SAMPLE_FORMAT_F32_MONO: u16 = 1;

/// Custom device type for `CTL_CODE` (matches the planned INF private interface).
pub const FILE_DEVICE_MINIMIX: u32 = 0x0000_9D41;

const fn ctl_code(device_type: u32, function: u32, method: u32, access: u32) -> u32 {
    (device_type << 16) | (access << 14) | (function << 2) | method
}

const METHOD_BUFFERED: u32 = 0;
const FILE_READ_DATA: u32 = 0x0001;
const FILE_WRITE_DATA: u32 = 0x0002;

/// Open / register the staging ring with the driver (buffered METHOD).
///
/// Input: [`RingOpenRequest`]. Output: [`RingOpenResponse`].
pub const IOCTL_MINIMIX_RING_OPEN: u32 = ctl_code(
    FILE_DEVICE_MINIMIX,
    0x800,
    METHOD_BUFFERED,
    FILE_READ_DATA | FILE_WRITE_DATA,
);

/// Producer commit: advance the write cursor after filling PCM.
///
/// Input: [`RingCommitRequest`]. No output buffer.
pub const IOCTL_MINIMIX_RING_COMMIT: u32 =
    ctl_code(FILE_DEVICE_MINIMIX, 0x801, METHOD_BUFFERED, FILE_WRITE_DATA);

/// Query cursors and capacity (debug / feed pacing).
///
/// Output: [`RingQueryResponse`].
pub const IOCTL_MINIMIX_RING_QUERY: u32 =
    ctl_code(FILE_DEVICE_MINIMIX, 0x802, METHOD_BUFFERED, FILE_READ_DATA);

/// Shared header at the start of the staging mapping (user and kernel agree).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RingHeader {
    /// Must be [`RING_MAGIC`].
    pub magic: u32,
    /// Must be [`RING_VERSION`].
    pub version: u32,
    /// Sample rate in Hz (typically 48000).
    pub sample_rate: u32,
    /// Channel count (Minimix capture pin is mono).
    pub channels: u16,
    /// Wire format tag ([`SAMPLE_FORMAT_F32_MONO`]).
    pub sample_format: u16,
    /// Capacity in frames (power of two recommended).
    pub frame_capacity: u32,
    /// Bytes from mapping start to first PCM sample.
    pub data_offset: u32,
    /// Reserved for alignment / future flags.
    pub reserved: u32,
    /// Pad to 32 bytes so kernel and user agree on layout without packing surprises.
    pub pad: u32,
}

impl RingHeader {
    /// Header size used as the default [`RingHeader::data_offset`].
    pub const SIZE: u32 = 32;

    /// Build a mono f32 header for `frame_capacity` frames.
    #[must_use]
    pub fn mono_f32(sample_rate: u32, frame_capacity: u32) -> Self {
        Self {
            magic: RING_MAGIC,
            version: RING_VERSION,
            sample_rate,
            channels: 1,
            sample_format: SAMPLE_FORMAT_F32_MONO,
            frame_capacity,
            data_offset: Self::SIZE,
            reserved: 0,
            pad: 0,
        }
    }

    /// Validate magic, version, and format before trusting cursors.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.magic == RING_MAGIC
            && self.version == RING_VERSION
            && self.channels == 1
            && self.sample_format == SAMPLE_FORMAT_F32_MONO
            && self.frame_capacity > 0
            && self.data_offset >= Self::SIZE
    }
}

/// Input for [`IOCTL_MINIMIX_RING_OPEN`].
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RingOpenRequest {
    pub sample_rate: u32,
    pub frame_capacity: u32,
}

/// Output for [`IOCTL_MINIMIX_RING_OPEN`].
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RingOpenResponse {
    pub header: RingHeader,
    /// Kernel-assigned ring id for subsequent commit/query IOCTLs.
    pub ring_id: u64,
}

/// Input for [`IOCTL_MINIMIX_RING_COMMIT`].
///
/// Producer has written `frame_count` frames starting at `write_cursor`
/// into the mapped PCM region, then issues this IOCTL so the timer may
/// observe them. Frames past capacity wrap modulo `frame_capacity`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RingCommitRequest {
    pub ring_id: u64,
    /// Absolute frame index of the first newly written frame.
    pub write_cursor: u64,
    /// Number of frames committed in this call.
    pub frame_count: u32,
}

/// Output for [`IOCTL_MINIMIX_RING_QUERY`].
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RingQueryResponse {
    pub ring_id: u64,
    pub write_cursor: u64,
    pub read_cursor: u64,
    pub frame_capacity: u32,
    /// Frames the timer filled with silence since open.
    pub underrun_frames: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_size_matches_repr() {
        assert_eq!(std::mem::size_of::<RingHeader>(), RingHeader::SIZE as usize);
    }

    #[test]
    fn mono_header_validates() {
        let h = RingHeader::mono_f32(48_000, 4096);
        assert!(h.is_valid());
    }

    #[test]
    fn ioctl_codes_are_distinct() {
        assert_ne!(IOCTL_MINIMIX_RING_OPEN, IOCTL_MINIMIX_RING_COMMIT);
        assert_ne!(IOCTL_MINIMIX_RING_COMMIT, IOCTL_MINIMIX_RING_QUERY);
    }
}
