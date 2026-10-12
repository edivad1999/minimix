//! In-memory staging ring matching the `WaveRT` IOCTL commit/underrun rules.

use crate::protocol::RingHeader;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

/// Result of a consumer pull (driver timer equivalent).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PullResult {
    /// Frames copied from committed PCM.
    pub from_ring: usize,
    /// Frames filled with silence because the producer lagged.
    pub silence: usize,
}

impl PullResult {
    /// True when any silence was inserted.
    #[must_use]
    pub fn underran(&self) -> bool {
        self.silence > 0
    }
}

/// Single-producer staging ring. Consumer pull inserts silence on underrun.
#[derive(Debug)]
pub struct StagingRing {
    header: RingHeader,
    samples: Box<[AtomicU32]>,
    write: AtomicU64,
    read: AtomicU64,
    underrun_frames: AtomicU64,
}

impl StagingRing {
    /// Allocate a zeroed ring from a validated header.
    ///
    /// # Panics
    ///
    /// Panics when `header` fails [`RingHeader::is_valid`] or capacity is not
    /// a power of two.
    #[must_use]
    pub fn from_header(header: RingHeader) -> Self {
        assert!(header.is_valid(), "invalid ring header");
        let cap = header.frame_capacity as usize;
        assert!(cap.is_power_of_two(), "frame_capacity must be power of two");
        let mut storage = Vec::with_capacity(cap);
        for _ in 0..cap {
            storage.push(AtomicU32::new(0));
        }
        Self {
            header,
            samples: storage.into_boxed_slice(),
            write: AtomicU64::new(0),
            read: AtomicU64::new(0),
            underrun_frames: AtomicU64::new(0),
        }
    }

    /// Mono f32 ring at `sample_rate` with `frame_capacity` slots.
    #[must_use]
    pub fn mono_f32(sample_rate: u32, frame_capacity: u32) -> Self {
        Self::from_header(RingHeader::mono_f32(sample_rate, frame_capacity))
    }

    /// Shared header describing this ring.
    #[must_use]
    pub fn header(&self) -> &RingHeader {
        &self.header
    }

    /// Absolute producer cursor (frames committed).
    #[must_use]
    pub fn write_cursor(&self) -> u64 {
        self.write.load(Ordering::Acquire)
    }

    /// Absolute consumer cursor (frames consumed or silenced).
    #[must_use]
    pub fn read_cursor(&self) -> u64 {
        self.read.load(Ordering::Acquire)
    }

    /// Frames committed but not yet consumed.
    #[must_use]
    pub fn available(&self) -> u64 {
        self.write_cursor().saturating_sub(self.read_cursor())
    }

    /// Cumulative silence frames inserted on underrun.
    #[must_use]
    pub fn underrun_frames(&self) -> u64 {
        self.underrun_frames.load(Ordering::Acquire)
    }

    /// Producer write + commit. Drops oldest when the ring would overflow.
    pub fn commit(&self, src: &[f32]) -> usize {
        if src.is_empty() {
            return 0;
        }
        let cap = u64::from(self.header.frame_capacity);
        let mask = (self.header.frame_capacity - 1) as usize;
        let mut w = self.write.load(Ordering::Relaxed);
        let r = self.read.load(Ordering::Acquire);
        let filled = w.saturating_sub(r);
        if filled + src.len() as u64 > cap {
            let drop_n = filled + src.len() as u64 - cap;
            self.read.store(r.wrapping_add(drop_n), Ordering::Release);
        }
        for (i, sample) in src.iter().enumerate() {
            let idx = cursor_index(w, i, mask);
            self.samples[idx].store(sample.to_bits(), Ordering::Relaxed);
        }
        w = w.wrapping_add(src.len() as u64);
        self.write.store(w, Ordering::Release);
        src.len()
    }

    /// Consumer pull. Short commits yield silence for the remainder.
    pub fn pull(&self, dst: &mut [f32]) -> PullResult {
        if dst.is_empty() {
            return PullResult {
                from_ring: 0,
                silence: 0,
            };
        }
        let mask = (self.header.frame_capacity - 1) as usize;
        let mut r = self.read.load(Ordering::Relaxed);
        let w = self.write.load(Ordering::Acquire);
        let available = usize::try_from(w.saturating_sub(r)).unwrap_or(usize::MAX);
        let from_ring = available.min(dst.len());
        for (i, slot) in dst.iter_mut().take(from_ring).enumerate() {
            let idx = cursor_index(r, i, mask);
            *slot = f32::from_bits(self.samples[idx].load(Ordering::Relaxed));
        }
        let silence = dst.len() - from_ring;
        for slot in dst.iter_mut().skip(from_ring) {
            *slot = 0.0;
        }
        r = r.wrapping_add(dst.len() as u64);
        self.read.store(r, Ordering::Release);
        if silence > 0 {
            self.underrun_frames
                .fetch_add(silence as u64, Ordering::Relaxed);
        }
        PullResult { from_ring, silence }
    }
}

fn cursor_index(cursor: u64, offset: usize, mask: usize) -> usize {
    let mask_u64 = u64::try_from(mask).unwrap_or(u64::MAX);
    let base = usize::try_from(cursor & mask_u64).unwrap_or(0);
    base.wrapping_add(offset) & mask
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commit_then_pull_round_trips() {
        let ring = StagingRing::mono_f32(48_000, 16);
        let src = [0.1_f32, -0.2, 0.3];
        assert_eq!(ring.commit(&src), 3);
        let mut dst = [0.0_f32; 3];
        let pull = ring.pull(&mut dst);
        assert_eq!(
            pull,
            PullResult {
                from_ring: 3,
                silence: 0
            }
        );
        assert_eq!(dst, src);
        assert_eq!(ring.underrun_frames(), 0);
    }

    #[test]
    fn underrun_fills_silence_and_counts() {
        let ring = StagingRing::mono_f32(48_000, 8);
        assert_eq!(ring.commit(&[1.0, 2.0]), 2);
        let mut dst = [9.0_f32; 4];
        let pull = ring.pull(&mut dst);
        assert_eq!(pull.from_ring, 2);
        assert_eq!(pull.silence, 2);
        assert!(pull.underran());
        assert_eq!(dst, [1.0, 2.0, 0.0, 0.0]);
        assert_eq!(ring.underrun_frames(), 2);
    }

    #[test]
    fn empty_pull_is_all_silence() {
        let ring = StagingRing::mono_f32(48_000, 8);
        let mut dst = [1.0_f32; 3];
        let pull = ring.pull(&mut dst);
        assert_eq!(
            pull,
            PullResult {
                from_ring: 0,
                silence: 3
            }
        );
        assert_eq!(dst, [0.0, 0.0, 0.0]);
    }
}
