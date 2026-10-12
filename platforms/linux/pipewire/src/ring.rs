//! Preallocated single-producer single-consumer sample ring.

use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

/// Fixed-capacity f32 ring. One producer thread, one consumer thread.
///
/// Usable capacity is `capacity - 1` so full and empty stay distinct.
/// Samples are stored as `f32` bit patterns in `AtomicU32` so the crate
/// stays `forbid(unsafe_code)`.
#[derive(Debug)]
pub struct SampleRing {
    buf: Box<[AtomicU32]>,
    mask: usize,
    write: AtomicUsize,
    read: AtomicUsize,
}

impl SampleRing {
    /// Allocate a zeroed ring. `min_capacity` is rounded up to a power of two.
    ///
    /// # Panics
    ///
    /// Panics when `min_capacity` is zero.
    #[must_use]
    pub fn with_capacity(min_capacity: usize) -> Self {
        assert!(min_capacity > 0, "ring capacity must be non-zero");
        let capacity = min_capacity.next_power_of_two().max(2);
        let mut storage = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            storage.push(AtomicU32::new(0));
        }
        Self {
            buf: storage.into_boxed_slice(),
            mask: capacity - 1,
            write: AtomicUsize::new(0),
            read: AtomicUsize::new(0),
        }
    }

    /// Storage length (power of two). Usable slots are one less.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.buf.len()
    }

    /// Samples waiting to be read.
    #[must_use]
    pub fn len(&self) -> usize {
        let w = self.write.load(Ordering::Acquire);
        let r = self.read.load(Ordering::Acquire);
        w.wrapping_sub(r)
    }

    /// True when no samples are buffered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Free slots available to the producer.
    #[must_use]
    pub fn free(&self) -> usize {
        self.capacity() - 1 - self.len()
    }

    /// Push samples. Returns how many were written (may be short on full).
    pub fn push_slice(&self, src: &[f32]) -> usize {
        let mut written = 0;
        while written < src.len() {
            let w = self.write.load(Ordering::Relaxed);
            let r = self.read.load(Ordering::Acquire);
            let filled = w.wrapping_sub(r);
            let free = self.capacity() - 1 - filled;
            if free == 0 {
                break;
            }
            let idx = w & self.mask;
            let contig = (self.capacity() - idx).min(free).min(src.len() - written);
            for i in 0..contig {
                self.buf[idx + i].store(src[written + i].to_bits(), Ordering::Relaxed);
            }
            self.write.store(w.wrapping_add(contig), Ordering::Release);
            written += contig;
        }
        written
    }

    /// Pop samples into `dst`. Returns how many were read (may be short on empty).
    pub fn pop_slice(&self, dst: &mut [f32]) -> usize {
        let mut read_n = 0;
        while read_n < dst.len() {
            let r = self.read.load(Ordering::Relaxed);
            let w = self.write.load(Ordering::Acquire);
            let available = w.wrapping_sub(r);
            if available == 0 {
                break;
            }
            let idx = r & self.mask;
            let contig = (self.capacity() - idx)
                .min(available)
                .min(dst.len() - read_n);
            for i in 0..contig {
                dst[read_n + i] = f32::from_bits(self.buf[idx + i].load(Ordering::Relaxed));
            }
            self.read.store(r.wrapping_add(contig), Ordering::Release);
            read_n += contig;
        }
        read_n
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_pop_round_trips_literal_samples() {
        let ring = SampleRing::with_capacity(8);
        let src = [0.1_f32, -0.2, 0.3, -0.4];
        assert_eq!(ring.push_slice(&src), 4);
        let mut dst = [0.0_f32; 4];
        assert_eq!(ring.pop_slice(&mut dst), 4);
        assert_eq!(dst, src);
    }

    #[test]
    fn full_ring_push_returns_short_count() {
        let ring = SampleRing::with_capacity(4);
        assert_eq!(ring.push_slice(&[1.0, 2.0, 3.0, 4.0]), 3);
        assert_eq!(ring.push_slice(&[5.0]), 0);
        let mut dst = [0.0_f32; 4];
        assert_eq!(ring.pop_slice(&mut dst), 3);
        assert_eq!(&dst[..3], &[1.0, 2.0, 3.0]);
    }

    #[test]
    fn empty_pop_returns_zero() {
        let ring = SampleRing::with_capacity(8);
        let mut dst = [9.0_f32; 2];
        assert_eq!(ring.pop_slice(&mut dst), 0);
        assert_eq!(dst, [9.0, 9.0]);
    }

    #[test]
    fn wraps_across_end_of_buffer() {
        let ring = SampleRing::with_capacity(4);
        assert_eq!(ring.push_slice(&[1.0, 2.0]), 2);
        let mut skip = [0.0_f32; 2];
        assert_eq!(ring.pop_slice(&mut skip), 2);
        assert_eq!(skip, [1.0, 2.0]);
        assert_eq!(ring.push_slice(&[3.0, 4.0, 5.0]), 3);
        let mut dst = [0.0_f32; 3];
        assert_eq!(ring.pop_slice(&mut dst), 3);
        assert_eq!(dst, [3.0, 4.0, 5.0]);
    }
}
