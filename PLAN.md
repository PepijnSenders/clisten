# clisten — Implementation Plan (ATDD)

> NTS Radio & SoundCloud TUI Player — Rust
>
> Each phase defines acceptance criteria (AC) that drive the implementation.
> Tests are written first to encode the AC, then code is written to make them pass.
>
> **Two test layers**:
> - **Rust tests** (`tests/*.rs`) — unit + integration tests for logic, data models, state
> - **Visual e2e tests** (`tests/e2e/*.test.ts`) — [tuistory](https://github.com/remorses/tuistory) tests that launch the real binary and assert on rendered terminal output
>
> See `spec/` for detailed module specifications.

---

# Phase 1: Scaffolding

**Goal**: Running TUI with correct layout, tab switching, keyboard navigation, graceful quit. No data.

**Spec references**: [00-overview.md](spec/00-overview.md) §2-4, [01-architecture.md](spec/01-architecture.md) §5, [07-config.md](spec/07-config.md) §11, [08-components.md](spec/08-components.md) §8.1-8.5, [09-keybindings.md](spec/09-keybindings.md) §12

**Tests**: `tests/phase1_scaffolding.rs`, `tests/e2e/phase1.test.ts`

**✓ Phase 1 Complete When:**
```bash
# Build
cargo build                              # No errors

# Rust unit tests
cargo test --test phase1_scaffolding     # All pass

# Visual e2e tests
cargo build && cd tests/e2e && npx vitest run phase1

# Manual smoke test
cargo run
# → See 3-panel layout (left list, right "Now Playing", bottom bar)
# → Tab switches NTS ↔ SoundCloud
# → / focuses search, Escape unfocuses
# → j/k scrolls (empty list)
# → q quits cleanly (terminal restored)
```

## 1.1 Initialize Project
> Spec: [00-overview.md](spec/00-overview.md) §4 Cargo.toml

- [x] Create `Cargo.toml` with all dependencies (ratatui, crossterm, tokio, reqwest, serde, rusqlite, clap, etc.)
- [x] Create `migrations/001_init.sql` — SQL schema for favorites + history tables
- [x] Verify `cargo build` succeeds

## 1.2 Core Module Stubs
> Spec: [00-overview.md](spec/00-overview.md) §3, Appendix

- [x] `src/errors.rs` — `color_eyre::install()` wrapper
- [x] `src/logging.rs` — tracing to `~/.local/share/clisten/clisten.log`
- [x] `src/theme.rs` — `Theme` struct with style methods (title, highlight, active_tab, etc.)
- [x] `src/action.rs` — `Action` enum with all variants (Quit, NextTab, ScrollDown, PlayItem, etc.)

## 1.3 Config Loading
> Spec: [07-config.md](spec/07-config.md) §11

- [x] `src/config.rs` — `Config`, `GeneralConfig`, `SoundCloudConfig`, `KeybindingConfig` structs
- [x] `Config::load()` — load from `~/.config/clisten/config.toml` or fall back to defaults
- [x] `Config::save()`, `save_soundcloud_token()` helper
- [x] **Test**: `test_config_default_values` — default frame_rate=30.0, expected keybindings
- [x] **Test**: `test_config_parse_toml` — parses valid TOML string
- [x] **Test**: `test_config_missing_file_uses_defaults` — non-existent path returns defaults

## 1.4 TUI Event Loop
> Spec: [08-components.md](spec/08-components.md) §8.3

- [x] `src/tui.rs` — `Tui` struct wrapping crossterm + ratatui terminal
- [x] `TuiEvent` enum: `Key(KeyEvent)`, `Resize(u16, u16)`, `Tick`
- [x] `enter()` — enable raw mode, alternate screen, hide cursor, start event polling
- [x] `exit()` — disable raw mode, restore screen, show cursor
- [x] Event polling task with `crossterm::event::EventStream`, tick at frame_rate

## 1.5 Component Trait + Components
> Spec: [08-components.md](spec/08-components.md) §8.1, §8.5-8.9

- [x] `src/components/mod.rs` — `Component` trait (register_action_handler, handle_key_event, update, draw)
- [x] `src/components/tabs.rs` — `TabBar` with `PrimaryTab::Nts | SoundCloud`, NextTab/PrevTab cycling
- [x] `src/components/discovery_list.rs` — `DiscoveryList` with items, ListState, next/prev/selected_item
- [x] `src/components/search_bar.rs` — `SearchBar` with focused state, input string, key handling
- [x] `src/components/now_playing.rs` — `NowPlaying` with current_item, position_secs, paused
- [x] `src/components/play_controls.rs` — `PlayControls` with playing/paused/queue state, hint rendering
- [x] **Test**: `test_tab_bar_initial_state` — starts on NTS
- [x] **Test**: `test_tab_bar_next_tab` — NTS → SoundCloud
- [x] **Test**: `test_tab_bar_wraps_around` — SoundCloud → NTS
- [x] **Test**: `test_discovery_list_scroll_down` — j/Down advances selection
- [x] **Test**: `test_discovery_list_scroll_up` — k/Up moves back
- [x] **Test**: `test_discovery_list_clamps_at_bounds` — can't scroll past first/last
- [x] **Test**: `test_search_bar_focus` — FocusSearch sets focused=true
- [x] **Test**: `test_search_bar_unfocus` — Back clears input, unfocuses
- [x] **Test**: `test_search_bar_typing` — char events append to input
- [x] **Test**: `test_now_playing_initial_state` — current_item is None
- [x] **Test**: `test_play_controls_initial_state` — not playing, not paused

## 1.6 App + Layout + Key Handling
> Spec: [08-components.md](spec/08-components.md) §8.4, [00-overview.md](spec/00-overview.md) §2, [09-keybindings.md](spec/09-keybindings.md) §12

- [x] `src/app.rs` — `App` struct holding all components, action channel, event loop
- [x] Layout: outer vertical (main + bottom 3), main horizontal (60% left + 40% right), left vertical (tabs 1 + sub-tabs 1 + list Min + search 3)
- [x] Key handling: q=Quit, Tab/BackTab=NextTab/PrevTab, Space=TogglePlayPause, j/k=scroll, /=FocusSearch, Esc=Back
- [x] Search mode: all char keys go to search input, only Escape/Enter special
- [x] `src/main.rs` — CLI parsing (clap), bootstrap, dependency checks
- [x] **Test**: `test_quit_sends_action` — q sends Action::Quit when search unfocused
- [x] **Test**: `test_quit_ignored_in_search` — q in search mode types 'q'

## 1.7 Visual E2E — Layout + Interaction
> Visual verification of the rendered TUI

- [x] **E2E**: `shows 3-panel layout on launch` — waitForText "NTS", "Now Playing", "Play/Pause"
- [x] **E2E**: `Tab switches to SoundCloud` — press Tab, waitForText "SoundCloud" highlighted
- [x] **E2E**: `/ focuses search bar` — press "/", type "test", assert "test" in search area
- [x] **E2E**: `Escape unfocuses search` — press Escape, assert "/ Search..." placeholder returns
- [x] **E2E**: `q quits the app` — press "q", assert session exits cleanly
- [x] **E2E**: `Now Playing shows nothing playing` — waitForText "Nothing playing"
- [x] **E2E**: `bottom bar shows keybinding hints` — waitForText "Play/Pause", "Quit", "Search"

---

# Phase 2: NTS Core

**Goal**: Browse live, picks, recently added. Play audio through mpv with pause support.

**Spec references**: [02-data-models.md](spec/02-data-models.md) §7, [03-api-nts.md](spec/03-api-nts.md) §6.1, [05-player.md](spec/05-player.md) §9, [08-components.md](spec/08-components.md) §8.4, §8.10-8.12

**Tests**: `tests/phase2_nts_core.rs`, `tests/e2e/phase2.test.ts`

**✓ Phase 2 Complete When:**
```bash
cargo build
cargo test --test phase2_nts_core
cd tests/e2e && npx vitest run phase2

# Manual: launch, see live channels, select one → hear audio, Space pauses
```

## 2.1 API Serde Types
> Spec: [02-data-models.md](spec/02-data-models.md) §7.1-7.2, [03-api-nts.md](spec/03-api-nts.md) §6.1

- [x] `src/api/mod.rs` — module declarations
- [x] `src/api/models.rs` — NTS types: `ApiLink`, `Genre`, `NtsMedia`, `AudioSource`, `ResultSet`, `PaginationMetadata`, `NtsLiveResponse`, `NtsChannel`, `NtsBroadcast`, `BroadcastEmbeds`, `NtsEpisodeDetail`, `NtsCollectionResponse`, `NtsShowsResponse`, `NtsShow`, `NtsMixtapesResponse`, `NtsMixtape`, `MixtapeCredit`, `MixtapeMedia`
- [x] `NtsChannel.upcoming()` method — extracts next2..next17 broadcasts from extra fields
- [x] SoundCloud types (stubs for now): `SoundCloudSearchResponse`, `SoundCloudTrack`, `SoundCloudUser`, `SoundCloudStreamResponse`
- [x] **Test**: `test_nts_live_response_deserializes` — parse example /api/v2/live JSON
- [x] **Test**: `test_nts_collection_response_deserializes` — parse nts-picks JSON
- [x] **Test**: `test_nts_channel_upcoming_extraction` — extracts next2..next17

## 2.2 DiscoveryItem Enum
> Spec: [02-data-models.md](spec/02-data-models.md) §7.3

- [x] `DiscoveryItem` enum: `NtsLiveChannel`, `NtsEpisode`, `NtsMixtape`, `NtsShow`, `SoundCloudTrack`
- [x] `title()`, `subtitle()`, `playback_url()`, `favorite_key()` methods
- [x] **Test**: `test_discovery_item_title` — correct display text per variant
- [x] **Test**: `test_discovery_item_subtitle` — genres/location/duration per variant
- [x] **Test**: `test_discovery_item_playback_url` — stream URLs for live/mixtape, None for shows
- [x] **Test**: `test_discovery_item_favorite_key` — unique keys per variant

## 2.3 NTS API Client
> Spec: [03-api-nts.md](spec/03-api-nts.md) Appendix

- [x] `src/api/nts.rs` — `NtsClient` with reqwest::Client
- [x] `fetch_live()` → `Vec<DiscoveryItem::NtsLiveChannel>`
- [x] `fetch_picks()` → `Vec<DiscoveryItem::NtsEpisode>`
- [x] `fetch_recent(offset, limit)` → `Vec<DiscoveryItem::NtsEpisode>`
- [x] `fetch_shows(offset, limit)` → `Vec<DiscoveryItem::NtsShow>`
- [x] `fetch_show_episodes(alias, offset, limit)` → `Vec<DiscoveryItem::NtsEpisode>`
- [x] `fetch_mixtapes()` → `Vec<DiscoveryItem::NtsMixtape>`
- [x] `episode_to_discovery()` helper function
- [x] **Test**: `test_nts_client_fetch_live` — (integration) returns 2 channels
- [x] **Test**: `test_nts_client_fetch_picks` — (integration) returns ~15 episodes

## 2.4 MpvPlayer
> Spec: [05-player.md](spec/05-player.md) §9.1

- [x] `src/player/mod.rs` — `MpvPlayer` with IPC socket at `/tmp/clisten-mpv-{pid}.sock`
- [x] `play(url)` — kill existing, spawn mpv with `--no-video --no-terminal --input-ipc-server=`
- [x] `toggle_pause()` — send `{"command":["cycle","pause"]}` via IPC
- [x] `stop()` — send quit command, kill process, remove socket
- [x] `seek(secs)` — relative seek via IPC
- [x] Position polling task — sends `PlaybackPosition(f64)` every second
- [x] Process exit monitoring — sends `PlaybackFinished` when mpv exits
- [x] `Drop` impl — remove socket file
- [x] **Test**: `test_mpv_player_new` — socket path uses pid
- [x] **Test**: `test_mpv_player_play_spawns_process` — (integration, requires mpv)

## 2.5 NTS Sub-Tab Coordinator
> Spec: [08-components.md](spec/08-components.md) §8.10-8.12

- [x] `src/components/nts/mod.rs` — `NtsTab` with `NtsSubTab` enum (Live, Picks, Recent, Mixtapes, Shows, Schedule, Favorites, History)
- [x] `switch_sub_tab(index)` — changes active_sub, returns load action if first visit
- [x] `load_if_needed()` — lazy loading: only triggers API fetch on first visit
- [x] `src/components/nts/live.rs` — `live_items_from_action()` helper
- [x] `src/components/nts/collections.rs` — `picks_items_from_action()`, `recent_items_from_action()`
- [x] **Test**: `test_nts_tab_initial_state` — starts on Live
- [x] **Test**: `test_nts_tab_lazy_loading` — first visit returns load action, second doesn't

## 2.6 Wire Up App — NTS Data + Playback
> Spec: [08-components.md](spec/08-components.md) §8.4

- [x] Add NtsClient, MpvPlayer to App struct
- [x] Handle `LoadNtsLive` → spawn async fetch → `NtsLiveLoaded`
- [x] Handle `LoadNtsPicks` → spawn async fetch → `NtsPicksLoaded`
- [x] Handle `LoadNtsRecent` → spawn async fetch → `NtsRecentLoaded`
- [x] Handle `PlayItem` → player.play(url), send PlaybackStarted + AddToHistory
- [x] Handle `TogglePlayPause` → player.toggle_pause()
- [x] Handle `Stop` → player.stop()
- [x] Update NowPlaying on PlayItem/PlaybackPosition/Stop/PlaybackFinished
- [x] Initial data load: send `LoadNtsLive` on startup

## 2.7 Visual E2E — NTS Data + Playback
- [x] **E2E**: `live channels appear on launch` — waitForText matching /Channel [12]/
- [x] **E2E**: `switch to Picks shows episodes` — press "2", waitForText with content
- [x] **E2E**: `Enter on live channel starts playback` — press Enter, "Nothing playing" disappears
- [x] **E2E**: `Space pauses and resumes` — press Space, see pause indicator, press again, see play
- [x] **E2E**: `Now Playing shows title and position` — waitForText /\d+:\d{2}/
- [x] **E2E**: `sub-tab bar shows NTS views` — assert "Live", "Picks", "Recent" visible

---

# Phase 3: NTS Discovery

**Goal**: Full NTS — mixtapes, shows browser, favorites, history in SQLite.

**Spec references**: [02-data-models.md](spec/02-data-models.md) §7.4, [03-api-nts.md](spec/03-api-nts.md) §6.1, [06-database.md](spec/06-database.md) §10, [08-components.md](spec/08-components.md) §8.13-8.15

**Tests**: `tests/phase3_nts_discovery.rs`, `tests/e2e/phase3.test.ts`

**✓ Phase 3 Complete When:**
```bash
cargo build
cargo test --test phase3_nts_discovery
cd tests/e2e && npx vitest run phase3

# Manual: browse mixtapes, shows drill-down, f favorites, Favorites/History tabs
```

## 3.1 SQLite Database
> Spec: [06-database.md](spec/06-database.md) §10

- [x] `src/db.rs` — `Database` struct with rusqlite Connection
- [x] `Database::open()` — create `~/.local/share/clisten/clisten.db`, run migrations
- [x] `FavoriteRecord` and `HistoryRecord` structs
- [x] `add_favorite(item)`, `remove_favorite(key)`, `is_favorite(key)`, `list_favorites(source, limit, offset)`
- [x] `add_to_history(item)`, `list_history(limit, offset)`, `clear_history()`
- [x] **Test**: `test_database_open_creates_file`
- [x] **Test**: `test_database_add_favorite` — insert + is_favorite returns true
- [x] **Test**: `test_database_remove_favorite` — delete + is_favorite returns false
- [x] **Test**: `test_database_add_duplicate_favorite` — INSERT OR IGNORE
- [x] **Test**: `test_database_list_favorites` — ordered by created_at DESC
- [x] **Test**: `test_database_list_favorites_by_source` — filter by "nts" or "soundcloud"
- [x] **Test**: `test_database_add_to_history`
- [x] **Test**: `test_database_list_history` — ordered by played_at DESC
- [x] **Test**: `test_database_clear_history`
- [x] **Test**: `test_database_history_allows_duplicates`

## 3.2 NTS Mixtapes View
> Spec: [03-api-nts.md](spec/03-api-nts.md) §6.1 GET /api/v2/mixtapes, [08-components.md](spec/08-components.md) §8.14

- [x] `src/components/nts/mixtapes.rs` — `mixtape_items_from_action()` helper
- [x] Handle `LoadNtsMixtapes` → fetch → `NtsMixtapesLoaded` in App
- [x] **Test**: `test_nts_client_fetch_mixtapes` — (integration) returns 18 mixtapes
- [x] **Test**: `test_mixtape_has_stream_url` — non-empty audio_stream_endpoint

## 3.3 NTS Shows View with Drill-Down
> Spec: [03-api-nts.md](spec/03-api-nts.md) §6.1 GET /api/v2/shows, [08-components.md](spec/08-components.md) §8.13

- [x] `src/components/nts/shows.rs` — `ShowsView` with drill_down_alias, shows list, episodes list
- [x] `visible_items()` — returns episodes if drilled in, shows otherwise
- [x] Handle `NtsShowEpisodesLoaded` → set drill-down
- [x] Handle `Back` when drilled down → clear drill-down, restore shows
- [x] Handle `LoadNtsShows` → fetch → `NtsShowsLoaded` in App
- [x] Handle `LoadNtsShowEpisodes` → fetch → `NtsShowEpisodesLoaded` in App
- [x] **Test**: `test_shows_view_drill_down` — episodes loaded, visible_items returns episodes
- [x] **Test**: `test_shows_view_back` — clears drill-down, visible_items returns shows

## 3.4 NTS Schedule View
> Spec: [08-components.md](spec/08-components.md) §8.15

- [x] `src/components/nts/schedule.rs` — `schedule_from_channels()` function
- [x] Build schedule from NtsChannel.upcoming(), sorted by start timestamp
- [x] Interleave both channels
- [x] **Test**: `test_schedule_from_channels` — sorted by timestamp
- [x] **Test**: `test_schedule_interleaves_channels` — both ch1 and ch2 included

## 3.5 Favorites Integration
> Spec: [09-keybindings.md](spec/09-keybindings.md) §12, [08-components.md](spec/08-components.md) §8.6

- [x] Add Database to App, load favorites HashSet at startup
- [x] Handle `ToggleFavorite` — add/remove from DB + HashSet based on selected item
- [x] Pass favorites set to DiscoveryList for heart (♥) rendering
- [x] Handle `AddToHistory` — insert into DB on playback start
- [x] Wire Favorites sub-tab (7) and History sub-tab (8) — load from DB
- [x] **Test**: `test_toggle_favorite_adds` — adds to DB + set
- [x] **Test**: `test_toggle_favorite_removes` — removes from DB + set

## 3.6 Number Keys for Sub-Tabs
> Spec: [09-keybindings.md](spec/09-keybindings.md) §12

- [x] Keys 1-8 send `SwitchSubTab(0-7)` when not in search mode
- [x] In search mode, digit keys type into search input
- [x] **Test**: `test_number_keys_send_switch_sub_tab`
- [x] **Test**: `test_number_keys_ignored_in_search`

## 3.7 Visual E2E — Discovery Features
- [x] **E2E**: `Mixtapes sub-tab shows mixtapes` — press "4", waitForText "Poolside"
- [x] **E2E**: `Shows drill-down and back` — press "5", Enter on show, Escape returns to shows
- [x] **E2E**: `Schedule shows upcoming` — press "6", see broadcast items
- [x] **E2E**: `f toggles heart` — press "f", waitForText "♥", press "f" again, heart gone
- [x] **E2E**: `Favorites sub-tab shows favorited items` — favorite item, press "7", see it listed
- [x] **E2E**: `History shows played items` — play item, press "8", see it in History
- [x] **E2E**: `all 8 sub-tabs accessible` — press "1" through "8", each highlights in turn

---

# Phase 4: SoundCloud

**Goal**: Search and play SC tracks, authenticate for favorites.

**Spec references**: [02-data-models.md](spec/02-data-models.md) §7.2, [04-api-soundcloud.md](spec/04-api-soundcloud.md) §6.2, §13-14, [08-components.md](spec/08-components.md) §8.16-8.18

**Tests**: `tests/phase4_soundcloud.rs`, `tests/e2e/phase4.test.ts`

**✓ Phase 4 Complete When:**
```bash
cargo build
cargo test --test phase4_soundcloud
cd tests/e2e && npx vitest run phase4

# Manual: Tab to SC, search, play track, check SC Favorites tab shows auth message
```

## 4.1 SoundCloud Client
> Spec: [04-api-soundcloud.md](spec/04-api-soundcloud.md) §13

- [x] `src/api/soundcloud.rs` — `SoundCloudClient` with client_id cache + TTL (4 hours)
- [x] `fetch_client_id()` — scrape SoundCloud homepage JS bundles, regex `client_id:"[a-zA-Z0-9]{32}"`
- [x] `ensure_client_id()` — lazy fetch + cache
- [x] `search_tracks(query, limit, offset)` → `Vec<SoundCloudTrack>`
- [x] `get_likes(limit, offset)` → `Vec<SoundCloudTrack>` (requires OAuth token)
- [x] `set_oauth_token(token)` setter
- [x] **Test**: `test_sc_client_new` — no client_id, no token
- [x] **Test**: `test_sc_client_id_extraction` — (integration) returns 32-char alphanumeric
- [x] **Test**: `test_sc_client_id_caching` — second call within TTL returns cached
- [x] **Test**: `test_sc_client_id_regex_pattern` — regex matches expected format
- [x] **Test**: `test_sc_search_tracks` — (integration) search "ambient" returns results

## 4.2 SoundCloud DiscoveryItem Conversion
> Spec: [02-data-models.md](spec/02-data-models.md) §7.3, [08-components.md](spec/08-components.md) §8.17

- [x] `src/components/soundcloud/search.rs` — `tracks_to_discovery()` converter
- [x] `src/components/soundcloud/favorites.rs` — `NO_AUTH_MESSAGE` constant
- [x] **Test**: `test_sc_tracks_to_discovery` — correct fields mapped
- [x] **Test**: `test_sc_track_playback_url` — returns permalink_url
- [x] **Test**: `test_sc_track_subtitle_format` — "artist · M:SS"
- [x] **Test**: `test_no_auth_message` — contains "clisten auth soundcloud"

## 4.3 SoundCloud Tab Coordinator
> Spec: [08-components.md](spec/08-components.md) §8.16

- [x] `src/components/soundcloud/mod.rs` — `SoundCloudTab` with `ScSubTab::Search | Favorites`
- [x] Search sub-tab: default view, search triggers on SearchSubmit
- [x] Favorites sub-tab: triggers LoadSoundCloudLikes, shows auth message if no token
- [x] **Test**: `test_sc_tab_initial_state` — starts on Search
- [x] **Test**: `test_sc_tab_switch_to_favorites` — triggers LoadSoundCloudLikes

## 4.4 Auth Subcommand
> Spec: [04-api-soundcloud.md](spec/04-api-soundcloud.md) §14

- [x] `clisten auth soundcloud` CLI subcommand in `src/main.rs`
- [x] Opens browser, prints instructions, reads token from stdin
- [x] Strips "OAuth " prefix, saves to config via `save_soundcloud_token()`
- [x] **Test**: `test_save_soundcloud_token` — writes + reads back
- [x] **Test**: `test_strip_oauth_prefix` — "OAuth abc123" → "abc123"

## 4.5 Wire Up App — SoundCloud
- [x] Add SoundCloudClient to App, load OAuth token from config
- [x] Handle `SearchSoundCloud(query)` → search_tracks → `SoundCloudSearchLoaded`
- [x] Handle `SearchSubmit` when SC tab active → send `SearchSoundCloud`
- [x] Handle `LoadSoundCloudLikes` → get_likes → `SoundCloudLikesLoaded`
- [x] SC tracks play via permalink_url (mpv + yt-dlp resolves internally)
- [x] SC tracks can be favorited to local DB

## 4.6 Visual E2E — SoundCloud
- [x] **E2E**: `SoundCloud tab shows Search and Favorites` — Tab, assert sub-tab labels
- [x] **E2E**: `search and see results` — Tab, "/", type "bonobo", Enter, waitForText tracks
- [x] **E2E**: `Enter on SC track starts playback` — search, Enter, Now Playing updates
- [x] **E2E**: `Favorites without auth shows message` — Tab, "2", waitForText "clisten auth soundcloud"

---

# Phase 5: Queue & Polish

**Goal**: Queue, error handling, help overlay, full config support.

**Spec references**: [05-player.md](spec/05-player.md) §9.2, [08-components.md](spec/08-components.md) §8.4, [09-keybindings.md](spec/09-keybindings.md) §12, §15

**Tests**: `tests/phase5_queue_polish.rs`, `tests/e2e/phase5.test.ts`

**✓ Phase 5 Complete When:**
```bash
cargo build
cargo test --test phase5_queue_polish
cd tests/e2e && npx vitest run phase5

# Manual: a adds to queue, n/p navigates, c clears, ? help, error retry with r
```

## 5.1 Queue
> Spec: [05-player.md](spec/05-player.md) §9.2

- [x] `src/player/queue.rs` — `Queue` struct with items Vec, current_index
- [x] `add(item)` — append to end, set current to 0 if first
- [x] `add_next(item)` — insert after current position
- [x] `remove(index)` — delete at index, adjust current_index
- [x] `clear()` — empty queue, current = None
- [x] `next()` — advance, return next item
- [x] `prev()` — go back, return previous item
- [x] `current()`, `items()`, `len()`, `is_empty()`, `current_index()`
- [x] **Test**: `test_queue_new_empty`
- [x] **Test**: `test_queue_add` — appends, sets current to 0
- [x] **Test**: `test_queue_add_next` — inserts after current
- [x] **Test**: `test_queue_remove` — deletes, adjusts index
- [x] **Test**: `test_queue_clear`
- [x] **Test**: `test_queue_next` — advances, returns item
- [x] **Test**: `test_queue_next_at_end` — returns None
- [x] **Test**: `test_queue_prev` — decrements, returns item
- [x] **Test**: `test_queue_prev_at_start` — returns None

## 5.2 Queue Integration in App
> Spec: [08-components.md](spec/08-components.md) §8.4, [09-keybindings.md](spec/09-keybindings.md) §12

- [x] Add Queue to App struct
- [x] Handle `AddToQueue(item)` — queue.add()
- [x] Handle `AddToQueueNext(item)` — queue.add_next()
- [x] Handle `ClearQueue` — queue.clear()
- [x] Handle `NextTrack` — queue.next() → play
- [x] Handle `PrevTrack` — queue.prev() → play
- [x] Handle `PlaybackFinished` — auto-advance: queue.next() → play, or stop if empty
- [x] Key 'a' → AddToQueue(selected), 'A' → AddToQueueNext(selected), 'c' → ClearQueue
- [x] Update PlayControls with queue_pos / queue_len
- [x] **Test**: `test_playback_finished_advances_queue`
- [x] **Test**: `test_playback_finished_empty_queue`
- [x] **Test**: `test_key_a_adds_to_queue`
- [x] **Test**: `test_key_c_clears_queue`

## 5.3 Error Handling
> Spec: [09-keybindings.md](spec/09-keybindings.md) §15

- [x] `ShowError(msg)` — set error_message, spawn 5-second delayed ClearError
- [x] `ClearError` — clear error_message
- [x] Display error in status line above play controls
- [x] 'r' key when error present → retry last load action
- [x] SC client_id failure → retry up to 3 times with backoff
- [x] SC 401 → show "Token expired. Run: clisten auth soundcloud"
- [x] **Test**: `test_show_error_sets_message`
- [x] **Test**: `test_clear_error_clears_message`
- [x] **Test**: `test_retry_key_resends_load`
- [x] **Test**: `test_retry_key_ignored_without_error`

## 5.4 Help Overlay
> Spec: [09-keybindings.md](spec/09-keybindings.md) §12

- [x] '?' toggles help overlay (ShowHelp / HideHelp)
- [x] Overlay lists all keybindings from §12
- [x] Any key dismisses overlay
- [x] **Test**: `test_help_toggle_on` — ShowHelp sets true
- [x] **Test**: `test_help_toggle_off` — HideHelp sets false
- [x] **Test**: `test_question_mark_toggles_help`

## 5.5 Dependency Check on Startup
> Spec: [04-api-soundcloud.md](spec/04-api-soundcloud.md) §14

- [x] `check_dependencies()` in main.rs — missing mpv → error + exit, missing yt-dlp → warning
- [x] **Test**: `test_check_mpv_present` — (integration) which::which("mpv")

## 5.6 Visual E2E — Queue + Polish
- [x] **E2E**: `a adds to queue, bottom bar shows count` — press "a", waitForText "Track 1/1"
- [x] **E2E**: `c clears queue` — press "c", counter disappears
- [x] **E2E**: `? shows help overlay` — press "?", waitForText all keybinding names
- [x] **E2E**: `any key dismisses help` — press "a" from help, overlay gone
- [x] **E2E**: `missing mpv shows error` — launch with empty PATH, waitForText "mpv is required"
- [x] **E2E**: `error auto-dismisses` — trigger error, wait 6s, error gone

---

# E2E Test Infrastructure

## Setup (`tests/e2e/`)

```
tests/e2e/
├── package.json
├── vitest.config.ts
├── helpers.ts
├── phase1.test.ts
├── phase2.test.ts
├── phase3.test.ts
├── phase4.test.ts
└── phase5.test.ts
```

### `package.json`
```json
{
  "private": true,
  "scripts": {
    "test": "vitest run",
    "test:watch": "vitest --watch"
  },
  "devDependencies": {
    "tuistory": "latest",
    "vitest": "latest"
  }
}
```

### `helpers.ts`
```typescript
import { launchTerminal } from 'tuistory'
import { resolve } from 'path'

const BINARY = resolve(__dirname, '../../target/debug/clisten')

export async function launchClisten(opts?: {
  cols?: number
  rows?: number
  env?: Record<string, string>
}) {
  return launchTerminal({
    command: BINARY,
    cols: opts?.cols ?? 120,
    rows: opts?.rows ?? 40,
    env: { ...process.env, ...opts?.env },
  })
}
```

### Running

```bash
# All e2e tests (requires cargo build first)
cargo build && cd tests/e2e && npx vitest run

# Single phase
cd tests/e2e && npx vitest run phase1

# Watch mode
cd tests/e2e && npx vitest --watch
```

---

# Phase 6: Refinements

**Goal**: Fix UX issues — visible sub-tab bar, client-side NTS filtering, faster SC search, search bar UX, volume control.

**Tests**: `tests/phase6_refinements.rs`, `tests/e2e/phase6.test.ts`

**✓ Phase 6 Complete When:**
```bash
cargo build
cargo test --test phase6_refinements
cd tests/e2e && npx vitest run phase6

# Manual: sub-tabs visible, NTS search filters, SC search shows spinner, volume [ ] works
```

## 6.1 Visible Sub-Tab Bar
> Bug: NtsTab/SoundCloudTab draw() methods exist but are never called in the app layout

- [x] Add `NtsTab` and `SoundCloudTab` as fields on `App` struct
- [x] Register action handlers for both tab coordinators
- [x] In the draw closure, render the active tab coordinator's sub-tab bar at `left[1]`
  - When `PrimaryTab::Nts` → draw NtsTab sub-tabs (Live|Picks|Recent|Mixtapes|Shows|Schedule|Favorites|History)
  - When `PrimaryTab::SoundCloud` → draw SoundCloudTab sub-tabs (Search|Favorites)
- [x] Route `SwitchSubTab` through the correct tab coordinator based on active primary tab
- [x] **Test**: `test_sub_tab_bar_renders_nts` — NtsTab draw produces tabs with "Live", "Picks", etc.
- [x] **Test**: `test_sub_tab_bar_renders_soundcloud` — SoundCloudTab draw produces "Search", "Favorites"
- [x] **Test**: `test_switch_sub_tab_routes_to_active_tab` — sub-tab switch goes to NTS coordinator when NTS active

## 6.2 SoundCloud Client ID Pre-Fetch
> Bug: first SC search is slow because client_id is fetched on-demand (scrapes SC homepage + JS bundles)

- [x] On app startup, spawn a background task to call `sc_client.ensure_client_id()` so the client_id is cached before the user ever searches
- [x] **Test**: `test_sc_client_id_prefetched` — after App::new(), client_id fetch has been spawned

## 6.3 NTS Client-Side Search (Filter)
> Feature: no NTS search API exists; filter the currently loaded list items by title/subtitle as user types

- [x] Add `filter_query: Option<String>` field to `DiscoveryList`
- [x] Add `all_items: Vec<DiscoveryItem>` to DiscoveryList — stores the full unfiltered dataset
- [x] `set_items()` stores into `all_items` and applies current filter to derive `items`
- [x] `set_filter(query: Option<String>)` — filters `all_items` by case-insensitive title/subtitle match, updates `items`
- [x] Handle `SearchSubmit` when NTS tab active → apply filter to DiscoveryList instead of API call
- [x] Handle `SearchClear` / `Back` when NTS tab → clear filter, restore full list
- [x] Add `Action::FilterList(String)` and `Action::ClearFilter` variants
- [x] **Test**: `test_discovery_list_filter` — set_items with 5 items, set_filter("jazz") → only matching items shown
- [x] **Test**: `test_discovery_list_clear_filter` — after clear, all items restored
- [x] **Test**: `test_nts_search_submit_filters` — SearchSubmit on NTS tab sends FilterList action

## 6.4 Search Bar UX — Clear on Submit + Loading State
> Bug: search bar keeps text after Enter. No visual feedback while results load.

- [x] After `SearchSubmit`, clear the search bar input and unfocus it
- [x] Add `loading: bool` state to `DiscoveryList`
- [x] On search/load actions, set `loading = true`; on data loaded, set `loading = false`
- [x] When `loading == true`, render "Searching..." or a spinner text in the list area instead of items
- [x] For NTS filter: no loading state needed (instant), just clear + unfocus search bar
- [x] **Test**: `test_search_bar_clears_on_submit` — input is empty after SearchSubmit
- [x] **Test**: `test_discovery_list_loading_state` — loading=true shows loading text

## 6.5 Volume Control
> Feature: [ and ] keys to control mpv volume

- [x] Add `VolumeUp` and `VolumeDown` action variants to `Action` enum
- [x] Add `set_volume(delta: f64)` method to `MpvPlayer` — sends `{"command":["add","volume",<delta>]}` via IPC
- [x] Add `get_volume()` method — sends `{"command":["get_property","volume"]}`, returns f64
- [x] Wire `]` key → `VolumeUp`, `[` key → `VolumeDown` in `handle_key()`
- [x] Handle `VolumeUp` → `player.set_volume(5.0)`, Handle `VolumeDown` → `player.set_volume(-5.0)` (5% steps)
- [x] Add `volume: Option<u8>` to `PlayControls`, display volume percentage in bottom bar
- [x] Poll volume alongside position (or update on volume change) — send `Action::VolumeChanged(u8)`
- [x] Update help overlay with `[` / `]` volume keybindings
- [x] **Test**: `test_volume_up_action_exists` — VolumeUp variant exists
- [x] **Test**: `test_volume_down_action_exists` — VolumeDown variant exists
- [x] **Test**: `test_bracket_keys_send_volume` — `]` sends VolumeUp, `[` sends VolumeDown
- [x] **Test**: `test_play_controls_shows_volume` — volume rendered in bottom bar

## 6.6 Visual E2E — Refinements
- [x] **E2E**: `sub-tab bar visible on NTS` — launch, waitForText "Live", "Picks", "Recent" in sub-tab row
- [x] **E2E**: `sub-tab bar visible on SoundCloud` — Tab to SC, waitForText "Search", "Favorites" in sub-tab row
- [x] **E2E**: `NTS search filters list` — type search on NTS, list filters to matching items
- [x] **E2E**: `search bar clears after submit` — search + Enter, search bar shows "/ Search..." placeholder
- [x] **E2E**: `volume keys change volume` — play item, press "]", see volume indicator in bottom bar

---

# Dependency Graph

```
Phase 1 (Scaffolding)
   └── Phase 2 (NTS Core)
          ├── Phase 3 (NTS Discovery)
          │      └── Phase 5 (Queue & Polish)
          └── Phase 4 (SoundCloud)
                 └── Phase 5 (Queue & Polish)
```

---

# File Manifest

Every file to be created, with spec reference:

```
Cargo.toml                              → 00-overview.md §4
migrations/001_init.sql                 → 06-database.md §10.1
src/main.rs                             → 04-api-soundcloud.md §14
src/app.rs                              → 08-components.md §8.4
src/action.rs                           → 08-components.md §8.2
src/tui.rs                              → 08-components.md §8.3
src/config.rs                           → 07-config.md §11
src/errors.rs                           → 00-overview.md Appendix
src/logging.rs                          → 00-overview.md Appendix
src/theme.rs                            → 00-overview.md Appendix
src/db.rs                               → 06-database.md §10.2
src/api/mod.rs                          → 00-overview.md Appendix
src/api/models.rs                       → 02-data-models.md §7
src/api/nts.rs                          → 03-api-nts.md Appendix
src/api/soundcloud.rs                   → 04-api-soundcloud.md §13
src/player/mod.rs                       → 05-player.md §9.1
src/player/queue.rs                     → 05-player.md §9.2
src/components/mod.rs                   → 08-components.md §8.1
src/components/tabs.rs                  → 08-components.md §8.5
src/components/discovery_list.rs        → 08-components.md §8.6
src/components/search_bar.rs            → 08-components.md §8.7
src/components/now_playing.rs           → 08-components.md §8.8
src/components/play_controls.rs         → 08-components.md §8.9
src/components/nts/mod.rs               → 08-components.md §8.10
src/components/nts/live.rs              → 08-components.md §8.11
src/components/nts/collections.rs       → 08-components.md §8.12
src/components/nts/shows.rs             → 08-components.md §8.13
src/components/nts/mixtapes.rs          → 08-components.md §8.14
src/components/nts/schedule.rs          → 08-components.md §8.15
src/components/soundcloud/mod.rs        → 08-components.md §8.16
src/components/soundcloud/search.rs     → 08-components.md §8.17
src/components/soundcloud/favorites.rs  → 08-components.md §8.18
```
