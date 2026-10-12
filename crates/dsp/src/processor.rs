//! Top-level realtime processor.

use crate::compressor::Compressor;
use crate::equalizer::Equalizer;
use crate::Effect;

/// Chains equalizer then compressor. Bypass skips all stages.
#[derive(Debug, Clone)]
pub struct AudioProcessor {
    /// When true, `process` is a no-op.
    pub bypass: bool,
    eq: Equalizer,
    compressor: Compressor,
    makeup_gain: f32,
}

impl AudioProcessor {
    /// Create a processor at `sample_rate` with bypass off.
    #[must_use]
    pub fn new(sample_rate: f32) -> Self {
        Self {
            bypass: false,
            eq: Equalizer::new(sample_rate),
            compressor: Compressor::new(sample_rate),
            makeup_gain: 1.0,
        }
    }

    /// Mutable equalizer for parameter updates off the audio thread contract.
    pub fn equalizer_mut(&mut self) -> &mut Equalizer {
        &mut self.eq
    }

    /// Mutable compressor for parameter updates.
    pub fn compressor_mut(&mut self) -> &mut Compressor {
        &mut self.compressor
    }

    /// Linear output gain after dynamics (simple makeup / master).
    pub fn set_output_gain(&mut self, amount: f32) {
        self.makeup_gain = amount;
    }

    /// Process `buffer` in place when not bypassed.
    pub fn process(&mut self, buffer: &mut [f32]) {
        if self.bypass {
            return;
        }
        self.eq.process(buffer);
        self.compressor.process(buffer);
        let g = self.makeup_gain;
        if (g - 1.0).abs() > f32::EPSILON {
            for sample in buffer {
                *sample *= g;
            }
        }
    }

    /// Clear filter and envelope memory.
    pub fn reset(&mut self) {
        self.eq.reset();
        self.compressor.reset();
    }
}

impl Default for AudioProcessor {
    fn default() -> Self {
        Self::new(48_000.0)
    }
}

impl Effect for AudioProcessor {
    fn process(&mut self, buffer: &mut [f32]) {
        AudioProcessor::process(self, buffer);
    }

    fn reset(&mut self) {
        AudioProcessor::reset(self);
    }
}
