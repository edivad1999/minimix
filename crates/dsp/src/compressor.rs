//! Feed-forward peak compressor with exponential attack and release.

use crate::Effect;

/// Dynamics compressor. Process path allocates nothing.
#[derive(Debug, Clone)]
pub struct Compressor {
    sample_rate: f32,
    threshold_db: f32,
    ratio: f32,
    attack_coeff: f32,
    release_coeff: f32,
    makeup: f32,
    envelope: f32,
}

impl Compressor {
    /// Build at `sample_rate` with gentle defaults.
    #[must_use]
    pub fn new(sample_rate: f32) -> Self {
        let mut comp = Self {
            sample_rate: sample_rate.max(1.0),
            threshold_db: -18.0,
            ratio: 4.0,
            attack_coeff: 0.0,
            release_coeff: 0.0,
            makeup: 1.0,
            envelope: 0.0,
        };
        comp.set_timing(0.005, 0.050);
        comp
    }

    /// Threshold in dBFS, ratio (>= 1), makeup linear gain.
    pub fn set_dynamics(&mut self, threshold_db: f32, ratio: f32, makeup_linear: f32) {
        self.threshold_db = threshold_db;
        self.ratio = ratio.max(1.0);
        self.makeup = makeup_linear.max(0.0);
    }

    /// Attack and release times in seconds.
    pub fn set_timing(&mut self, attack_s: f32, release_s: f32) {
        self.attack_coeff = time_to_coeff(attack_s, self.sample_rate);
        self.release_coeff = time_to_coeff(release_s, self.sample_rate);
    }
}

impl Default for Compressor {
    fn default() -> Self {
        Self::new(48_000.0)
    }
}

impl Effect for Compressor {
    fn process(&mut self, buffer: &mut [f32]) {
        let threshold_lin = db_to_lin(self.threshold_db);
        let slope = 1.0 - 1.0 / self.ratio;
        for sample in buffer {
            let input = sample.abs();
            if input > self.envelope {
                self.envelope += self.attack_coeff * (input - self.envelope);
            } else {
                self.envelope += self.release_coeff * (input - self.envelope);
            }

            let mut gain = 1.0;
            if self.envelope > threshold_lin && threshold_lin > 0.0 {
                let over_db = lin_to_db(self.envelope) - self.threshold_db;
                let reduction_db = over_db * slope;
                gain = db_to_lin(-reduction_db);
            }
            *sample *= gain * self.makeup;
        }
    }

    fn reset(&mut self) {
        self.envelope = 0.0;
    }
}

#[inline]
fn time_to_coeff(seconds: f32, sample_rate: f32) -> f32 {
    let seconds = seconds.max(1.0e-4);
    1.0 - (-1.0 / (seconds * sample_rate)).exp()
}

#[inline]
fn db_to_lin(db: f32) -> f32 {
    10.0_f32.powf(db / 20.0)
}

#[inline]
fn lin_to_db(lin: f32) -> f32 {
    20.0 * lin.max(1.0e-12).log10()
}
