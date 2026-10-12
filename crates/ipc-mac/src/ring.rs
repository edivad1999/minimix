//! Lock-free single-producer / single-consumer ring of `f32` samples.
//!
//! The HAL plug-in's mach receive thread pushes; `process_io` pops. Capacity is
//! fixed at construction so the realtime path never allocates.

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

struct Shared {
    buf: Box<[UnsafeCell<f32>]>,
    capacity: usize,
    head: AtomicUsize,
    tail: AtomicUsize,
}

// SAFETY: SPSC discipline — one producer writes `tail`, one consumer writes
// `head`, and loads use Acquire/Release.
unsafe impl Send for Shared {}
unsafe impl Sync for Shared {}

/// Producer half of a [`SampleRing`].
pub struct SampleProducer {
    shared: Arc<Shared>,
}

/// Consumer half of a [`SampleRing`].
pub struct SampleConsumer {
    shared: Arc<Shared>,
}

/// Fixed-capacity SPSC sample ring.
pub struct SampleRing;

impl SampleRing {
    /// Allocate a ring with `capacity` sample slots (must be non-zero).
    ///
    /// # Panics
    ///
    /// Panics when `capacity` is zero.
    #[must_use]
    pub fn channel(capacity: usize) -> (SampleProducer, SampleConsumer) {
        assert!(capacity > 0, "SampleRing capacity must be non-zero");
        let mut storage = Vec::with_capacity(capacity);
        storage.resize_with(capacity, || UnsafeCell::new(0.0));
        let shared = Arc::new(Shared {
            buf: storage.into_boxed_slice(),
            capacity,
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
        });
        (
            SampleProducer {
                shared: Arc::clone(&shared),
            },
            SampleConsumer { shared },
        )
    }
}

impl SampleProducer {
    /// Push one sample. Returns `false` when the ring is full.
    #[inline]
    #[must_use]
    pub fn try_push(&self, sample: f32) -> bool {
        let shared = &*self.shared;
        let tail = shared.tail.load(Ordering::Relaxed);
        let head = shared.head.load(Ordering::Acquire);
        if tail.wrapping_sub(head) == shared.capacity {
            return false;
        }
        let idx = tail % shared.capacity;
        // SAFETY: sole producer; slot exclusive until `tail` advances.
        unsafe {
            *shared.buf[idx].get() = sample;
        }
        shared.tail.store(tail.wrapping_add(1), Ordering::Release);
        true
    }

    /// Push as many samples as fit. Returns how many were written.
    #[must_use]
    pub fn push_slice(&self, samples: &[f32]) -> usize {
        let mut written = 0;
        for &sample in samples {
            if !self.try_push(sample) {
                break;
            }
            written += 1;
        }
        written
    }
}

impl SampleConsumer {
    /// Pop one sample. Returns `None` when the ring is empty.
    #[inline]
    #[must_use]
    pub fn try_pop(&self) -> Option<f32> {
        let shared = &*self.shared;
        let head = shared.head.load(Ordering::Relaxed);
        let tail = shared.tail.load(Ordering::Acquire);
        if head == tail {
            return None;
        }
        let idx = head % shared.capacity;
        // SAFETY: sole consumer; slot exclusive until `head` advances.
        let sample = unsafe { *shared.buf[idx].get() };
        shared.head.store(head.wrapping_add(1), Ordering::Release);
        Some(sample)
    }

    /// Fill `dst` from the ring; unread slots stay at `0.0`. Returns frames read.
    #[must_use]
    pub fn fill(&self, dst: &mut [f32]) -> usize {
        let mut n = 0;
        for slot in dst.iter_mut() {
            match self.try_pop() {
                Some(sample) => {
                    *slot = sample;
                    n += 1;
                }
                None => {
                    *slot = 0.0;
                }
            }
        }
        n
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_pop_preserves_order() {
        let (tx, rx) = SampleRing::channel(4);
        assert!(tx.try_push(1.0));
        assert!(tx.try_push(2.0));
        assert_eq!(rx.try_pop(), Some(1.0));
        assert_eq!(rx.try_pop(), Some(2.0));
        assert_eq!(rx.try_pop(), None);
    }

    #[test]
    fn full_ring_rejects_push() {
        let (tx, _rx) = SampleRing::channel(2);
        assert!(tx.try_push(0.1));
        assert!(tx.try_push(0.2));
        assert!(!tx.try_push(0.3));
    }

    #[test]
    fn fill_zero_pads_underrun() {
        let (tx, rx) = SampleRing::channel(8);
        assert_eq!(tx.push_slice(&[0.5, -0.5]), 2);
        let mut dst = [9.0_f32; 4];
        assert_eq!(rx.fill(&mut dst), 2);
        assert_eq!(dst, [0.5, -0.5, 0.0, 0.0]);
    }
}
