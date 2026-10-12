//! Linux `PipeWire` capture → DSP rings → `Audio/Source` publish.

use crate::config::VirtualMicConfig;
use crate::error::PipewireError;
use crate::ring::SampleRing;
use crate::stats::RingStats;
use minimix_engine::Engine;
use pipewire as pw;
use pw::spa::pod::Pod;
use pw::stream::{StreamFlags, StreamState};
use pw::{properties::properties, spa};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

const RING_CAPACITY: usize = 16_384;
const DSP_BLOCK: usize = 256;
/// `SPA_AUDIO_CHANNEL_MONO`
const CHANNEL_MONO: u32 = 1;

type ReadyTx = std::sync::mpsc::Sender<Result<pw::channel::Sender<()>, PipewireError>>;

/// Running virtual-mic session. Dropping requests stop and joins the PW thread.
pub struct VirtualMicSession {
    quit: Option<pw::channel::Sender<()>>,
    ready: Arc<AtomicBool>,
    stats: Arc<RingStats>,
    join: Option<JoinHandle<Result<(), PipewireError>>>,
}

impl VirtualMicSession {
    /// Ask the `PipeWire` thread to quit.
    pub fn request_stop(&self) {
        if let Some(tx) = &self.quit {
            let _ = tx.send(());
        }
    }

    /// True once the published source reaches `Streaming`.
    #[must_use]
    pub fn is_ready(&self) -> bool {
        self.ready.load(Ordering::Acquire)
    }

    /// Shared ring hop counters.
    #[must_use]
    pub fn stats(&self) -> Arc<RingStats> {
        Arc::clone(&self.stats)
    }
}

