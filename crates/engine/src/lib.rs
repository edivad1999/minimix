//! Realtime engine. Owns the DSP graph and lock-free parameter slots.

use minimix_dsp::AudioProcessor;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

/// Shared parameter block safe to poke from the UI thread.
#[derive(Debug, Default)]
pub struct ParamBlock {
    /// Bypass the DSP chain.
    pub bypass: AtomicBool,
    /// Output gain as Q16 fixed point (`1.0 == 65536`).
    pub output_gain_q16: AtomicU32,
}

impl ParamBlock {
    /// Unity gain, bypass off.
    #[must_use]
    pub fn new() -> Self {
        Self {
            bypass: AtomicBool::new(false),
            output_gain_q16: AtomicU32::new(65536),
        }
    }
}

/// Runs `AudioProcessor` and samples `ParamBlock` once per buffer.
#[derive(Debug)]
pub struct Engine {
    processor: AudioProcessor,
    params: ParamBlock,
}

impl Engine {
    /// Build at `sample_rate`.
    #[must_use]
    pub fn new(sample_rate: f32) -> Self {
        Self {
            processor: AudioProcessor::new(sample_rate),
            params: ParamBlock::new(),
        }
    }

    /// Borrow the param block for external writers.
    #[must_use]
    pub fn params(&self) -> &ParamBlock {
        &self.params
    }

    /// Access the processor for one-time setup (EQ bands, compressor).
    pub fn processor_mut(&mut self) -> &mut AudioProcessor {
        &mut self.processor
    }

    /// Apply atomics then process `buffer` in place. No locks, no alloc.
    pub fn process(&mut self, buffer: &mut [f32]) {
        let bypass = self.params.bypass.load(Ordering::Relaxed);
        let gain_q16 = self.params.output_gain_q16.load(Ordering::Relaxed);
        self.processor.bypass = bypass;
        #[allow(clippy::cast_precision_loss)]
        let gain = gain_q16 as f32 / 65536.0;
        self.processor.set_output_gain(gain);
        self.processor.process(buffer);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_bypass_skips_processing() {
        let mut engine = Engine::new(48_000.0);
        engine.params().bypass.store(true, Ordering::Relaxed);
        engine
            .processor_mut()
            .equalizer_mut()
            .set_peak(1, 1000.0, 1.0, 12.0);
        let mut buf = [0.5_f32, -0.5];
        let before = buf;
        engine.process(&mut buf);
        assert_eq!(buf, before);
    }
}
