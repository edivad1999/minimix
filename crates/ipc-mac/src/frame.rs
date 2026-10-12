//! Little-endian PCM packet framing shared by the daemon and HAL plug-in.

use thiserror::Error;

/// ASCII `MXPC`.
pub const FRAME_MAGIC: u32 = 0x4D_58_50_43;
/// Current framing version.
pub const FRAME_VERSION: u16 = 1;
/// Fixed header size in bytes.
pub const HEADER_LEN: usize = 20;
/// Hard cap on samples per packet (keeps mach inline messages small).
pub const MAX_SAMPLES: usize = 1024;

/// Wire header that precedes interleaved little-endian `f32` samples.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PacketHeader {
    /// Must equal [`FRAME_MAGIC`].
    pub magic: u32,
    /// Must equal [`FRAME_VERSION`].
    pub version: u16,
    /// Interleaved channel count (1 = mono).
    pub channels: u16,
    /// Sample rate in Hz.
    pub sample_rate: u32,
    /// Number of `f32` samples following the header.
    pub sample_count: u32,
    /// Monotonic sequence from the producer.
    pub sequence: u32,
}

impl PacketHeader {
    /// Build a v1 header for `sample_count` floats.
    #[must_use]
    pub fn new(channels: u16, sample_rate: u32, sample_count: u32, sequence: u32) -> Self {
        Self {
            magic: FRAME_MAGIC,
            version: FRAME_VERSION,
            channels,
            sample_rate,
            sample_count,
            sequence,
        }
    }

    /// Serialize the header to little-endian bytes.
    #[must_use]
    pub fn to_bytes(self) -> [u8; HEADER_LEN] {
        let mut out = [0_u8; HEADER_LEN];
        out[0..4].copy_from_slice(&self.magic.to_le_bytes());
        out[4..6].copy_from_slice(&self.version.to_le_bytes());
        out[6..8].copy_from_slice(&self.channels.to_le_bytes());
        out[8..12].copy_from_slice(&self.sample_rate.to_le_bytes());
        out[12..16].copy_from_slice(&self.sample_count.to_le_bytes());
        out[16..20].copy_from_slice(&self.sequence.to_le_bytes());
        out
    }

    /// Parse a header from the first [`HEADER_LEN`] bytes.
    ///
    /// # Errors
    ///
    /// Returns [`FrameError`] when the buffer is short or fields are invalid.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, FrameError> {
        if bytes.len() < HEADER_LEN {
            return Err(FrameError::TruncatedHeader { got: bytes.len() });
        }
        let magic = u32::from_le_bytes(copy4(&bytes[0..4]));
        let version = u16::from_le_bytes(copy2(&bytes[4..6]));
        let channels = u16::from_le_bytes(copy2(&bytes[6..8]));
        let sample_rate = u32::from_le_bytes(copy4(&bytes[8..12]));
        let sample_count = u32::from_le_bytes(copy4(&bytes[12..16]));
        let sequence = u32::from_le_bytes(copy4(&bytes[16..20]));
        let header = Self {
            magic,
            version,
            channels,
            sample_rate,
            sample_count,
            sequence,
        };
        header.validate()?;
        Ok(header)
    }

    fn validate(self) -> Result<(), FrameError> {
        if self.magic != FRAME_MAGIC {
            return Err(FrameError::BadMagic { got: self.magic });
        }
        if self.version != FRAME_VERSION {
            return Err(FrameError::BadVersion { got: self.version });
        }
        if self.channels == 0 {
            return Err(FrameError::BadChannels);
        }
        if self.sample_rate == 0 {
            return Err(FrameError::BadSampleRate);
        }
        if self.sample_count as usize > MAX_SAMPLES {
            return Err(FrameError::TooManySamples {
                got: self.sample_count,
            });
        }
        if self.sample_count % u32::from(self.channels) != 0 {
            return Err(FrameError::MisalignedSampleCount {
                sample_count: self.sample_count,
                channels: self.channels,
            });
        }
        Ok(())
    }
}