impl Drop for VirtualMicSession {
    fn drop(&mut self) {
        self.request_stop();
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

/// Start capture + DSP + `Audio/Source` on a dedicated `PipeWire` thread.
///
/// # Errors
///
/// Returns [`PipewireError`] when the session thread fails to spawn or `PipeWire` setup fails.
pub fn start_virtual_mic(
    config: VirtualMicConfig,
    engine: Arc<Mutex<Engine>>,
) -> Result<VirtualMicSession, PipewireError> {
    let ready = Arc::new(AtomicBool::new(false));
    let stats = Arc::new(RingStats::default());
    let (ready_tx, ready_rx) = std::sync::mpsc::channel();

    let ready_flag = Arc::clone(&ready);
    let stats_thread = Arc::clone(&stats);
    let join = thread::Builder::new()
        .name("minimix-pipewire".into())
        .spawn(move || run_session(config, engine, ready_flag, stats_thread, ready_tx))
        .map_err(|e| PipewireError::Session(e.to_string()))?;

    let quit = ready_rx
        .recv_timeout(Duration::from_secs(15))
        .map_err(|e| PipewireError::Session(e.to_string()))??;

    Ok(VirtualMicSession {
        quit: Some(quit),
        ready,
        stats,
        join: Some(join),
    })
}

fn fail_ready(ready_tx: &ReadyTx, err: PipewireError) -> PipewireError {
    let _ = ready_tx.send(Err(PipewireError::Pipewire(err.to_string())));
    err
}

#[allow(clippy::too_many_lines)]
#[allow(clippy::needless_pass_by_value)] // config + Arcs moved onto the PipeWire thread
fn run_session(
    config: VirtualMicConfig,
    engine: Arc<Mutex<Engine>>,
    ready: Arc<AtomicBool>,
    stats: Arc<RingStats>,
    ready_tx: ReadyTx,
) -> Result<(), PipewireError> {
    pw::init();

    let mainloop = pw::main_loop::MainLoopRc::new(None)
        .map_err(|e| fail_ready(&ready_tx, PipewireError::Pipewire(e.to_string())))?;
    let context = pw::context::ContextRc::new(&mainloop, None)
        .map_err(|e| fail_ready(&ready_tx, PipewireError::Pipewire(e.to_string())))?;
    let core = context
        .connect_rc(None)
        .map_err(|e| fail_ready(&ready_tx, PipewireError::Pipewire(e.to_string())))?;

    let capture_ring = Arc::new(SampleRing::with_capacity(RING_CAPACITY));
    let source_ring = Arc::new(SampleRing::with_capacity(RING_CAPACITY));
    let wake = Arc::new((Mutex::new(()), Condvar::new()));
    let stop_dsp = Arc::new(AtomicBool::new(false));
    let capture_channels = Arc::new(AtomicU32::new(config.channels));

    let dsp_stop = Arc::clone(&stop_dsp);
    let dsp_capture = Arc::clone(&capture_ring);
    let dsp_source = Arc::clone(&source_ring);
    let dsp_wake = Arc::clone(&wake);
    let dsp_stats = Arc::clone(&stats);
    let dsp_engine = Arc::clone(&engine);
    let dsp_join = thread::Builder::new()
        .name("minimix-dsp".into())
        .spawn(move || {
            dsp_loop(
                dsp_stop,
                dsp_capture,
                dsp_source,
                dsp_wake,
                dsp_engine,
                dsp_stats,
            );
        })
        .map_err(|e| fail_ready(&ready_tx, PipewireError::Session(e.to_string())))?;

    let (quit_tx, quit_rx) = pw::channel::channel::<()>();
    let _quit_attach = quit_rx.attach(mainloop.loop_(), {
        let mainloop = mainloop.clone();
        let stop_dsp = Arc::clone(&stop_dsp);
        let wake = Arc::clone(&wake);
        move |()| {
            stop_dsp.store(true, Ordering::Release);
            wake.1.notify_one();
            mainloop.quit();
        }
    });

    let mut capture_props = properties! {
        *pw::keys::MEDIA_TYPE => "Audio",
        *pw::keys::MEDIA_CATEGORY => "Capture",
        *pw::keys::MEDIA_ROLE => "Music",
        *pw::keys::MEDIA_CLASS => "Stream/Input/Audio",
        "node.want-driver" => "true",
    };
    capture_props.insert("node.link-group", config.link_group.as_str());
    capture_props.insert(*pw::keys::NODE_DESCRIPTION, "Minimix Capture");
    if let Some(target) = &config.capture_target {
        capture_props.insert("target.object", target.as_str());
    }

    let capture = pw::stream::StreamBox::new(&core, "minimix-capture", capture_props)
        .map_err(|e| fail_ready(&ready_tx, PipewireError::Pipewire(e.to_string())))?;

    let capture_ring_cb = Arc::clone(&capture_ring);
    let capture_stats = Arc::clone(&stats);
    let capture_wake = Arc::clone(&wake);
    let capture_ch = Arc::clone(&capture_channels);
    let _capture_listener = capture
        .add_local_listener_with_user_data(())
        .param_changed({
            let capture_ch = Arc::clone(&capture_channels);
            move |_, (), id, param| {
                let Some(param) = param else {
                    return;
                };
                if id != spa::param::ParamType::Format.as_raw() {
                    return;
                }
                let mut info = spa::param::audio::AudioInfoRaw::new();
                if info.parse(param).is_ok() {
                    capture_ch.store(info.channels().max(1), Ordering::Release);
                }
            }
        })
        .process(move |stream, ()| {
            let Some(mut buffer) = stream.dequeue_buffer() else {
                return;
            };
            let datas = buffer.datas_mut();
            if datas.is_empty() {
                return;
            }
            let data = &mut datas[0];
            let channels = capture_ch.load(Ordering::Acquire).max(1) as usize;
            let n_samples = data.chunk().size() as usize / std::mem::size_of::<f32>();
            let Some(samples) = data.data() else {
                return;
            };
            let frames = n_samples / channels;
            if frames == 0 {
                return;
            }
            let mut mono = [0.0_f32; 2048];
            let frames = frames.min(mono.len());
            for (i, out) in mono[..frames].iter_mut().enumerate() {
                let mut sum = 0.0_f32;
                for ch in 0..channels {
                    let idx = (i * channels + ch) * std::mem::size_of::<f32>();
                    let end = idx + std::mem::size_of::<f32>();
                    if end > samples.len() {
                        break;
                    }
                    let mut bytes = [0_u8; 4];
                    bytes.copy_from_slice(&samples[idx..end]);
                    sum += f32::from_le_bytes(bytes);
                }
                #[allow(clippy::cast_precision_loss)]
                {
                    *out = sum / channels as f32;
                }
            }
            let wrote = capture_ring_cb.push_slice(&mono[..frames]);
            if wrote < frames {
                capture_stats
                    .capture_overruns
                    .fetch_add(1, Ordering::Relaxed);
            }
            capture_wake.1.notify_one();
        })
        .register()
        .map_err(|e| fail_ready(&ready_tx, PipewireError::Pipewire(e.to_string())))?;

    let mut source_props = properties! {
        *pw::keys::MEDIA_TYPE => "Audio",
        *pw::keys::MEDIA_CATEGORY => "Capture",
        *pw::keys::MEDIA_ROLE => "Music",
        *pw::keys::MEDIA_CLASS => "Audio/Source",
        *pw::keys::NODE_VIRTUAL => "true",
    };
    source_props.insert(*pw::keys::NODE_NAME, config.node_name.as_str());
    source_props.insert(*pw::keys::NODE_DESCRIPTION, config.media_name.as_str());
    source_props.insert(*pw::keys::MEDIA_NAME, config.media_name.as_str());
    source_props.insert(*pw::keys::AUDIO_CHANNELS, config.channels.to_string());
    source_props.insert(
        *pw::keys::PRIORITY_SESSION,
        config.priority_session.to_string(),
    );
    source_props.insert("node.link-group", config.link_group.as_str());
    source_props.insert("audio.position", "[MONO]");

    let source = pw::stream::StreamBox::new(&core, config.media_name.as_str(), source_props)
        .map_err(|e| fail_ready(&ready_tx, PipewireError::Pipewire(e.to_string())))?;

    let source_ring_cb = Arc::clone(&source_ring);
    let source_stats = Arc::clone(&stats);
    let ready_cb = Arc::clone(&ready);
    let ready_node = config.node_name.clone();
    let _source_listener = source
        .add_local_listener_with_user_data(())
        .state_changed(move |_, (), _old, new| {
            if matches!(new, StreamState::Streaming) {
                ready_cb.store(true, Ordering::Release);
                eprintln!("source_ready node={ready_node}");
            }
        })
        .process(move |stream, ()| {
            let Some(mut buffer) = stream.dequeue_buffer() else {
                return;
            };
            let datas = buffer.datas_mut();
            if datas.is_empty() {
                return;
            }
            let data = &mut datas[0];
            let stride = std::mem::size_of::<f32>();
            let mut mono = [0.0_f32; 2048];
            let frames = {
                let Some(samples) = data.data() else {
                    return;
                };
                let max_frames = samples.len() / stride;
                let frames = max_frames.min(mono.len());
                let got = source_ring_cb.pop_slice(&mut mono[..frames]);
                if got < frames {
                    source_stats
                        .source_underruns
                        .fetch_add(1, Ordering::Relaxed);
                    mono[got..frames].fill(0.0);
                }
                for (i, sample) in mono[..frames].iter().enumerate() {
                    let start = i * stride;
                    let end = start + stride;
                    samples[start..end].copy_from_slice(&sample.to_le_bytes());
                }
                frames
            };
            let chunk = data.chunk_mut();
            *chunk.offset_mut() = 0;
            *chunk.stride_mut() = i32::try_from(stride).unwrap_or(i32::MAX);
            *chunk.size_mut() = u32::try_from(frames * stride).unwrap_or(u32::MAX);
        })
        .register()
        .map_err(|e| fail_ready(&ready_tx, PipewireError::Pipewire(e.to_string())))?;

    let capture_params = make_audio_params(config.sample_rate, config.channels)
        .map_err(|e| fail_ready(&ready_tx, e))?;
    let mut capture_pods = [Pod::from_bytes(&capture_params).ok_or_else(|| {
        fail_ready(
            &ready_tx,
            PipewireError::Pipewire("capture format pod".into()),
        )
    })?];
    capture
        .connect(
            spa::utils::Direction::Input,
            None,
            StreamFlags::AUTOCONNECT | StreamFlags::MAP_BUFFERS | StreamFlags::RT_PROCESS,
            &mut capture_pods,
        )
        .map_err(|e| fail_ready(&ready_tx, PipewireError::Pipewire(e.to_string())))?;

    let source_params = make_audio_params(config.sample_rate, config.channels)
        .map_err(|e| fail_ready(&ready_tx, e))?;
    let mut source_pods = [Pod::from_bytes(&source_params).ok_or_else(|| {
        fail_ready(
            &ready_tx,
            PipewireError::Pipewire("source format pod".into()),
        )
    })?];
    source
        .connect(
            spa::utils::Direction::Output,
            None,
            StreamFlags::MAP_BUFFERS | StreamFlags::RT_PROCESS,
            &mut source_pods,
        )
        .map_err(|e| fail_ready(&ready_tx, PipewireError::Pipewire(e.to_string())))?;

    if ready_tx.send(Ok(quit_tx)).is_err() {
        stop_dsp.store(true, Ordering::Release);
        wake.1.notify_one();
        return Err(PipewireError::Session("caller dropped".into()));
    }

    eprintln!(
        "daemon_pipewire_running node={} rate={} channels={}",
        config.node_name, config.sample_rate, config.channels
    );
    mainloop.run();

    stop_dsp.store(true, Ordering::Release);
    wake.1.notify_one();
    let _ = dsp_join.join();
    ready.store(false, Ordering::Release);
    Ok(())
}

fn make_audio_params(rate: u32, channels: u32) -> Result<Vec<u8>, PipewireError> {
    let mut audio_info = spa::param::audio::AudioInfoRaw::new();
    audio_info.set_format(spa::param::audio::AudioFormat::F32LE);
    audio_info.set_rate(rate);
    audio_info.set_channels(channels);
    let mut position = [0_u32; spa::param::audio::MAX_CHANNELS];
    position[0] = CHANNEL_MONO;
    audio_info.set_position(position);

    let values = spa::pod::serialize::PodSerializer::serialize(
        std::io::Cursor::new(Vec::new()),
        &spa::pod::Value::Object(spa::pod::Object {
            type_: spa::utils::SpaTypes::ObjectParamFormat.as_raw(),
            id: spa::param::ParamType::EnumFormat.as_raw(),
            properties: audio_info.into(),
        }),
    )
    .map_err(|e| PipewireError::Pipewire(e.to_string()))?
    .0
    .into_inner();
    Ok(values)
}

#[allow(clippy::needless_pass_by_value)] // Arcs moved onto the DSP thread
fn dsp_loop(
    stop: Arc<AtomicBool>,
    capture_ring: Arc<SampleRing>,
    source_ring: Arc<SampleRing>,
    wake: Arc<(Mutex<()>, Condvar)>,
    engine: Arc<Mutex<Engine>>,
    stats: Arc<RingStats>,
) {
    let mut block = [0.0_f32; DSP_BLOCK];
    while !stop.load(Ordering::Acquire) {
        let got = capture_ring.pop_slice(&mut block);
        if got == 0 {
            let (lock, cvar) = &*wake;
            let Ok(guard) = lock.lock() else {
                break;
            };
            let _ = cvar.wait_timeout(guard, Duration::from_millis(5));
            continue;
        }
        let t0 = Instant::now();
        if let Ok(mut eng) = engine.lock() {
            eng.process(&mut block[..got]);
        }
        let ns = u64::try_from(t0.elapsed().as_nanos()).unwrap_or(u64::MAX);
        stats.dsp_blocks.fetch_add(1, Ordering::Relaxed);
        let _ = stats.max_dsp_ns.fetch_max(ns, Ordering::Relaxed);
        let _ = source_ring.push_slice(&block[..got]);
    }
}
