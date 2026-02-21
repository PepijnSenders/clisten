## 5. Architecture

### Component Pattern

Each UI panel implements a `Component` trait. The `App` struct holds named component fields (not a `Vec<Box<dyn Component>>`) for precise layout control.

```
┌─────────────────────────────────────────────────┐
│                    App                           │
│  ┌──────────┐  ┌──────────────┐  ┌───────────┐ │
│  │   Tabs   │  │ DiscoveryList│  │ NowPlaying│ │
│  └──────────┘  └──────────────┘  └───────────┘ │
│  ┌──────────┐  ┌──────────────┐                 │
│  │SearchBar │  │ PlayControls │                 │
│  └──────────┘  └──────────────┘                 │
│  ┌──────────────────────────────────┐           │
│  │    NtsTab / SoundCloudTab        │           │
│  │  (sub-tab coordinators)          │           │
│  └──────────────────────────────────┘           │
└─────────────────────────────────────────────────┘
```

### Action / Message Passing

All communication flows through a single `mpsc::UnboundedSender<Action>` channel:

```
User Input ──→ Component::handle_key_event()
                    │
                    ▼
              Action::PlayItem(item)
                    │
                    ▼
         App::handle_action() ──→ MpvPlayer::play(url)
                    │              Database::add_history()
                    ▼
              Action::PlaybackStarted(info)
                    │
                    ▼
         NowPlaying::update() ──→ re-renders with new track info
```

### Async Model

```rust
// Main event loop (in app.rs)
loop {
    tokio::select! {
        // Terminal events (keyboard, resize)
        Some(event) = tui.events.next() => {
            self.handle_event(event)?;
        }
        // Actions from components or async tasks
        Some(action) = action_rx.recv() => {
            self.handle_action(action).await?;
        }
        // Render tick (30 FPS)
        _ = render_interval.tick() => {
            tui.draw(|frame| self.render(frame))?;
        }
    }
}
```

### Data Flow

```
NTS API ──→ NtsClient ──→ NtsLiveResponse ──→ Vec<DiscoveryItem> ──→ DiscoveryList
SC  API ──→ SoundCloudClient ──→ ScSearchResponse ──→ Vec<DiscoveryItem> ──→ DiscoveryList
                                                                           │
                                                                     User selects
                                                                           │
                                                                           ▼
                                                                   Action::PlayItem
                                                                           │
                                                              ┌────────────┼────────────┐
                                                              ▼            ▼             ▼
                                                         MpvPlayer    Database     NowPlaying
                                                        (spawn mpv)  (history)   (show info)
```

### Module Dependency Graph

```
main.rs ──→ app.rs ──→ tui.rs (terminal event loop)
                   ──→ components/* (render + handle_events)
                   ──→ action.rs (message passing)
                   ──→ config.rs (keybindings, settings)

components/* ──→ action.rs (send actions)
             ──→ api/models.rs (DiscoveryItem)

app.rs ──→ player/mod.rs (playback control)
       ──→ api/*.rs (data fetching)
       ──→ db.rs (favorites, history)
```

---

