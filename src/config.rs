// src/config.rs

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub keybindings: KeybindingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    #[serde(default = "default_frame_rate")]
    pub frame_rate: f64,
}

fn default_frame_rate() -> f64 { 30.0 }

impl Default for GeneralConfig {
    fn default() -> Self {
        Self { frame_rate: default_frame_rate() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeybindingConfig {
    #[serde(default = "default_quit")] pub quit: String,
    #[serde(default = "default_next_tab")] pub next_tab: String,
    #[serde(default = "default_prev_tab")] pub prev_tab: String,
    #[serde(default = "default_scroll_down")] pub scroll_down: String,
    #[serde(default = "default_scroll_up")] pub scroll_up: String,
    #[serde(default = "default_select")] pub select: String,
    #[serde(default = "default_back")] pub back: String,
    #[serde(default = "default_play_pause")] pub play_pause: String,
    #[serde(default = "default_next_track")] pub next_track: String,
    #[serde(default = "default_prev_track")] pub prev_track: String,
    #[serde(default = "default_stop")] pub stop: String,
    #[serde(default = "default_search")] pub search: String,
    #[serde(default = "default_favorite")] pub favorite: String,
    #[serde(default = "default_add_queue")] pub add_queue: String,
    #[serde(default = "default_clear_queue")] pub clear_queue: String,
    #[serde(default = "default_help")] pub help: String,
}

fn default_quit() -> String { "q".into() }
fn default_next_tab() -> String { "Tab".into() }
fn default_prev_tab() -> String { "Shift+Tab".into() }
fn default_scroll_down() -> String { "j".into() }
fn default_scroll_up() -> String { "k".into() }
fn default_select() -> String { "Enter".into() }
fn default_back() -> String { "Escape".into() }
fn default_play_pause() -> String { "Space".into() }
fn default_next_track() -> String { "n".into() }
fn default_prev_track() -> String { "p".into() }
fn default_stop() -> String { "s".into() }
fn default_search() -> String { "/".into() }
fn default_favorite() -> String { "f".into() }
fn default_add_queue() -> String { "a".into() }
fn default_clear_queue() -> String { "c".into() }
fn default_help() -> String { "?".into() }

impl Default for KeybindingConfig {
    fn default() -> Self {
        Self {
            quit: default_quit(),
            next_tab: default_next_tab(),
            prev_tab: default_prev_tab(),
            scroll_down: default_scroll_down(),
            scroll_up: default_scroll_up(),
            select: default_select(),
            back: default_back(),
            play_pause: default_play_pause(),
            next_track: default_next_track(),
            prev_track: default_prev_track(),
            stop: default_stop(),
            search: default_search(),
            favorite: default_favorite(),
            add_queue: default_add_queue(),
            clear_queue: default_clear_queue(),
            help: default_help(),
        }
    }
}

impl Config {
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

    #[allow(dead_code)]
    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            keybindings: KeybindingConfig::default(),
        }
    }
}
