// User configuration loaded from ~/.config/clisten/config.toml.
// Falls back to sensible defaults when the file is missing.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::components::visualizers::VisualizerKind;

/// Application configuration, deserialized from `~/.config/clisten/config.toml`.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Config {
    #[serde(default)]
    pub general: GeneralConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GeneralConfig {
    /// Target TUI refresh rate in frames per second (default: 30).
    #[serde(default = "default_frame_rate")]
    pub frame_rate: f64,

    /// Color theme: "dark" or "light" (default: "dark").
    #[serde(default = "default_theme")]
    pub theme: String,

    /// Active visualizer kind (default: Blob).
    #[serde(default)]
    pub visualizer: VisualizerKind,

    /// IDs of onboarding screens the user has already completed.
    #[serde(default)]
    pub completed_onboarding: Vec<String>,

    /// Automatically skip the ~3s NTS intro jingle on archived episodes.
    #[serde(default)]
    pub skip_nts_intro: bool,
}

fn default_frame_rate() -> f64 {
    30.0
}

fn default_theme() -> String {
    crate::theme::THEME_DARK.to_string()
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            frame_rate: default_frame_rate(),
            theme: default_theme(),
            visualizer: VisualizerKind::default(),
            completed_onboarding: Vec::new(),
            skip_nts_intro: false,
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

    /// Returns the IDs of onboarding screens that haven't been completed yet.
    pub fn pending_onboarding_screens(&self) -> Vec<&'static str> {
        use crate::components::onboarding::ALL_SCREENS;
        ALL_SCREENS
            .iter()
            .filter(|id| !self.general.completed_onboarding.contains(&id.to_string()))
            .copied()
            .collect()
    }

    /// Write current config to disk, creating parent directories if needed.
    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }
}
