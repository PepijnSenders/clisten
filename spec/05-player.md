
## 9. Player Module

### 9.1 MpvPlayer with IPC Socket

```rust
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
    socket_path: PathBuf,
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

    pub fn set_action_tx(&mut self, tx: mpsc::UnboundedSender<Action>) {
        self.action_tx = Some(tx);
    }

    /// Spawn mpv with IPC socket for the given URL.
    pub async fn play(&mut self, url: &str) -> anyhow::Result<()> {
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
    pub async fn seek(&self, secs: f64) -> anyhow::Result<()> {
        self.send_command(&format!(
            r#"{{"command":["seek","{}","relative"]}}"#, secs
        )).await?;
        Ok(())
    }

    /// Get current playback position.
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
        let _ = std::fs::remove_file(&self.socket_path);
        // Note: child process cleanup happens via the Arc<Mutex<Child>>
    }
}
```

### 9.2 Queue

```rust
// src/player/queue.rs

use crate::api::models::DiscoveryItem;

#[derive(Debug, Clone)]
pub struct QueueItem {
    pub item: DiscoveryItem,
    pub url: String,
}

pub struct Queue {
    items: Vec<QueueItem>,
    current_index: Option<usize>,
}

impl Queue {
    pub fn new() -> Self {
        Self { items: vec![], current_index: None }
    }

    /// Add item to end of queue.
    pub fn add(&mut self, item: QueueItem) {
        self.items.push(item);
        if self.current_index.is_none() {
            self.current_index = Some(0);
        }
    }

    /// Insert item right after current position.
    pub fn add_next(&mut self, item: QueueItem) {
        let pos = self.current_index.map_or(0, |i| i + 1);
        self.items.insert(pos, item);
        if self.current_index.is_none() {
            self.current_index = Some(0);
        }
    }

    /// Remove item at index.
    pub fn remove(&mut self, index: usize) {
        if index < self.items.len() {
            self.items.remove(index);
            if self.items.is_empty() {
                self.current_index = None;
            } else if let Some(curr) = self.current_index {
                if index <= curr && curr > 0 {
                    self.current_index = Some(curr - 1);
                }
            }
        }
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.current_index = None;
    }

    pub fn current(&self) -> Option<&QueueItem> {
        self.current_index.and_then(|i| self.items.get(i))
    }

    /// Advance to next track. Returns the next item if available.
    pub fn next(&mut self) -> Option<&QueueItem> {
        if let Some(i) = self.current_index {
            if i + 1 < self.items.len() {
                self.current_index = Some(i + 1);
                return self.items.get(i + 1);
            }
        }
        None
    }

    /// Go back to previous track.
    pub fn prev(&mut self) -> Option<&QueueItem> {
        if let Some(i) = self.current_index {
            if i > 0 {
                self.current_index = Some(i - 1);
                return self.items.get(i - 1);
            }
        }
        None
    }

    pub fn items(&self) -> &[QueueItem] {
        &self.items
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn current_index(&self) -> Option<usize> {
        self.current_index
    }
}
```

---

