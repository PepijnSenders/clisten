// Entry point: checks runtime deps (mpv, yt-dlp), loads config, and runs the TUI.

mod action;
mod api;
mod app;
mod components;
mod config;
mod db;
mod logging;
mod player;
mod theme;
mod tui;
mod ui;

use crate::config::Config;

/// Kill mpv instances left behind by previous clisten sessions.
/// Scans the temp dir for stale `clisten-mpv-*.sock` files and sends quit via IPC.
async fn kill_orphaned_mpv() {
    let tmp = std::env::temp_dir();
    let own_socket = format!("clisten-mpv-{}.sock", std::process::id());

    let Ok(entries) = std::fs::read_dir(&tmp) else {
        return;
    };

    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name_str) = name.to_str() else {
            continue;
        };
        if !name_str.starts_with("clisten-mpv-") || !name_str.ends_with(".sock") {
            continue;
        }
        // Don't kill our own socket
        if name_str == own_socket {
            continue;
        }
        let path = entry.path();
        // Best-effort quit + cleanup
        let _ = player::ipc::send_command(&path, r#"{"command":["quit"]}"#).await;
        let _ = std::fs::remove_file(&path);
    }
}

fn check_dependencies() {
    if which::which("mpv").is_err() {
        eprintln!("Error: mpv is required but not found. Install with: brew install mpv");
        std::process::exit(1);
    }
    if which::which("yt-dlp").is_err() {
        eprintln!("Warning: yt-dlp not found. Some playback may not work.");
        eprintln!("Install with: brew install yt-dlp");
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if std::env::args().any(|a| a == "--version" || a == "-V") {
        println!("clisten {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    check_dependencies();
    kill_orphaned_mpv().await;

    let config = Config::load().unwrap_or_else(|e| {
        eprintln!("Warning: failed to load config: {e}. Using defaults.");
        Config::default()
    });
    let _log_guard = logging::init()?;

    let pending = config.pending_onboarding_screens();
    let mut app = app::App::new(config)?;
    if !pending.is_empty() {
        app.onboarding.activate(pending);
    }
    app.run().await?;

    Ok(())
}
