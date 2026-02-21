// src/player/mod.rs

pub mod queue;

use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::process::{Child, Command};
use tokio::sync::mpsc;

use crate::action::Action;

#[derive(Clone)]
pub struct MpvPlayer {
    pub socket_path: PathBuf,
    action_tx: Option<mpsc::UnboundedSender<Action>>,
    child: Option<std::sync::Arc<tokio::sync::Mutex<Option<Child>>>>,
}

impl MpvPlayer {
    pub fn new() -> Self {
        let pid = std::process::id();
        Self {
            socket_path: PathBuf::from(format!("/tmp/clisten-mpv-{}.sock", pid)),
            action_tx: None,
            child: Some(std::sync::Arc::new(tokio::sync::Mutex::new(None))),
        }
    }

    #[allow(dead_code)]
    pub fn set_action_tx(&mut self, tx: mpsc::UnboundedSender<Action>) {
        self.action_tx = Some(tx);
    }

    /// Spawn mpv with IPC socket for the given URL.
    pub async fn play(&mut self, url: &str) -> anyhow::Result<()> {
        // Signal loading state
        if let Some(tx) = &self.action_tx {
            tx.send(Action::PlaybackLoading).ok();
        }

        // Kill existing mpv if running
        self.stop().await?;

        // Remove stale socket
        let _ = std::fs::remove_file(&self.socket_path);

        let child = Command::new("mpv")
            .arg("--no-video")
            .arg("--no-terminal")
            .arg(format!("--input-ipc-server={}", self.socket_path.display()))
            .arg(url)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;

        let child_arc = self.child.clone().unwrap();
        *child_arc.lock().await = Some(child);

        // Monitor mpv process exit
        let tx = self.action_tx.clone();
        let arc = child_arc.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                let mut guard = arc.lock().await;
                if let Some(ref mut child) = *guard {
                    match child.try_wait() {
                        Ok(Some(_status)) => {
                            *guard = None;
                            if let Some(tx) = &tx {
                                tx.send(Action::PlaybackFinished).ok();
                            }
                            break;
                        }
                        Ok(None) => {} // still running
                        Err(_) => break,
                    }
                } else {
                    break;
                }
            }
        });

        // Start position polling
        let socket_path = self.socket_path.clone();
        let tx = self.action_tx.clone();
        tokio::spawn(async move {
            // Wait for socket to appear
            for _ in 0..20 {
                if socket_path.exists() { break; }
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }

            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                match Self::send_command_static(&socket_path, r#"{"command":["get_property","playback-time"]}"#).await {
                    Ok(response) => {
                        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&response) {
                            if let Some(pos) = val.get("data").and_then(|d| d.as_f64()) {
                                if let Some(tx) = &tx {
                                    tx.send(Action::PlaybackPosition(pos)).ok();
                                }
                            }
                        }
                    }
                    Err(_) => break, // socket gone, mpv exited
                }
            }
        });

        // Start media-title observation for stream metadata (ICY tags)
        let socket_path = self.socket_path.clone();
        let tx = self.action_tx.clone();
        let url_owned = url.to_string();
        tokio::spawn(async move {
            // Wait for socket to appear
            for _ in 0..20 {
                if socket_path.exists() { break; }
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }

            let Ok(stream) = UnixStream::connect(&socket_path).await else { return; };
            let (reader, mut writer) = stream.into_split();
            let mut lines = BufReader::new(reader).lines();

            // Observe media-title property
            let cmd = r#"{"command":["observe_property",1,"media-title"]}"#;
            if writer.write_all(format!("{}\n", cmd).as_bytes()).await.is_err() {
                return;
            }

            while let Ok(Some(line)) = lines.next_line().await {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&line) {
                    if val.get("event").and_then(|e| e.as_str()) == Some("property-change")
                        && val.get("name").and_then(|n| n.as_str()) == Some("media-title")
                    {
                        if let Some(title) = val.get("data").and_then(|d| d.as_str()) {
                            let title = title.trim().to_string();
                            // Skip empty strings, raw URLs, and junk values
                            if !title.is_empty()
                                && title != "stream"
                                && !title.starts_with("http://")
                                && !title.starts_with("https://")
                                && title != url_owned
                            {
                                if let Some(tx) = &tx {
                                    tx.send(Action::StreamMetadataChanged(title)).ok();
                                }
                            }
                        }
                    }
                }
            }
        });

        Ok(())
    }

    /// Toggle pause via IPC.
    pub async fn toggle_pause(&self) -> anyhow::Result<()> {
        self.send_command(r#"{"command":["cycle","pause"]}"#).await?;
        Ok(())
    }

    /// Stop playback by quitting mpv.
    pub async fn stop(&self) -> anyhow::Result<()> {
        let _ = self.send_command(r#"{"command":["quit"]}"#).await;
        let _ = std::fs::remove_file(&self.socket_path);
        if let Some(arc) = &self.child {
            let mut guard = arc.lock().await;
            if let Some(ref mut child) = *guard {
                let _ = child.kill().await;
            }
            *guard = None;
        }
        Ok(())
    }

    /// Seek by relative seconds.
    #[allow(dead_code)]
    pub async fn seek(&self, secs: f64) -> anyhow::Result<()> {
        self.send_command(&format!(
            r#"{{"command":["seek","{}","relative"]}}"#, secs
        )).await?;
        Ok(())
    }

    /// Adjust volume by delta (positive = up, negative = down), clamped to 0-100.
    pub async fn set_volume(&self, delta: f64) -> anyhow::Result<()> {
        let current = self.get_volume().await.unwrap_or(50.0);
        let target = (current + delta).clamp(0.0, 100.0);
        self.send_command(&format!(
            r#"{{"command":["set_property","volume",{}]}}"#, target
        )).await?;
        Ok(())
    }

    /// Get current volume (0-100).
    pub async fn get_volume(&self) -> anyhow::Result<f64> {
        let response = self.send_command(
            r#"{"command":["get_property","volume"]}"#
        ).await?;
        let val: serde_json::Value = serde_json::from_str(&response)?;
        val.get("data")
            .and_then(|d| d.as_f64())
            .ok_or_else(|| anyhow::anyhow!("No volume data"))
    }

    /// Get current playback position.
    #[allow(dead_code)]
    pub async fn get_position(&self) -> anyhow::Result<f64> {
        let response = self.send_command(
            r#"{"command":["get_property","playback-time"]}"#
        ).await?;
        let val: serde_json::Value = serde_json::from_str(&response)?;
        val.get("data")
            .and_then(|d| d.as_f64())
            .ok_or_else(|| anyhow::anyhow!("No position data"))
    }

    async fn send_command(&self, cmd: &str) -> anyhow::Result<String> {
        Self::send_command_static(&self.socket_path, cmd).await
    }

    async fn send_command_static(socket_path: &PathBuf, cmd: &str) -> anyhow::Result<String> {
        let mut stream = UnixStream::connect(socket_path).await?;
        let msg = format!("{}\n", cmd);
        stream.write_all(msg.as_bytes()).await?;
        let mut reader = BufReader::new(stream);
        let mut response = String::new();
        reader.read_line(&mut response).await?;
        Ok(response)
    }
}

impl Drop for MpvPlayer {
    fn drop(&mut self) {
        // Kill the mpv process if still running
        if let Some(arc) = &self.child {
            if let Ok(mut guard) = arc.try_lock() {
                if let Some(ref mut child) = *guard {
                    let _ = child.start_kill();
                }
                *guard = None;
            }
        }
        let _ = std::fs::remove_file(&self.socket_path);
    }
}
