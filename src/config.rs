// User configuration loaded from ~/.config/clisten/config.toml.
// Falls back to sensible defaults when the file is missing.

use serde::Deserialize;
use std::path::PathBuf;

/// Application configuration, deserialized from `~/.config/clisten/config.toml`.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub general: GeneralConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GeneralConfig {
    /// Target TUI refresh rate in frames per second (default: 30).
    #[serde(default = "default_frame_rate")]
    pub frame_rate: f64,
}

fn default_frame_rate() -> f64 {
    30.0
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            frame_rate: default_frame_rate(),
        }
    }
}

impl Config {
    /// Read config from disk, or return defaults if the file doesn't exist.
    pub fn load() -> anyhow::Result<Self> {
        let config_path = Self::config_path();
        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            let config: Config = toml::from_str(&content)?;
            Ok(config)
        } else {
            Ok(Config::default())
        }
    }

    pub fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("clisten")
            .join("config.toml")
    }
}
