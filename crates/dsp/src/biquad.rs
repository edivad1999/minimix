//! Direct-form I biquad. Coefficients are recomputed on parameter change only.

/// Second-order IIR section.
#[derive(Debug, Clone, Copy)]
pub struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

impl Default for Biquad {
    fn default() -> Self {
        Self::identity()
    }
}

impl Biquad {
    /// Pass-through section.
    #[must_use]
    pub const fn identity() -> Self {
        Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }

    /// High-pass from Audio EQ Cookbook (RBJ).
    #[must_use]
    pub fn highpass(sample_rate: f32, cutoff_hz: f32, q: f32) -> Self {
        let (b0, b1, b2, a1, a2) = rbj_highpass(sample_rate, cutoff_hz, q);
        Self {
            b0,
            b1,
            b2,
            a1,
            a2,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }

    /// Peaking EQ from Audio EQ Cookbook (RBJ).
    #[must_use]
    pub fn peaking(sample_rate: f32, freq_hz: f32, q: f32, gain_db: f32) -> Self {
        let (b0, b1, b2, a1, a2) = rbj_peaking(sample_rate, freq_hz, q, gain_db);
        Self {
            b0,
            b1,
            b2,
            a1,
            a2,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }

    /// Keep delay memory while swapping coefficients.
    pub fn replace_coeffs_keep_memory(&mut self, other: Self) {
        let (x1, x2, y1, y2) = (self.x1, self.x2, self.y1, self.y2);
        *self = other;
        self.x1 = x1;
        self.x2 = x2;
        self.y1 = y1;
        self.y2 = y2;
    }

    /// Clear delay memory.
    pub fn reset(&mut self) {
        self.x1 = 0.0;
        self.x2 = 0.0;
        self.y1 = 0.0;
        self.y2 = 0.0;
    }

    /// Process one sample.
    #[inline]
    pub fn tick(&mut self, x0: f32) -> f32 {
        let y0 = self.b0 * x0 + self.b1 * self.x1 + self.b2 * self.x2
            - self.a1 * self.y1
            - self.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = x0;
        self.y2 = self.y1;
        self.y1 = y0;
        y0
    }
}

fn rbj_highpass(sample_rate: f32, cutoff_hz: f32, q: f32) -> (f32, f32, f32, f32, f32) {
    let sr = sample_rate.max(1.0);
    let f0 = cutoff_hz.clamp(1.0, sr * 0.49);
    let q = q.max(0.05);
    let w0 = std::f32::consts::TAU * f0 / sr;
    let cos_w0 = w0.cos();
    let sin_w0 = w0.sin();
    let alpha = sin_w0 / (2.0 * q);
    let b0 = (1.0 + cos_w0) / 2.0;
    let b1 = -(1.0 + cos_w0);
    let b2 = (1.0 + cos_w0) / 2.0;
    let a0 = 1.0 + alpha;
    let a1 = -2.0 * cos_w0;
    let a2 = 1.0 - alpha;
    (b0 / a0, b1 / a0, b2 / a0, a1 / a0, a2 / a0)
}

fn rbj_peaking(sample_rate: f32, freq_hz: f32, q: f32, gain_db: f32) -> (f32, f32, f32, f32, f32) {
    let sr = sample_rate.max(1.0);
    let f0 = freq_hz.clamp(1.0, sr * 0.49);
    let q = q.max(0.05);
    let a = 10.0_f32.powf(gain_db / 40.0);
    let w0 = std::f32::consts::TAU * f0 / sr;
    let cos_w0 = w0.cos();
    let sin_w0 = w0.sin();
    let alpha = sin_w0 / (2.0 * q);
    let b0 = 1.0 + alpha * a;
    let b1 = -2.0 * cos_w0;
    let b2 = 1.0 - alpha * a;
    let a0 = 1.0 + alpha / a;
    let a1 = -2.0 * cos_w0;
    let a2 = 1.0 - alpha / a;
    (b0 / a0, b1 / a0, b2 / a0, a1 / a0, a2 / a0)
}
