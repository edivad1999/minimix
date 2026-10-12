//! Real-time DSP for Minimix.
//!
//! Process paths must stay allocation-free. Parameter updates arrive through
//! atomics or a preallocated control channel owned by callers.

/// Marker for effects that may run inside an audio callback.
pub trait Effect: Send {
    /// Process `buffer` in place.
    fn process(&mut self, buffer: &mut [f32]);

    /// Clear filter memory and envelopes.
    fn reset(&mut self);
}

/// Placeholder gain stage used to stand up the workspace and CI.
#[derive(Debug, Clone)]
pub struct Gain {
    /// Linear amplitude multiplier.
    pub amount: f32,
}

impl Default for Gain {
    fn default() -> Self {
        Self { amount: 1.0 }
    }
}

impl Effect for Gain {
    fn process(&mut self, buffer: &mut [f32]) {
        let amount = self.amount;
        for sample in buffer {
            *sample *= amount;
        }
    }

    fn reset(&mut self) {}
}

/// Top-level processor. More stages land in later PRs.
#[derive(Debug, Default)]
pub struct AudioProcessor {
    /// When true, `process` is a no-op.
    pub bypass: bool,
    gain: Gain,
}

impl AudioProcessor {
    /// Create a processor with unity gain and bypass off.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Process `buffer` in place when not bypassed.
    pub fn process(&mut self, buffer: &mut [f32]) {
        if self.bypass {
            return;
        }
        self.gain.process(buffer);
    }

    /// Set linear gain.
    pub fn set_gain(&mut self, amount: f32) {
        self.gain.amount = amount;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unity_gain_leaves_samples_unchanged() {
        let mut proc = AudioProcessor::new();
        let mut buffer = [0.5_f32, -0.25, 0.0];
        proc.process(&mut buffer);
        assert_eq!(buffer, [0.5, -0.25, 0.0]);
    }

    #[test]
    fn bypass_skips_gain() {
        let mut proc = AudioProcessor::new();
        proc.set_gain(2.0);
        proc.bypass = true;
        let mut buffer = [1.0_f32];
        proc.process(&mut buffer);
        assert_eq!(buffer, [1.0]);
    }

    #[test]
    fn gain_scales_samples() {
        let mut proc = AudioProcessor::new();
        proc.set_gain(0.5);
        let mut buffer = [1.0_f32, -2.0];
        proc.process(&mut buffer);
        assert_eq!(buffer, [0.5, -1.0]);
    }
}
