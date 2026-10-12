//! Device discovery and duplex monitor streams via cpal.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, SampleFormat, Stream, StreamConfig};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

const MAX_FRAMES: usize = 4096;

/// Errors from opening or running audio devices.
#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    /// Host or device enumeration failed.
    #[error("audio host error: {0}")]
    Host(String),
    /// Requested configuration is unavailable.
    #[error("unsupported config: {0}")]
    Config(String),
    /// Stream build or start failed.
    #[error("stream error: {0}")]
    Stream(String),
}

/// Named input device.
#[derive(Debug, Clone)]
pub struct InputDeviceInfo {
    /// Device name from the host.
    pub name: String,
}

/// List input devices on the default host.
///
/// # Errors
///
/// Returns [`AudioError::Host`] when the host cannot enumerate devices.
pub fn list_input_devices() -> Result<Vec<InputDeviceInfo>, AudioError> {
    let host = cpal::default_host();
    let mut out = Vec::new();
    let devices = host
        .input_devices()
        .map_err(|e| AudioError::Host(e.to_string()))?;
    for device in devices {
        let name = device.name().unwrap_or_else(|_| "<unknown>".into());
        out.push(InputDeviceInfo { name });
    }
    Ok(out)
}

/// Live callback stats for the monitor path.
#[derive(Debug, Default)]
pub struct CallbackStats {
    /// Callbacks observed.
    pub callbacks: AtomicU64,
    /// Max callback duration in nanoseconds.
    pub max_callback_ns: AtomicU64,
    /// Frames delivered to the process callback.
    pub frames: AtomicU64,
    /// Underruns when the playback ring was empty.
    pub underruns: AtomicU64,
}

/// Running duplex monitor. Dropping stops the streams.
pub struct MonitorSession {
    _input: Stream,
    _output: Stream,
    /// Shared stop flag.
    pub stop: Arc<AtomicBool>,
    /// Callback statistics.
    pub stats: Arc<CallbackStats>,
}

impl MonitorSession {
    /// Signal streams to stop. Dropping also ends them.
    pub fn request_stop(&self) {
        self.stop.store(true, Ordering::Release);
    }
}

/// Capture-only session. Dropping stops the input stream.
pub struct CaptureSession {
    _input: Stream,
    /// Shared stop flag.
    pub stop: Arc<AtomicBool>,
    /// Callback statistics.
    pub stats: Arc<CallbackStats>,
    /// Negotiated sample rate in Hz.
    pub sample_rate: u32,
}

impl CaptureSession {
    /// Signal the input stream to stop. Dropping also ends it.
    pub fn request_stop(&self) {
        self.stop.store(true, Ordering::Release);
    }
}

/// Open default input and output, process through `on_block`.
///
/// `on_block` runs on the capture callback with mono f32 frames.
///
/// # Errors
///
/// Returns [`AudioError`] when devices are missing or streams fail to start.
#[allow(clippy::too_many_lines)]
pub fn start_monitor<F>(mut on_block: F) -> Result<MonitorSession, AudioError>
where
    F: FnMut(&mut [f32]) + Send + 'static,
{
    let host = cpal::default_host();
    let input = host
        .default_input_device()
        .ok_or_else(|| AudioError::Host("no default input".into()))?;
    let output = host
        .default_output_device()
        .ok_or_else(|| AudioError::Host("no default output".into()))?;

    let in_name = input.name().unwrap_or_else(|_| "<in>".into());
    let out_name = output.name().unwrap_or_else(|_| "<out>".into());
    eprintln!("monitor_started input={in_name} output={out_name}");

    let in_config = pick_f32_config(&input, true)?;
    let out_config = pick_f32_config(&output, false)?;
    eprintln!(
        "monitor_config in_hz={} in_ch={} out_hz={} out_ch={}",
        in_config.sample_rate.0, in_config.channels, out_config.sample_rate.0, out_config.channels
    );

    let stop = Arc::new(AtomicBool::new(false));
    let stats = Arc::new(CallbackStats::default());
    let ring = Arc::new(Mutex::new(Vec::<f32>::with_capacity(48_000)));

    let stats_in = Arc::clone(&stats);
    let ring_in = Arc::clone(&ring);
    let stop_in = Arc::clone(&stop);
    let in_channels = in_config.channels as usize;
    let mut mono_scratch = [0.0_f32; MAX_FRAMES];

    let input_stream = input
        .build_input_stream(
            &in_config,
            move |data: &[f32], _| {
                if stop_in.load(Ordering::Acquire) {
                    return;
                }
                let t0 = std::time::Instant::now();
                let frames = (data.len() / in_channels.max(1)).min(MAX_FRAMES);
                let mono = &mut mono_scratch[..frames];
                for (i, sample) in mono.iter_mut().enumerate() {
                    let mut sum = 0.0_f32;
                    for ch in 0..in_channels {
                        sum += data[i * in_channels + ch];
                    }
                    #[allow(clippy::cast_precision_loss)]
                    {
                        *sample = sum / in_channels as f32;
                    }
                }
                on_block(mono);
                if let Ok(mut q) = ring_in.lock() {
                    q.extend_from_slice(mono);
                    let max_keep = 48_000usize;
                    if q.len() > max_keep {
                        let drain = q.len() - max_keep;
                        q.drain(0..drain);
                    }
                }
                let ns = u64::try_from(t0.elapsed().as_nanos()).unwrap_or(u64::MAX);
                stats_in.callbacks.fetch_add(1, Ordering::Relaxed);
                stats_in.frames.fetch_add(frames as u64, Ordering::Relaxed);
                let _ = stats_in.max_callback_ns.fetch_max(ns, Ordering::Relaxed);
            },
            |err| eprintln!("input stream error: {err}"),
            None,
        )
        .map_err(|e| AudioError::Stream(e.to_string()))?;

    let stats_out = Arc::clone(&stats);
    let ring_out = Arc::clone(&ring);
    let stop_out = Arc::clone(&stop);
    let out_channels = out_config.channels as usize;
    let mut out_scratch = [0.0_f32; MAX_FRAMES];

    let output_stream = output
        .build_output_stream(
            &out_config,
            move |data: &mut [f32], _| {
                if stop_out.load(Ordering::Acquire) {
                    data.fill(0.0);
                    return;
                }
                let frames = (data.len() / out_channels.max(1)).min(MAX_FRAMES);
                let mono = &mut out_scratch[..frames];
                mono.fill(0.0);
                if let Ok(mut q) = ring_out.lock() {
                    let take = frames.min(q.len());
                    mono[..take].copy_from_slice(&q[..take]);
                    q.drain(0..take);
                    if take < frames {
                        stats_out.underruns.fetch_add(1, Ordering::Relaxed);
                    }
                }
                for i in 0..frames {
                    for ch in 0..out_channels {
                        data[i * out_channels + ch] = mono[i];
                    }
                }
            },
            |err| eprintln!("output stream error: {err}"),
            None,
        )
        .map_err(|e| AudioError::Stream(e.to_string()))?;

    input_stream
        .play()
        .map_err(|e| AudioError::Stream(e.to_string()))?;
    output_stream
        .play()
        .map_err(|e| AudioError::Stream(e.to_string()))?;

    Ok(MonitorSession {
        _input: input_stream,
        _output: output_stream,
        stop,
        stats,
    })
}

