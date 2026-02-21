## 16. Phased Implementation Order

### Phase 1: Scaffolding

**Goal**: Running TUI with correct layout, tab switching, keyboard navigation, graceful quit. No data.

**Create (15 files):**
```
Cargo.toml
src/main.rs
src/app.rs
src/action.rs
src/tui.rs
src/config.rs
src/errors.rs
src/logging.rs
src/theme.rs
src/components/mod.rs
src/components/tabs.rs
src/components/discovery_list.rs
src/components/search_bar.rs
src/components/now_playing.rs
src/components/play_controls.rs
```

**Testable after Phase 1:**
- `cargo build` succeeds
- Launch `clisten` → see 3-panel layout
- `Tab` switches tabs, `j`/`k` scrolls (empty) list
- `/` focuses search, `Escape` unfocuses
- `q` quits cleanly (terminal restored)

### Phase 2: NTS Core

**Goal**: Browse live, picks, recently added. Play audio through mpv with pause support.

**Create (7 files):**
```
src/api/mod.rs
src/api/models.rs
src/api/nts.rs
src/player/mod.rs
src/components/nts/mod.rs
src/components/nts/live.rs
src/components/nts/collections.rs
```

**Modify:**
```
src/app.rs       — add NtsClient, MpvPlayer, async task spawning
src/action.rs    — add NTS data actions, playback actions
src/components/discovery_list.rs — render DiscoveryItem
src/components/now_playing.rs    — show track info
```

**Testable after Phase 2:**
- Live NTS data appears in the list
- Browse Picks and Recently Added
- Select a live stream → hear audio through mpv
- `Space` pauses/resumes
- Now Playing shows show name, genres
- Position counter updates

### Phase 3: NTS Discovery

**Goal**: Full NTS — mixtapes, shows browser, search, favorites, history in SQLite.

**Create (5 files):**
```
migrations/001_init.sql
src/db.rs
src/components/nts/mixtapes.rs
src/components/nts/shows.rs
src/components/nts/schedule.rs
```

**Modify:**
```
src/app.rs                       — add Database, favorites/history handlers
src/action.rs                    — add favorite/history/search actions
src/components/nts/mod.rs        — add sub-views (Favorites, History tabs)
src/components/discovery_list.rs — favorite indicators (♥), filtering
src/components/search_bar.rs     — per-view search behavior
```

**Testable after Phase 3:**
- Browse all 18 Infinite Mixtapes, play them (direct stream URLs)
- Browse Shows A-Z, drill into a show to see episodes
- `f` toggles favorite (heart appears)
- View Favorites and History sub-tabs
- All persists across sessions
- `/` filters current view client-side

### Phase 4: SoundCloud

**Goal**: Search and play SC tracks, authenticate for favorites.

**Create (4 files):**
```
src/api/soundcloud.rs
src/components/soundcloud/mod.rs
src/components/soundcloud/search.rs
src/components/soundcloud/favorites.rs
```

**Modify:**
```
src/main.rs      — add `auth` subcommand
src/app.rs       — add SoundCloudClient, SC action handlers
src/action.rs    — add SoundCloud actions
src/api/models.rs — add SoundCloud serde types
```

**Testable after Phase 4:**
- Switch to SoundCloud tab
- Type search query → see track results
- Select a track → hear it through mpv
- `clisten auth soundcloud` → opens browser, saves token
- With auth: browse liked tracks
- SC tracks saveable to local favorites

### Phase 5: Queue & Polish

**Goal**: Queue, refined keybindings, error handling, config, help overlay.

**Create (1 file):**
```
src/player/queue.rs
```

**Modify:**
```
src/app.rs                       — queue integration, error display, dep check
src/action.rs                    — queue actions, error timing
src/player/mod.rs                — process monitoring, auto-advance
src/components/play_controls.rs  — queue info display
src/config.rs                    — full config file support with keybinding overrides
```

**Testable after Phase 5:**
- `a` adds to queue, `A` adds next
- `n`/`p` navigates queue, auto-advance on track end
- `c` clears queue
- `?` shows help overlay
- Config file overrides keybindings
- Missing mpv/yt-dlp shows clear error on startup
- Network errors show in status line with retry prompt

---
