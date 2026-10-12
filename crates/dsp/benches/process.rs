//! Simple process-path timing (no criterion dep).

#![allow(clippy::cast_precision_loss)]

use minimix_dsp::AudioProcessor;
use std::hint::black_box;
use std::time::Instant;

fn main() {
    const FRAMES: usize = 256;
    const ITERS: usize = 200_000;
    let mut proc = AudioProcessor::new(48_000.0);
    proc.equalizer_mut().set_peak(1, 1000.0, 1.0, 3.0);
    proc.compressor_mut().set_dynamics(-18.0, 4.0, 1.0);
    let mut buf = [0.1_f32; FRAMES];

    for _ in 0..1_000 {
        proc.process(&mut buf);
    }

    let start = Instant::now();
    for _ in 0..ITERS {
        proc.process(black_box(&mut buf));
    }
    black_box(buf);
    let ns = start.elapsed().as_nanos() as f64 / ITERS as f64;
    let budget = (FRAMES as f64 / 48_000.0) * 1e9;
    println!("frames={FRAMES}");
    println!("iters={ITERS}");
    println!("ns_per_buffer={ns:.1}");
    println!("rt_budget_ns={budget:.1}");
    println!("cpu_fraction={:.6}", ns / budget);
    assert!(ns < 500_000.0, "process exceeded 500us budget: {ns} ns");
}