/// Framing failures at the IPC boundary.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum FrameError {
    /// Header bytes missing.
    #[error("truncated packet header: got {got} bytes, need {HEADER_LEN}")]
    TruncatedHeader {
        /// Bytes available.
        got: usize,
    },
    /// Magic mismatch.
    #[error("bad frame magic: {got:#010x}")]
    BadMagic {
        /// Parsed magic.
        got: u32,
    },
    /// Unsupported version.
    #[error("unsupported frame version: {got}")]
    BadVersion {
        /// Parsed version.
        got: u16,
    },
    /// Zero channels.
    #[error("channels must be non-zero")]
    BadChannels,
    /// Zero sample rate.
    #[error("sample rate must be non-zero")]
    BadSampleRate,
    /// Payload longer than [`MAX_SAMPLES`].
    #[error("sample_count {got} exceeds MAX_SAMPLES ({MAX_SAMPLES})")]
    TooManySamples {
        /// Parsed count.
        got: u32,
    },
    /// `sample_count` not divisible by channel count.
    #[error("sample_count {sample_count} not divisible by channels {channels}")]
    MisalignedSampleCount {
        /// Parsed count.
        sample_count: u32,
        /// Parsed channels.
        channels: u16,
    },
    /// Payload shorter than `sample_count` floats.
    #[error("truncated sample payload: need {need} bytes, got {got}")]
    TruncatedPayload {
        /// Expected payload bytes.
        need: usize,
        /// Available payload bytes.
        got: usize,
    },
}

/// Encode `header` + `samples` into a contiguous packet buffer.
///
/// # Errors
///
/// Returns [`FrameError`] when the header is invalid or `samples.len()` mismatches.
pub fn encode_packet(header: PacketHeader, samples: &[f32]) -> Result<Vec<u8>, FrameError> {
    header.validate()?;
    if samples.len() != header.sample_count as usize {
        return Err(FrameError::TruncatedPayload {
            need: header.sample_count as usize * 4,
            got: samples.len() * 4,
        });
    }
    let mut out = Vec::with_capacity(HEADER_LEN + samples.len() * 4);
    out.extend_from_slice(&header.to_bytes());
    for sample in samples {
        out.extend_from_slice(&sample.to_le_bytes());
    }
    Ok(out)
}

/// Decode a packet into its header and sample vector.
///
/// # Errors
///
/// Returns [`FrameError`] on truncated or invalid frames.
pub fn decode_packet(bytes: &[u8]) -> Result<(PacketHeader, Vec<f32>), FrameError> {
    let header = PacketHeader::from_bytes(bytes)?;
    let need = header.sample_count as usize * 4;
    let payload = bytes.get(HEADER_LEN..).unwrap_or(&[]);
    if payload.len() < need {
        return Err(FrameError::TruncatedPayload {
            need,
            got: payload.len(),
        });
    }
    let mut samples = Vec::with_capacity(header.sample_count as usize);
    for chunk in payload[..need].chunks_exact(4) {
        samples.push(f32::from_le_bytes(copy4(chunk)));
    }
    Ok((header, samples))
}

fn copy2(bytes: &[u8]) -> [u8; 2] {
    let mut out = [0_u8; 2];
    out.copy_from_slice(bytes);
    out
}

fn copy4(bytes: &[u8]) -> [u8; 4] {
    let mut out = [0_u8; 4];
    out.copy_from_slice(bytes);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_mono_packet() {
        let samples = [0.0_f32, 0.5, -0.5, 1.0];
        let header = PacketHeader::new(1, 48_000, 4, 7);
        let bytes = encode_packet(header, &samples).unwrap();
        let (decoded, out) = decode_packet(&bytes).unwrap();
        assert_eq!(decoded, header);
        assert_eq!(out, samples);
    }

    #[test]
    fn rejects_bad_magic() {
        let mut bytes = encode_packet(PacketHeader::new(1, 48_000, 1, 0), &[0.25]).unwrap();
        bytes[0] = 0;
        assert!(matches!(
            decode_packet(&bytes),
            Err(FrameError::BadMagic { .. })
        ));
    }

    #[test]
    fn rejects_channel_misalign() {
        let err = PacketHeader::new(2, 48_000, 3, 0).validate();
        assert!(matches!(err, Err(FrameError::MisalignedSampleCount { .. })));
    }

    #[test]
    fn header_len_is_stable() {
        assert_eq!(
            HEADER_LEN,
            PacketHeader::new(1, 48_000, 0, 0).to_bytes().len()
        );
    }
}
