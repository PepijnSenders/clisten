# clisten

A terminal UI for discovering and playing music from [NTS Radio](https://www.nts.live). Browse live streams, curated picks, and search by genre — all from your terminal.

Built with Rust, [ratatui](https://github.com/ratatui/ratatui), and [mpv](https://mpv.io).

![clisten screenshot](screenshot.png)

## Features

- **Live streams** — tune into NTS channels 1 & 2 in real-time
- **Curated picks** — browse NTS editorial selections
- **Genre search** — explore 120+ genres, server-side filtered
- **Queue management** — build playlists, reorder, play next
- **Favorites & history** — saved locally in SQLite
- **Direct URL playback** — paste any stream URL to play
- **mpv backend** — robust audio playback via IPC

## Requirements

| Dependency | Purpose | Install |
|---|---|---|
| [mpv](https://mpv.io) | Audio playback | `brew install mpv` |
| [yt-dlp](https://github.com/yt-dlp/yt-dlp) | URL resolution (used by mpv) | `brew install yt-dlp` |

## Install

```sh
git clone https://github.com/PepijnSenders/clisten.git
cd clisten
cargo build --release
```

The binary will be at `target/release/clisten`.

## Usage

```sh
cargo run
# or after building:
./target/release/clisten
```

## Keybindings

| Key | Action |
|---|---|
| `Space` | Play / Pause |
| `Enter` | Play selected item |
| `n` / `p` | Next / Previous track |
| `s` | Stop playback |
| `a` | Add to queue |
| `A` | Add to queue (play next) |
| `c` | Clear queue |
| `f` | Toggle favorite |
| `Tab` / `Shift+Tab` | Cycle sub-tabs |
| `1` `2` `3` | Jump to Live / Picks / Search |
| `/` | Focus search bar |
| `o` | Open direct URL player |
| `[` / `]` | Volume down / up |
| `Esc` | Back / unfocus |
| `?` | Help |
| `q` | Quit |

## License

MIT
