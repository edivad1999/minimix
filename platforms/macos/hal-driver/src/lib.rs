//! Thin Core Audio `AudioServerPlugIn` that exposes Minimix as a system input.
//!
//! DSP stays in the userspace daemon. This plug-in hosts the
//! `com.minimix.audio` mach service, pushes received PCM into a lock-free
//! ring, and serves clients from [`Driver::process_io`].

use minimix_ipc_mac::{SampleConsumer, SampleProducer, SampleRing, SERVICE_NAME};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::{self, JoinHandle};
use tympan_aspl::{
    BundleConfig, DeviceSpec, Driver, IoBuffer, RealtimeContext, StreamFormat, StreamSpec,
};

/// Stable device UID / bundle identifier. Do not change after release.
pub const DEVICE_UID: &str = "com.minimix.hal";
/// `CFPlugIn` factory UUID (unique to this driver).
pub const FACTORY_UUID: &str = "818352F8-693E-4308-BEA0-4A5EAC058237";
/// `CFPlugIn` factory symbol name emitted by [`tympan_aspl::plugin_entry!`].
pub const FACTORY_FUNCTION: &str = "TympanAsplDriverFactory";
/// Virtual mic sample rate.
pub const SAMPLE_RATE: f64 = 48_000.0;
/// Mono input for Discord / Zoom / Teams style capture.
pub const CHANNELS: u32 = 1;
/// Ring capacity: ~1 s of mono float32 at 48 kHz.
const RING_CAPACITY: usize = 48_000;

struct FeedSlots {
    producer: Mutex<Option<SampleProducer>>,
    consumer: Mutex<Option<SampleConsumer>>,
}

impl FeedSlots {
    fn new() -> Self {
        let (producer, consumer) = SampleRing::channel(RING_CAPACITY);
        Self {
            producer: Mutex::new(Some(producer)),
            consumer: Mutex::new(Some(consumer)),
        }
    }
}

fn feed() -> &'static FeedSlots {
    static FEED: OnceLock<FeedSlots> = OnceLock::new();
    FEED.get_or_init(FeedSlots::new)
}

/// Minimix virtual microphone driver.
pub struct MinimixDriver {
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl MinimixDriver {
    /// Bundle config used to generate and pin `Info.plist`.
    #[must_use]
    pub const fn bundle_config() -> BundleConfig {
        BundleConfig::new(DEVICE_UID, FACTORY_UUID, FACTORY_FUNCTION)
            .with_bundle_name("Minimix")
            .with_executable("Minimix")
            .with_version(Self::VERSION)
    }
}

impl Driver for MinimixDriver {
    const NAME: &'static str = "Minimix";
    const MANUFACTURER: &'static str = "Minimix";
    const VERSION: &'static str = "0.1.0";

    fn new() -> Self {
        let _ = feed();
        Self {
            stop: Arc::new(AtomicBool::new(false)),
            worker: None,
        }
    }

    fn device(&self) -> DeviceSpec {
        let format = StreamFormat::float32(SAMPLE_RATE, CHANNELS);
        DeviceSpec::new(DEVICE_UID, "Minimix", Self::MANUFACTURER)
            .with_sample_rate(SAMPLE_RATE)
            .with_input(StreamSpec::input(format))
    }

    fn initialize(&mut self) -> Result<(), tympan_aspl::OsStatus> {
        self.stop.store(false, Ordering::Release);
        let producer = match feed().producer.lock() {
            Ok(mut guard) => guard.take(),
            Err(_) => None,
        };
        let Some(producer) = producer else {
            return Ok(());
        };
        let stop = Arc::clone(&self.stop);
        let handle = thread::Builder::new()
            .name("minimix-mach-recv".into())
            .spawn(move || mach_recv_loop(producer, stop))
            .map_err(|_| tympan_aspl::OsStatus::UNSPECIFIED)?;
        self.worker = Some(handle);
        Ok(())
    }

