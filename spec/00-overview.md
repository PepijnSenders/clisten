# clisten — Complete Implementation Specification

> NTS Radio & SoundCloud TUI Player — Rust

## 1. Project Overview

**clisten** is a terminal user interface (TUI) application for discovering and playing music from NTS Radio and SoundCloud. It combines browsing (live streams, curated collections, shows, search) with playback (via mpv) and personal library features (favorites, history) in a multi-panel terminal interface.

### External Dependencies

| Dependency | Purpose | Install |
|---|---|---|
| `mpv` | Audio playback engine | `brew install mpv` |
| `yt-dlp` | SoundCloud/Mixcloud URL resolution (used by mpv internally) | `brew install yt-dlp` |

### Target Platform

- macOS (primary), Linux (should work)
- Rust stable toolchain
- Terminal with 256-color support (kitty, iTerm2, Alacritty, etc.)

---

## 2. Layout Wireframe

```
┌──────────────────────────────────────┬──────────────────┐
│  [NTS] [SoundCloud]                  │  Now Playing      │
│  ─────────────────────               │                   │
│  Live | Picks | Recent | Mixtapes |  │  Show name        │
│  Shows | Schedule | Favs | History   │  Artist           │
│                                      │  ■ LIVE / 12:34   │
│  > Channel 1: Show Name       ♥     │  Tags: ambient,   │
│    Channel 2: Other Show             │  drone, field     │
│    Episode Three                     │                   │
│    ...                               │  Tracklist:       │
│                                      │  1. Track One     │
│  [/ Search...]                       │  2. Track Two     │
├──────────────────────────────────────┴──────────────────┤
│  ▶ Space Play/Pause  n Next  p Prev  f Fav  / Search   │
│    Tab Switch  q Quit  ? Help            Track 2/5      │
└─────────────────────────────────────────────────────────┘
```

### Panel Breakdown

| Region | Widget | Constraint |
|---|---|---|
| Top-left | `Tabs` — primary source tabs (NTS / SoundCloud) | `Min(0)` in horizontal split, 60% |
| Sub-tabs | `Tabs` — view selector within NTS or SC | Part of left panel |
| Left body | `List` — scrollable discovery items | Fills remaining left-panel space |
| Left bottom | `Paragraph` — search input bar | `Length(3)` |
| Right panel | `Paragraph` — now playing info | 40% of horizontal split |
| Bottom bar | `Paragraph` — play controls + keybinding hints | `Length(3)` |

### Layout Code Structure

```rust
// Outer layout: main area + bottom bar
let outer = Layout::vertical([
    Constraint::Min(0),      // main content
    Constraint::Length(3),   // play controls bar
]);

// Main area: left panel + right panel
let main = Layout::horizontal([
    Constraint::Percentage(60),  // discovery panel
    Constraint::Percentage(40),  // now playing panel
]);

// Left panel: tabs + sub-tabs + list + search
let left = Layout::vertical([
    Constraint::Length(1),   // primary tabs
    Constraint::Length(1),   // sub-tabs
    Constraint::Min(0),      // discovery list
    Constraint::Length(3),   // search bar
]);
```

---

## 3. Full Project Structure

