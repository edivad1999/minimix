//! High-pass plus a small bank of peaking bands.

use crate::biquad::Biquad;
use crate::Effect;

/// Number of peaking bands after the high-pass.
pub const PEAK_BANDS: usize = 3;

/// Parametric equalizer. Process path allocates nothing.
#[derive(Debug, Clone)]
pub struct Equalizer {
    sample_rate: f32,
    highpass: Biquad,
    peaks: [Biquad; PEAK_BANDS],
}

impl Equalizer {
    /// Build at `sample_rate` with an 80 Hz high-pass and flat peaks.
    #[must_use]
    pub fn new(sample_rate: f32) -> Self {
        let mut eq = Self {
            sample_rate: sample_rate.max(1.0),
            highpass: Biquad::identity(),
            peaks: [Biquad::identity(); PEAK_BANDS],
        };
        eq.set_highpass(80.0, 0.707);
        eq.set_peak(0, 250.0, 1.0, 0.0);
        eq.set_peak(1, 1000.0, 1.0, 0.0);
        eq.set_peak(2, 4000.0, 1.0, 0.0);
        eq
    }

    /// Configure the high-pass cutoff and Q.
    pub fn set_highpass(&mut self, cutoff_hz: f32, q: f32) {
        let next = Biquad::highpass(self.sample_rate, cutoff_hz, q);
        self.highpass.replace_coeffs_keep_memory(next);
    }

    /// Configure peaking band `index` (`0..PEAK_BANDS`).
    pub fn set_peak(&mut self, index: usize, freq_hz: f32, q: f32, gain_db: f32) {
        if let Some(peak) = self.peaks.get_mut(index) {
            let next = Biquad::peaking(self.sample_rate, freq_hz, q, gain_db);
            peak.replace_coeffs_keep_memory(next);
        }
    }
}

impl Default for Equalizer {
    fn default() -> Self {
        Self::new(48_000.0)
    }
}

impl Effect for Equalizer {
    fn process(&mut self, buffer: &mut [f32]) {
        for sample in buffer {
            let mut x = self.highpass.tick(*sample);
            for peak in &mut self.peaks {
                x = peak.tick(x);
            }
            *sample = x;
        }
    }

    fn reset(&mut self) {
        self.highpass.reset();
        for peak in &mut self.peaks {
            peak.reset();
        }
    }
}
