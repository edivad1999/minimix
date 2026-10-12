//! Presets and persisted settings.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// One named DSP preset.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Preset {
    /// Display name.
    pub name: String,
    /// Bypass the chain.
    pub bypass: bool,
    /// Linear output gain.
    pub output_gain: f32,
    /// High-pass cutoff Hz.
    pub highpass_hz: f32,
    /// Peaking band gains in dB (three bands).
    pub peak_gain_db: [f32; 3],
    /// Compressor threshold dBFS.
    pub comp_threshold_db: f32,
    /// Compressor ratio.
    pub comp_ratio: f32,
}

impl Default for Preset {
    fn default() -> Self {
        Self {
            name: "Default".into(),
            bypass: false,
            output_gain: 1.0,
            highpass_hz: 80.0,
            peak_gain_db: [0.0, 0.0, 0.0],
            comp_threshold_db: -18.0,
            comp_ratio: 4.0,
        }
    }
}

/// On-disk settings file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Settings {
    /// Active preset.
    pub active: Preset,
    /// Saved presets.
    pub presets: Vec<Preset>,
}

impl Settings {
    /// Load JSON from `path`, or defaults if missing.
    ///
    /// # Errors
    ///
    /// Returns IO or JSON errors when the file exists but cannot be read.
    pub fn load(path: &Path) -> Result<Self, SettingsError> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&raw)?)
    }

    /// Write JSON to `path`, creating parent dirs.
    ///
    /// # Errors
    ///
    /// Returns IO or JSON errors on failure.
    pub fn save(&self, path: &Path) -> Result<(), SettingsError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let raw = serde_json::to_string_pretty(self)?;
        fs::write(path, raw)?;
        Ok(())
    }
}

/// Settings load/save failures.
#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    /// Filesystem error.
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    /// JSON error.
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

/// Default config path under the user config directory.
#[must_use]
pub fn default_config_path() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|h| {
                let mut p = PathBuf::from(h);
                p.push(".config");
                p
            })
        })
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("minimix").join("settings.json")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn round_trip_json() {
        let mut s = Settings::default();
        s.active.output_gain = 0.75;
        s.active.name = "Soft".into();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("minimix-settings-{nanos}.json"));
        s.save(&path).unwrap();
        let loaded = Settings::load(&path).unwrap();
        assert_eq!(loaded.active.output_gain, 0.75);
        assert_eq!(loaded.active.name, "Soft");
        let _ = fs::remove_file(path);
    }
}
