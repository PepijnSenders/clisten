## 12. Keybinding Map

| Key | Action | Context |
|---|---|---|
| `q` | Quit | Global (not in search mode) |
| `Tab` | Next tab (NTS ↔ SoundCloud) | Global |
| `Shift+Tab` | Previous tab | Global |
| `1`–`8` | Switch NTS sub-tab by number | NTS tab, not in search |
| `j` / `Down` | Scroll down | Discovery list |
| `k` / `Up` | Scroll up | Discovery list |
| `Enter` | Play selected / drill into show | Discovery list |
| `a` | Add to queue (without playing) | Discovery list |
| `A` (Shift+a) | Add to queue next (after current) | Discovery list |
| `Space` | Toggle play/pause | Global (not in search) |
| `n` | Next track in queue | Global (not in search) |
| `p` | Previous track in queue | Global (not in search) |
| `s` | Stop playback | Global (not in search) |
| `/` | Focus search bar | Global (not in search) |
| `Escape` | Unfocus search / go back | Global |
| `f` | Toggle favorite on highlighted item | Discovery list |
| `c` | Clear queue | Global (not in search) |
| `?` | Toggle help overlay | Global (not in search) |
| `r` | Retry failed request | Error state |

### Context Rules

1. **Search mode active**: All single-char keys go to search input. Only `Escape` and `Enter` have special behavior.
2. **Help overlay active**: Any key dismisses the overlay.
3. **Discovery list focused** (default): j/k/Enter/a/f work on the highlighted item.
4. **NTS sub-tab numbers**: `1`=Live, `2`=Picks, `3`=Recent, `4`=Mixtapes, `5`=Shows, `6`=Schedule, `7`=Favorites, `8`=History.

---

## 15. Error Handling Strategy

### Error Categories

| Category | Example | Handling |
|---|---|---|
| **Startup** | mpv not found | Print message to stderr, exit before TUI |
| **Network** | NTS API timeout | Show in status line: "Failed to load. Press r to retry" |
| **Playback** | mpv crash, invalid URL | Show error, auto-clear after 5s, queue advances |
| **SoundCloud client_id** | JS bundle changed | Retry up to 3 times with backoff, then show error |
| **SoundCloud auth** | Token expired (401) | Show "Token expired. Run: clisten auth soundcloud" |
| **Database** | SQLite write error | Log to file, show error in status line |
| **Config** | Invalid TOML | Fall back to defaults, log warning |

### Error Display

```rust
// Error action with auto-dismiss duration
Action::ShowError(String),  // display in status line
Action::ClearError,         // auto-sent after 5 seconds via tokio::spawn delay
```

The status line (bottom of screen, above play controls) shows errors:

```
┌────────────────────────────────────────────────────┐
│  ⚠ Failed to load NTS data. Press r to retry.     │
│  ▶ Space Play/Pause  n Next  p Prev  ...          │
└────────────────────────────────────────────────────┘
```

### Implementation Pattern

```rust
// In App::handle_action for network requests:
Action::LoadNtsLive => {
    let tx = self.action_tx.clone();
    let client = self.nts_client.clone();
    tokio::spawn(async move {
        match client.fetch_live().await {
            Ok(items) => { tx.send(Action::NtsLiveLoaded(items)).ok(); }
            Err(e) => {
                tracing::error!("Failed to load NTS live: {}", e);
                tx.send(Action::ShowError(format!("Failed to load NTS live data: {}", e))).ok();
            }
        }
    });
}

// Auto-dismiss errors after 5 seconds:
Action::ShowError(msg) => {
    self.error_message = Some(msg);
    let tx = self.action_tx.clone();
    tokio::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        tx.send(Action::ClearError).ok();
    });
}
Action::ClearError => {
    self.error_message = None;
}
```

---

