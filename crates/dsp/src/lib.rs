//! Real-time DSP for Minimix.
//!
//! Process paths must stay allocation-free. Parameter updates arrive through
//! atomics or a preallocated control channel owned by callers.

mod biquad;
mod compressor;
mod equalizer;
mod processor;

pub use compressor::Compressor;
pub use equalizer::{Equalizer, PEAK_BANDS};
pub use processor::AudioProcessor;

/// Marker for effects that may run inside an audio callback.
pub trait Effect: Send {
    /// Process `buffer` in place.
    fn process(&mut self, buffer: &mut [f32]);

    /// Clear filter memory and envelopes.
    fn reset(&mut self);
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_no_alloc::assert_no_alloc;

    #[test]
    fn process_path_does_not_allocate() {
        let mut proc = AudioProcessor::new(48_000.0);
        let mut buffer = [0.1_f32; 256];
        assert_no_alloc(|| {
            proc.process(&mut buffer);
        });
    }

    #[test]
    fn bypass_leaves_samples_unchanged() {
        let mut proc = AudioProcessor::new(48_000.0);
        proc.equalizer_mut().set_peak(1, 1000.0, 1.0, 12.0);
        proc.bypass = true;
        let mut buffer = [0.5_f32, -0.25, 0.125];
        let before = buffer;
        proc.process(&mut buffer);
        assert_eq!(buffer, before);
    }

    #[test]
    fn reset_clears_filter_memory() {
        let mut a = AudioProcessor::new(48_000.0);
        a.equalizer_mut().set_highpass(200.0, 0.707);
        let mut impulse = [0.0_f32; 64];
        impulse[0] = 1.0;
        a.process(&mut impulse);

        a.reset();
        let mut b = AudioProcessor::new(48_000.0);
        b.equalizer_mut().set_highpass(200.0, 0.707);
        let mut again = [0.0_f32; 64];
        again[0] = 1.0;
        let mut fresh = [0.0_f32; 64];
        fresh[0] = 1.0;
        a.process(&mut again);
        b.process(&mut fresh);
        for (x, y) in again.iter().zip(fresh.iter()) {
            assert!((x - y).abs() < 1.0e-6);
        }
    }
}
