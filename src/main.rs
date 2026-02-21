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

use clap::Parser;

use crate::config::Config;

#[derive(Parser)]
#[command(name = "clisten", about = "NTS Radio TUI player")]
struct Cli {}

fn check_dependencies() -> anyhow::Result<()> {
    match which::which("mpv") {
        Err(_) => {
            eprintln!("Error: mpv is required but not found. Install with: brew install mpv");
            std::process::exit(1);
        }
        Ok(_) => {}
    }

    if which::which("yt-dlp").is_err() {
        eprintln!("Warning: yt-dlp not found. Some playback may not work.");
        eprintln!("Install with: brew install yt-dlp");
    }

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _cli = Cli::parse();

    check_dependencies()?;

    let config = Config::load().unwrap_or_default();
    logging::init(&config)?;

    let mut app = app::App::new(config)?;
    app.run().await?;

    Ok(())
}
