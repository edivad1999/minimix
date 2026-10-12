//! Userspace `WaveRT` staging-ring protocol and feed helpers for Minimix.
//!
//! The kernel driver under `platforms/windows/wave-rt/` is still a scaffold
//! (no `.sys`). This crate documents the private IOCTL framing, provides an
//! in-process staging ring that inserts silence on underrun, and (with the
//! `win-fake-ring` feature) exposes that ring for CI.

mod feed;
mod protocol;
mod ring;
mod session;

pub use feed::{open_device_ring, open_fake_ring, FakeRingFeed, RingFeed, WaveRtError};
pub use protocol::{
    RingCommitRequest, RingHeader, RingOpenRequest, RingOpenResponse, RingQueryResponse,
    FILE_DEVICE_MINIMIX, IOCTL_MINIMIX_RING_COMMIT, IOCTL_MINIMIX_RING_OPEN,
    IOCTL_MINIMIX_RING_QUERY, RING_MAGIC, RING_VERSION, SAMPLE_FORMAT_F32_MONO,
};
pub use ring::{PullResult, StagingRing};
pub use session::{start_feed, FeedSession, FeedStats};

#[cfg(all(test, feature = "win-fake-ring"))]
mod fake_ring_tests {
    use super::*;

    #[test]
    fn fake_ring_commit_visible_to_pull() {
        let feed = open_fake_ring(48_000, 32).expect("fake ring");
        assert_eq!(feed.commit(&[0.25, -0.5, 0.75]).unwrap(), 3);
        let mut dst = [0.0_f32; 3];
        let pull = feed.ring().pull(&mut dst);
        assert_eq!(pull.from_ring, 3);
        assert_eq!(pull.silence, 0);
        assert_eq!(dst, [0.25, -0.5, 0.75]);
        let q = feed.query();
        assert_eq!(q.write_cursor, 3);
        assert_eq!(q.read_cursor, 3);
        assert_eq!(q.underrun_frames, 0);
    }

    #[test]
    fn fake_ring_underrun_is_silence() {
        let feed = open_fake_ring(48_000, 16).expect("fake ring");
        assert_eq!(feed.commit(&[1.0]).unwrap(), 1);
        let mut dst = [7.0_f32; 4];
        let pull = feed.ring().pull(&mut dst);
        assert_eq!(pull.from_ring, 1);
        assert_eq!(pull.silence, 3);
        assert_eq!(dst, [1.0, 0.0, 0.0, 0.0]);
        assert_eq!(feed.query().underrun_frames, 3);
    }

    #[test]
    fn device_open_reports_missing_sys() {
        let err = open_device_ring().expect_err("no driver");
        assert!(matches!(err, WaveRtError::DriverNotLoaded(_)));
    }
}