    fn process_io(&mut self, _rt: &RealtimeContext, buffer: &mut IoBuffer<'_>) {
        let dst = &mut *buffer.output;
        if dst.is_empty() {
            return;
        }
        if let Ok(guard) = feed().consumer.lock() {
            if let Some(consumer) = guard.as_ref() {
                let _ = consumer.fill(dst);
                return;
            }
        }
        dst.fill(0.0);
    }
}

#[cfg(target_os = "macos")]
#[allow(clippy::needless_pass_by_value)] // moved onto the receive thread
fn mach_recv_loop(producer: SampleProducer, stop: Arc<AtomicBool>) {
    use minimix_ipc_mac::mach::PluginServer;
    use std::time::Duration;

    let server = loop {
        if stop.load(Ordering::Acquire) {
            return;
        }
        match PluginServer::bind() {
            Ok(server) => break server,
            Err(_) => thread::sleep(Duration::from_millis(250)),
        }
    };
    let _ = SERVICE_NAME;

    while !stop.load(Ordering::Acquire) {
        if let Ok((_header, samples)) = server.recv_packet(200) {
            let _ = producer.push_slice(&samples);
        }
    }
}

#[cfg(not(target_os = "macos"))]
#[allow(clippy::needless_pass_by_value)] // matches macOS signature; Arc moved onto the receive thread
fn mach_recv_loop(producer: SampleProducer, stop: Arc<AtomicBool>) {
    let _ = (producer, SERVICE_NAME);
    while !stop.load(Ordering::Acquire) {
        thread::sleep(std::time::Duration::from_millis(200));
    }
}

impl Drop for MinimixDriver {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(handle) = self.worker.take() {
            let _ = handle.join();
        }
    }
}

tympan_aspl::plugin_entry!(MinimixDriver);

#[cfg(test)]
mod tests {
    use super::*;
    use tympan_aspl::driver::DriverInstance;
    use tympan_aspl::io::{IoOperation, Timestamp};

    fn rt() -> RealtimeContext {
        // SAFETY: unit-test witness; no realtime HAL thread.
        unsafe { RealtimeContext::new_unchecked() }
    }

    #[test]
    fn device_is_mono_input() {
        let driver = MinimixDriver::new();
        let spec = driver.device();
        assert_eq!(spec.uid(), DEVICE_UID);
        assert!(spec.input().is_some());
        assert!(spec.output().is_none());
        assert_eq!(spec.input().unwrap().channels(), CHANNELS);
    }

    #[test]
    fn process_io_writes_from_ring() {
        let (tx, rx) = SampleRing::channel(16);
        {
            let mut guard = feed().consumer.lock().unwrap();
            *guard = Some(rx);
        }
        assert!(tx.try_push(0.25));
        assert!(tx.try_push(-0.5));

        let mut driver = MinimixDriver::new();
        let input: [f32; 0] = [];
        let mut output = [0.0_f32; 4];
        let mut buffer = IoBuffer::new(
            Timestamp::ZERO,
            IoOperation::READ_INPUT,
            &input,
            &mut output,
        );
        driver.process_io(&rt(), &mut buffer);
        assert_eq!(output[0], 0.25);
        assert_eq!(output[1], -0.5);
        assert_eq!(output[2], 0.0);
        assert_eq!(output[3], 0.0);
    }

    #[test]
    fn committed_info_plist_matches_generator() {
        let generated = tympan_aspl::bundle::plist::generate(&MinimixDriver::bundle_config())
            .replace("\r\n", "\n");
        let committed = include_str!("../Info.plist").replace("\r\n", "\n");
        assert_eq!(generated, committed);
    }

    #[test]
    fn lifecycle_reaches_running() {
        let d = DriverInstance::<MinimixDriver>::new();
        d.initialize().unwrap();
        d.start_io().unwrap();
        d.stop_io().unwrap();
    }
}
