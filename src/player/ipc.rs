// Low-level mpv IPC: socket communication, and background tasks for monitoring
// playback state (exit, position, metadata, audio levels).

use std::path::{Path, PathBuf};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::process::Child;
use tokio::sync::mpsc;

use super::StreamMetadata;
use crate::action::Action;

pub type MpvProcess = std::sync::Arc<tokio::sync::Mutex<Option<Child>>>;

// How long to wait for mpv's IPC socket to appear (20 * 100ms = 2s).
const SOCKET_POLL_ATTEMPTS: u32 = 20;
const SOCKET_POLL_INTERVAL_MS: u64 = 100;
// Silence floor for dB-to-linear conversion.
const SILENCE_FLOOR_DB: f64 = -60.0;

/// Wait for the IPC socket to appear on disk (up to 2 seconds).
pub async fn wait_for_socket(path: &Path) {
    for _ in 0..SOCKET_POLL_ATTEMPTS {
        if path.exists() {
            return;
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(SOCKET_POLL_INTERVAL_MS)).await;
    }
}

/// Send a single JSON command over a fresh IPC connection, return the response line.
pub async fn send_command(socket_path: &Path, cmd: &str) -> anyhow::Result<String> {
    let mut stream = UnixStream::connect(socket_path)
        .await
        .map_err(|e| anyhow::anyhow!("failed to connect to mpv IPC socket: {}", e))?;
    let msg = format!("{}\n", cmd);
    stream.write_all(msg.as_bytes()).await?;
    let mut reader = BufReader::new(stream);
    let mut response = String::new();
    reader.read_line(&mut response).await?;
    Ok(response)
}

/// Poll the child process and send PlaybackFinished when it exits.
pub fn spawn_exit_monitor(child: MpvProcess, tx: Option<mpsc::UnboundedSender<Action>>) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            let mut guard = child.lock().await;
            match guard.as_mut().and_then(|c| c.try_wait().ok()) {
                Some(Some(_)) => {
                    *guard = None;
                    if let Some(tx) = &tx {
                        tx.send(Action::PlaybackFinished).ok();
                    }
                    break;
                }
                Some(None) => {} // still running
                None => break,   // no child or wait error
            }
        }
    });
}

/// Poll playback-time once per second and forward it as PlaybackPosition.
pub fn spawn_position_poller(socket_path: PathBuf, tx: Option<mpsc::UnboundedSender<Action>>) {
    tokio::spawn(async move {
        wait_for_socket(&socket_path).await;
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            let Ok(response) = send_command(
                &socket_path,
                r#"{"command":["get_property","playback-time"]}"#,
            )
            .await
            else {
                break;
            };

            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&response) {
                if let Some(pos) = val.get("data").and_then(|d| d.as_f64()) {
                    if let Some(tx) = &tx {
                        tx.send(Action::PlaybackPosition(pos)).ok();
                    }
                }
            }
        }
    });
}

/// Filter out junk metadata values (empty, "stream", raw URLs).
fn is_junk_metadata(val: &str, url: &str) -> bool {
    let trimmed = val.trim();
    trimmed.is_empty()
        || trimmed == "stream"
        || trimmed.starts_with("http://")
        || trimmed.starts_with("https://")
        || trimmed == url
}

/// Observe multiple metadata properties from mpv (media-title, icy-name, artist, album).
pub fn spawn_metadata_observer(
    socket_path: PathBuf,
    tx: Option<mpsc::UnboundedSender<Action>>,
    url: String,
) {
    tokio::spawn(async move {
        wait_for_socket(&socket_path).await;

        let Ok(stream) = UnixStream::connect(&socket_path).await else {
            return;
        };
        let (reader, mut writer) = stream.into_split();
        let mut lines = BufReader::new(reader).lines();

        let commands = [
            r#"{"command":["observe_property",1,"media-title"]}"#,
            r#"{"command":["observe_property",2,"metadata/by-key/icy-name"]}"#,
            r#"{"command":["observe_property",3,"metadata/by-key/artist"]}"#,
            r#"{"command":["observe_property",4,"metadata/by-key/album"]}"#,
        ];
        for cmd in commands {
            if writer
                .write_all(format!("{}\n", cmd).as_bytes())
                .await
                .is_err()
            {
                return;
            }
        }

        let mut meta = StreamMetadata::default();

        while let Ok(Some(line)) = lines.next_line().await {
            let Ok(val) = serde_json::from_str::<serde_json::Value>(&line) else {
                continue;
            };
            if val.get("event").and_then(|e| e.as_str()) != Some("property-change") {
                continue;
            }

            let Some(id) = val.get("id").and_then(|i| i.as_u64()) else {
                continue;
            };
            let data_str = val
                .get("data")
                .and_then(|d| d.as_str())
                .map(|s| s.trim().to_string());

            let clean = data_str.filter(|s| !is_junk_metadata(s, &url));

            let field = match id {
                1 => &mut meta.title,
                2 => &mut meta.station_name,
                3 => &mut meta.artist,
                4 => &mut meta.album,
                _ => continue,
            };
            let changed = *field != clean;
            *field = clean;

            if changed && !meta.is_empty() {
                if let Some(tx) = &tx {
                    tx.send(Action::StreamMetadataChanged(meta.clone())).ok();
                }
            }
        }
    });
}

/// Poll audio levels at ~20 Hz via the astats lavfi filter.
pub fn spawn_audio_level_poller(socket_path: PathBuf, tx: Option<mpsc::UnboundedSender<Action>>) {
    tokio::spawn(async move {
        wait_for_socket(&socket_path).await;
        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            let Ok(response) = send_command(
                &socket_path,
                r#"{"command":["get_property","af-metadata/astats"]}"#,
            )
            .await
            else {
                break;
            };

            let Ok(val) = serde_json::from_str::<serde_json::Value>(&response) else {
                continue;
            };
            let Some(data) = val.get("data").and_then(|d| d.as_object()) else {
                continue;
            };

            let rms_db = data
                .get("lavfi.astats.Overall.RMS_level")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<f64>().ok());
            let peak_db = data
                .get("lavfi.astats.Overall.Peak_level")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<f64>().ok());

            if let (Some(rms_db), Some(peak_db)) = (rms_db, peak_db) {
                let rms = db_to_linear(rms_db);
                let peak = db_to_linear(peak_db);
                if let Some(tx) = &tx {
                    tx.send(Action::AudioLevels { rms, peak }).ok();
                }
            }
        }
    });
}

/// Convert decibels to a 0.0–1.0 linear amplitude. Silence floor at -60 dB.
fn db_to_linear(db: f64) -> f64 {
    if db <= SILENCE_FLOOR_DB {
        0.0
    } else {
        10.0_f64.powf(db / 20.0).clamp(0.0, 1.0)
    }
}
