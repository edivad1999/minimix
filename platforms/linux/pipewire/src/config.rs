//! Virtual mic identity and stream settings.

/// Settings for the capture stream and published `Audio/Source`.
#[derive(Debug, Clone)]
pub struct VirtualMicConfig {
    /// Negotiated sample rate for both streams.
    pub sample_rate: u32,
    /// Channel count. Mono (`1`) is the supported path.
    pub channels: u32,
    /// `PipeWire` `node.name` (also the Pulse source name).
    pub node_name: String,
    /// Friendly name shown in Discord / OBS pickers.
    pub media_name: String,
    /// Optional `target.object` for the capture stream.
    pub capture_target: Option<String>,
    /// Shared `node.link-group` for capture and source.
    pub link_group: String,
    /// Low `priority.session` so Minimix does not become the default mic.
    pub priority_session: i32,
}

impl Default for VirtualMicConfig {
    fn default() -> Self {
        Self {
            sample_rate: 48_000,
            channels: 1,
            node_name: "minimix-mic".into(),
            media_name: "Minimix".into(),
            capture_target: None,
            link_group: "minimix".into(),
            priority_session: 0,
        }
    }
}