```
~/projects/clisten/
├── Cargo.toml
├── migrations/
│   └── 001_init.sql
├── src/
│   ├── main.rs                  # CLI parsing (clap), bootstrap, dep checks
│   ├── app.rs                   # Event loop, layout, action dispatch
│   ├── action.rs                # Action enum (all app-wide messages)
│   ├── tui.rs                   # Terminal wrapper (crossterm + ratatui event stream)
│   ├── config.rs                # Config loading (~/.config/clisten/config.toml)
│   ├── errors.rs                # color-eyre setup
│   ├── logging.rs               # tracing to file (~/.local/share/clisten/clisten.log)
│   ├── theme.rs                 # Color palette, style constants
│   ├── db.rs                    # SQLite wrapper (rusqlite): favorites, history
│   ├── player/
│   │   ├── mod.rs               # MpvPlayer: spawn mpv, IPC via --input-ipc-server
│   │   └── queue.rs             # Queue: ordered list, next/prev, add/remove
│   ├── api/
│   │   ├── mod.rs               # Shared reqwest client
│   │   ├── nts.rs               # NTS API client (all endpoints)
│   │   ├── soundcloud.rs        # SC API client (search, resolve, likes)
│   │   └── models.rs            # Serde response types + DiscoveryItem enum
│   └── components/
│       ├── mod.rs               # Component trait definition
│       ├── tabs.rs              # [NTS] [SoundCloud] primary tab bar
│       ├── discovery_list.rs    # Left panel: scrollable item list
│       ├── search_bar.rs        # Bottom-of-left-panel search input
│       ├── now_playing.rs       # Right panel: track info, tracklist, tags
│       ├── play_controls.rs     # Bottom bar: transport controls + shortcut hints
│       ├── nts/
│       │   ├── mod.rs           # NTS tab coordinator (sub-tabs + state)
│       │   ├── live.rs          # Live Now view (channels 1 & 2)
│       │   ├── collections.rs   # NTS Picks / Recently Added views
│       │   ├── shows.rs         # Shows A-Z browser with drill-down
│       │   ├── mixtapes.rs      # 18 Infinite Mixtapes
│       │   └── schedule.rs      # Upcoming broadcasts
│       └── soundcloud/
│           ├── mod.rs           # SoundCloud tab coordinator
│           ├── search.rs        # Search results view
│           └── favorites.rs     # User likes (requires auth)
└── spec.md                      # This file
```

---

## 4. Cargo.toml

```toml
[package]
name = "clisten"
version = "0.1.0"
edition = "2021"
description = "NTS Radio & SoundCloud TUI player"

[dependencies]
# TUI
ratatui = { version = "0.30", features = ["crossterm"] }
crossterm = { version = "0.28", features = ["event-stream"] }

# Async runtime
tokio = { version = "1", features = ["full"] }
futures = "0.3"

# HTTP
reqwest = { version = "0.12", features = ["json", "rustls-tls"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
toml = "0.8"

# Database
rusqlite = { version = "0.31", features = ["bundled"] }

# CLI
clap = { version = "4.0", features = ["derive"] }

# Error handling
anyhow = "1.0"
color-eyre = "0.6"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
tracing-appender = "0.2"

# Utilities
chrono = { version = "0.4", features = ["serde"] }
dirs = "5"
open = "5"
regex = "1"
strum = { version = "0.26", features = ["derive"] }
which = "7"
```

---


## Appendix: Remaining Module Stubs

### errors.rs

```rust
// src/errors.rs

pub fn init() -> anyhow::Result<()> {
    color_eyre::install()?;
    Ok(())
}
```

### logging.rs

```rust
// src/logging.rs

use crate::config::Config;
use tracing_appender::rolling;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

pub fn init(_config: &Config) -> anyhow::Result<()> {
    let data_dir = dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("clisten");
    std::fs::create_dir_all(&data_dir)?;

    let file_appender = rolling::never(&data_dir, "clisten.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::registry()
        .with(
            fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false)
        )
        .with(EnvFilter::from_default_env().add_directive("clisten=debug".parse()?))
        .init();

    // Leak the guard so the appender stays alive for the lifetime of the program
    std::mem::forget(_guard);
    Ok(())
}
```

### theme.rs

```rust
// src/theme.rs

use ratatui::style::{Color, Modifier, Style};

pub struct Theme;

impl Theme {
    pub fn title() -> Style {
        Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
    }

    pub fn subtitle() -> Style {
        Style::default().fg(Color::DarkGray)
    }

    pub fn highlight() -> Style {
        Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD)
    }

    pub fn active_tab() -> Style {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    }

    pub fn inactive_tab() -> Style {
        Style::default().fg(Color::DarkGray)
    }

    pub fn favorite() -> Style {
        Style::default().fg(Color::Red)
    }

    pub fn playing() -> Style {
        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
    }

    pub fn error() -> Style {
        Style::default().fg(Color::Red)
    }

    pub fn search_focused() -> Style {
        Style::default().fg(Color::Yellow)
    }

    pub fn search_unfocused() -> Style {
        Style::default().fg(Color::DarkGray)
    }
}
```

### api/mod.rs

```rust
// src/api/mod.rs

pub mod models;
pub mod nts;
pub mod soundcloud;
```
