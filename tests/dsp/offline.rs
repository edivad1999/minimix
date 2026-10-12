//! Offline behavioral checks for `minimix-dsp`.

#![allow(clippy::cast_precision_loss)]

use minimix_dsp::AudioProcessor;

const SR: f32 = 48_000.0;

fn tone(freq_hz: f32, frames: usize) -> Vec<f32> {
    (0..frames)
        .map(|i| {
            let t = i as f32 / SR;
            (std::f32::consts::TAU * freq_hz * t).sin() * 0.5
        })
        .collect()
}

fn rms(buf: &[f32]) -> f32 {
    let sum: f32 = buf.iter().map(|s| s * s).sum();
    (sum / buf.len() as f32).sqrt()
}

fn peak(buf: &[f32]) -> f32 {
    buf.iter().fold(0.0_f32, |a, &s| a.max(s.abs()))
}

#[test]
fn impulse_is_finite() {
    let mut proc = AudioProcessor::new(SR);
    let mut buf = vec![0.0_f32; 256];
    buf[0] = 1.0;
    proc.process(&mut buf);
    assert!(buf.iter().all(|s| s.is_finite()));
}

#[test]
fn bypass_is_bit_identical() {
    let mut proc = AudioProcessor::new(SR);
    proc.equalizer_mut().set_peak(1, 1000.0, 1.0, 9.0);
    proc.bypass = true;
    let mut buf = tone(440.0, 512);
    let before = buf.clone();
    proc.process(&mut buf);
    assert_eq!(buf, before);
}

#[test]
fn highpass_attenuates_low_relative_to_mid() {
    let mut low_proc = AudioProcessor::new(SR);
    low_proc.equalizer_mut().set_highpass(300.0, 0.707);
    let mut mid_proc = AudioProcessor::new(SR);
    mid_proc.equalizer_mut().set_highpass(300.0, 0.707);

    let mut low = tone(20.0, 4800);
    let mut mid = tone(1000.0, 4800);
    // flush filter settling
    low_proc.process(&mut low[..512]);
    mid_proc.process(&mut mid[..512]);
    low_proc.process(&mut low[512..]);
    mid_proc.process(&mut mid[512..]);

    let low_e = rms(&low[512..]);
    let mid_e = rms(&mid[512..]);
    assert!(
        low_e < mid_e * 0.25,
        "low={low_e} mid={mid_e} expected low << mid"
    );
}

#[test]
fn compressor_reduces_hot_peak() {
    let mut proc = AudioProcessor::new(SR);
    proc.equalizer_mut().set_highpass(1.0, 0.707);
    proc.compressor_mut().set_dynamics(-24.0, 20.0, 1.0);
    proc.compressor_mut().set_timing(0.0005, 0.050);
    let mut buf = vec![0.95_f32; 8192];
    let before = peak(&buf);
    proc.process(&mut buf);
    let after = peak(&buf[4096..]);
    assert!(
        after < before * 0.5,
        "before={before} after={after} expected reduction"
    );
}

#[test]
fn mid_buffer_param_change_stays_finite() {
    let mut proc = AudioProcessor::new(SR);
    let mut buf = tone(880.0, 1024);
    proc.process(&mut buf[..512]);
    proc.equalizer_mut().set_peak(1, 880.0, 2.0, 6.0);
    proc.compressor_mut().set_dynamics(-20.0, 3.0, 1.2);
    proc.process(&mut buf[512..]);
    assert!(buf.iter().all(|s| s.is_finite()));
}

#[test]
fn identical_mono_channels_match_when_processed_separately() {
    let mut left_proc = AudioProcessor::new(SR);
    let mut right_proc = AudioProcessor::new(SR);
    left_proc.equalizer_mut().set_peak(1, 1000.0, 1.0, 3.0);
    right_proc.equalizer_mut().set_peak(1, 1000.0, 1.0, 3.0);
    let mut left = tone(1000.0, 512);
    let mut right = left.clone();
    left_proc.process(&mut left);
    right_proc.process(&mut right);
    for (l, r) in left.iter().zip(right.iter()) {
        assert!((l - r).abs() < 1.0e-6);
    }
}

#[test]
fn extreme_gain_stays_finite() {
    let mut proc = AudioProcessor::new(SR);
    proc.set_output_gain(1000.0);
    proc.equalizer_mut().set_peak(1, 1000.0, 1.0, 24.0);
    let mut buf = tone(1000.0, 256);
    proc.process(&mut buf);
    assert!(buf.iter().all(|s| s.is_finite()));
}
