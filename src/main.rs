// Entry point: checks runtime deps (mpv, yt-dlp), loads config, and runs the TUI.

mod action;
mod api;
mod app;
mod components;
mod config;
mod db;
mod logging;
mod player;
mod tui;
mod ui;

use crate::config::Config;

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

    let config = Config::load().unwrap_or_else(|e| {
        eprintln!("Warning: failed to load config: {e}. Using defaults.");
        Config::default()
    });
    logging::init()?;

    let mut app = app::App::new(config)?;
    app.run().await?;

    Ok(())
}
