// mpv wrapper: spawns mpv with an IPC socket for play/pause/stop/volume.
// Low-level IPC communication and background pollers live in the ipc submodule.

pub mod ipc;
pub mod queue;

use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;
use tokio::sync::mpsc;

use crate::action::Action;
use anyhow::Context;
use ipc::MpvProcess;

/// Metadata gleaned from an active stream (ICY headers, ID3 tags, etc.).
#[derive(Debug, Clone, Default)]
pub struct StreamMetadata {
    pub station_name: Option<String>, // icy-name
    pub title: Option<String>,        // media-title / ICY track
    pub artist: Option<String>,       // ID3/vorbis artist
    pub album: Option<String>,        // ID3/vorbis album
}

impl StreamMetadata {
    /// "Artist - Title", or just title, or just artist, or None.
    pub fn display_title(&self) -> Option<String> {
        match (&self.artist, &self.title) {
            (Some(a), Some(t)) => Some(format!("{} - {}", a, t)),
            (None, Some(t)) => Some(t.clone()),
            (Some(a), None) => Some(a.clone()),
            (None, None) => None,
        }
    }

    /// Station name, falling back to album.
    pub fn display_subtitle(&self) -> Option<String> {
        self.station_name.clone().or_else(|| self.album.clone())
    }

    /// True when no metadata fields have been populated.
    pub fn is_empty(&self) -> bool {
        self.station_name.is_none()
            && self.title.is_none()
            && self.artist.is_none()
            && self.album.is_none()
    }
}

/// Wraps an mpv child process, communicating over a Unix IPC socket.
#[derive(Clone)]
pub struct MpvPlayer {
    socket_path: PathBuf,
    action_tx: Option<mpsc::UnboundedSender<Action>>,
    child: MpvProcess,
}

impl Default for MpvPlayer {
    fn default() -> Self {
        let pid = std::process::id();
        Self {
            socket_path: std::env::temp_dir().join(format!("clisten-mpv-{}.sock", pid)),
            action_tx: None,
            child: std::sync::Arc::new(tokio::sync::Mutex::new(None)),
        }
    }
}

impl MpvPlayer {
    pub fn new() -> Self {
        Self::default()
    }

    #[allow(dead_code)] // used by integration tests
    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }

    pub fn set_action_tx(&mut self, tx: mpsc::UnboundedSender<Action>) {
        self.action_tx = Some(tx);
    }

    /// Spawn mpv with IPC socket for the given URL.
    pub async fn play(&mut self, url: &str) -> anyhow::Result<()> {
        let tx = self
            .action_tx
            .clone()
            .expect("action_tx must be set before play()");

        tx.send(Action::PlaybackLoading).ok();
        self.stop().await?;
        // Remove stale socket from a previous mpv instance, if any.
        let _ = std::fs::remove_file(&self.socket_path);

        let child = Command::new("mpv")
            .arg("--no-video")
            .arg("--no-terminal")
            .arg(format!("--input-ipc-server={}", self.socket_path.display()))
            .arg("--af=@astats:lavfi=[astats=metadata=1:reset=1:measure_perchannel=none:measure_overall=RMS_level+Peak_level]")
            .arg(url)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .context("failed to spawn mpv — is it installed?")?;

        *self.child.lock().await = Some(child);

        ipc::spawn_exit_monitor(self.child.clone(), tx.clone());
        ipc::spawn_position_poller(self.socket_path.clone(), tx.clone());
        ipc::spawn_duration_poller(self.socket_path.clone(), tx.clone());
        ipc::spawn_metadata_observer(self.socket_path.clone(), tx.clone(), url.to_string());
        ipc::spawn_audio_level_poller(self.socket_path.clone(), tx);

        Ok(())
    }

    /// Seek by the given number of seconds (negative = backward).
    pub async fn seek_relative(&self, seconds: f64) -> anyhow::Result<()> {
        ipc::send_command(
            &self.socket_path,
            &format!(r#"{{"command":["seek",{},"relative"]}}"#, seconds),
        )
        .await?;
        Ok(())
    }

    /// Toggle pause on the running mpv instance.
    pub async fn toggle_pause(&self) -> anyhow::Result<()> {
        ipc::send_command(&self.socket_path, r#"{"command":["cycle","pause"]}"#).await?;
        Ok(())
    }

    /// Quit mpv and clean up the IPC socket.
    pub async fn stop(&self) -> anyhow::Result<()> {
        // Best-effort shutdown: mpv may have already exited or the socket may
        // not exist. Errors are harmless — we just need to ensure cleanup.
        let _ = ipc::send_command(&self.socket_path, r#"{"command":["quit"]}"#).await;
        let _ = std::fs::remove_file(&self.socket_path);
        let mut guard = self.child.lock().await;
        if let Some(ref mut child) = *guard {
            let _ = child.kill().await;
        }
        *guard = None;
        Ok(())
    }

    /// Adjust volume by delta (positive = up, negative = down), clamped to 0-100.
    pub async fn set_volume(&self, delta: f64) -> anyhow::Result<()> {
        let current = self.get_volume().await.unwrap_or(50.0);
        let target = (current + delta).clamp(0.0, 100.0);
        ipc::send_command(
            &self.socket_path,
            &format!(r#"{{"command":["set_property","volume",{}]}}"#, target),
        )
        .await?;
        Ok(())
    }

    /// Read the current volume level from mpv.
    pub async fn get_volume(&self) -> anyhow::Result<f64> {
        let response = ipc::send_command(
            &self.socket_path,
            r#"{"command":["get_property","volume"]}"#,
        )
        .await?;
        let val: serde_json::Value = serde_json::from_str(&response)?;
        val.get("data")
            .and_then(|d| d.as_f64())
            .ok_or_else(|| anyhow::anyhow!("No volume data"))
    }
}

impl Drop for MpvPlayer {
    fn drop(&mut self) {
        // Best-effort cleanup on drop — try_lock because we can't await.
        if let Ok(mut guard) = self.child.try_lock() {
            if let Some(ref mut child) = *guard {
                let _ = child.start_kill();
            }
            *guard = None;
        }
        let _ = std::fs::remove_file(&self.socket_path);
    }
}