/// Open the default input device and deliver mono f32 frames to `on_block`.
///
/// # Errors
///
/// Returns [`AudioError`] when no input device is available or the stream fails.
pub fn start_capture<F>(mut on_block: F) -> Result<CaptureSession, AudioError>
where
    F: FnMut(&mut [f32]) + Send + 'static,
{
    let host = cpal::default_host();
    let input = host
        .default_input_device()
        .ok_or_else(|| AudioError::Host("no default input".into()))?;
    let in_name = input.name().unwrap_or_else(|_| "<in>".into());
    let in_config = pick_f32_config(&input, true)?;
    eprintln!(
        "capture_started input={in_name} hz={} ch={}",
        in_config.sample_rate.0, in_config.channels
    );

    let stop = Arc::new(AtomicBool::new(false));
    let stats = Arc::new(CallbackStats::default());
    let stats_in = Arc::clone(&stats);
    let stop_in = Arc::clone(&stop);
    let in_channels = in_config.channels as usize;
    let sample_rate = in_config.sample_rate.0;
    let mut mono_scratch = [0.0_f32; MAX_FRAMES];

    let input_stream = input
        .build_input_stream(
            &in_config,
            move |data: &[f32], _| {
                if stop_in.load(Ordering::Acquire) {
                    return;
                }
                let t0 = std::time::Instant::now();
                let frames = (data.len() / in_channels.max(1)).min(MAX_FRAMES);
                let mono = &mut mono_scratch[..frames];
                for (i, sample) in mono.iter_mut().enumerate() {
                    let mut sum = 0.0_f32;
                    for ch in 0..in_channels {
                        sum += data[i * in_channels + ch];
                    }
                    #[allow(clippy::cast_precision_loss)]
                    {
                        *sample = sum / in_channels as f32;
                    }
                }
                on_block(mono);
                let ns = u64::try_from(t0.elapsed().as_nanos()).unwrap_or(u64::MAX);
                stats_in.callbacks.fetch_add(1, Ordering::Relaxed);
                stats_in.frames.fetch_add(frames as u64, Ordering::Relaxed);
                let _ = stats_in.max_callback_ns.fetch_max(ns, Ordering::Relaxed);
            },
            |err| eprintln!("input stream error: {err}"),
            None,
        )
        .map_err(|e| AudioError::Stream(e.to_string()))?;

    input_stream
        .play()
        .map_err(|e| AudioError::Stream(e.to_string()))?;

    Ok(CaptureSession {
        _input: input_stream,
        stop,
        stats,
        sample_rate,
    })
}

fn pick_f32_config(device: &Device, input: bool) -> Result<StreamConfig, AudioError> {
    if input {
        let default = device
            .default_input_config()
            .map_err(|e| AudioError::Config(e.to_string()))?;
        if default.sample_format() == SampleFormat::F32 {
            return Ok(default.config());
        }
        let ranges = device
            .supported_input_configs()
            .map_err(|e| AudioError::Config(e.to_string()))?;
        for range in ranges {
            if range.sample_format() == SampleFormat::F32 {
                return Ok(range.with_sample_rate(default.sample_rate()).config());
            }
        }
    } else {
        let default = device
            .default_output_config()
            .map_err(|e| AudioError::Config(e.to_string()))?;
        if default.sample_format() == SampleFormat::F32 {
            return Ok(default.config());
        }
        let ranges = device
            .supported_output_configs()
            .map_err(|e| AudioError::Config(e.to_string()))?;
        for range in ranges {
            if range.sample_format() == SampleFormat::F32 {
                return Ok(range.with_sample_rate(default.sample_rate()).config());
            }
        }
    }

    Err(AudioError::Config(
        "no f32 stream configuration available".into(),
    ))
}
