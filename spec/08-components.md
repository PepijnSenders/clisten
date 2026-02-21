## 8. Component Trait + Each Component

### 8.1 Component Trait

```rust
// src/components/mod.rs

pub mod tabs;
pub mod discovery_list;
pub mod search_bar;
pub mod now_playing;
pub mod play_controls;
pub mod nts;
pub mod soundcloud;

use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;
use tokio::sync::mpsc::UnboundedSender;

use crate::action::Action;

pub trait Component {
    /// Register the action sender for this component.
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>);

    /// Handle a key event. Return Ok(true) if the event was consumed.
    fn handle_key_event(&mut self, key: KeyEvent) -> anyhow::Result<bool> {
        let _ = key;
        Ok(false)
    }

    /// Handle an action dispatched by App. Return optional follow-up actions.
    fn update(&mut self, action: &Action) -> anyhow::Result<Vec<Action>> {
        let _ = action;
        Ok(vec![])
    }

    /// Render this component into the given area.
    fn draw(&self, frame: &mut Frame, area: Rect);
}
```

### 8.2 Action Enum

```rust
// src/action.rs

use crate::api::models::DiscoveryItem;

#[derive(Debug, Clone)]
pub enum Action {
    // ── Navigation ──
    Quit,
    NextTab,
    PrevTab,
    SwitchSubTab(usize),
    ScrollDown,
    ScrollUp,
    Select,       // Enter on highlighted item
    Back,         // Escape — go back / unfocus

    // ── Search ──
    FocusSearch,
    SearchInput(char),
    SearchBackspace,
    SearchSubmit,
    SearchClear,

    // ── Playback ──
    PlayUrl(String),
    PlayItem(DiscoveryItem),
    TogglePlayPause,
    Stop,
    NextTrack,
    PrevTrack,
    PlaybackStarted { title: String, url: String },
    PlaybackFinished,
    PlaybackPosition(f64), // seconds

    // ── Queue ──
    AddToQueue(DiscoveryItem),
    AddToQueueNext(DiscoveryItem),
    ClearQueue,

    // ── Favorites ──
    ToggleFavorite,
    FavoritesLoaded(Vec<DiscoveryItem>),

    // ── History ──
    AddToHistory(DiscoveryItem),
    HistoryLoaded(Vec<DiscoveryItem>),

    // ── NTS Data Loading ──
    LoadNtsLive,
    NtsLiveLoaded(Vec<DiscoveryItem>),
    LoadNtsPicks,
    NtsPicksLoaded(Vec<DiscoveryItem>),
    LoadNtsRecent { offset: u64 },
    NtsRecentLoaded(Vec<DiscoveryItem>),
    LoadNtsShows { offset: u64 },
    NtsShowsLoaded(Vec<DiscoveryItem>),
    LoadNtsShowEpisodes { show_alias: String },
    NtsShowEpisodesLoaded { show_alias: String, episodes: Vec<DiscoveryItem> },
    LoadNtsMixtapes,
    NtsMixtapesLoaded(Vec<DiscoveryItem>),

    // ── SoundCloud ──
    SearchSoundCloud(String),
    SoundCloudSearchLoaded(Vec<DiscoveryItem>),
    LoadSoundCloudLikes { offset: u64 },
    SoundCloudLikesLoaded(Vec<DiscoveryItem>),

    // ── UI ──
    ShowError(String),
    ClearError,
    ShowHelp,
    HideHelp,
    Resize(u16, u16),
    Tick,
    Render,
}
```

### 8.3 TUI Event Loop Wrapper

```rust
// src/tui.rs

use std::time::Duration;
use crossterm::{
    event::{self, Event as CrosstermEvent, KeyEvent, KeyEventKind},
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
    execute,
};
use futures::StreamExt;
use ratatui::{backend::CrosstermBackend, Terminal};
use tokio::sync::mpsc;

pub type CrosstermTerminal = Terminal<CrosstermBackend<std::io::Stderr>>;

pub struct Tui {
    pub terminal: CrosstermTerminal,
    pub event_rx: mpsc::UnboundedReceiver<TuiEvent>,
    event_tx: mpsc::UnboundedSender<TuiEvent>,
    frame_rate: f64,
}

#[derive(Debug)]
pub enum TuiEvent {
    Key(KeyEvent),
    Resize(u16, u16),
    Tick,
}

impl Tui {
    pub fn new(frame_rate: f64) -> anyhow::Result<Self> {
        let backend = CrosstermBackend::new(std::io::stderr());
        let terminal = Terminal::new(backend)?;
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        Ok(Self { terminal, event_rx, event_tx, frame_rate })
    }

    pub fn enter(&mut self) -> anyhow::Result<()> {
        terminal::enable_raw_mode()?;
        execute!(std::io::stderr(), EnterAlternateScreen)?;
        self.terminal.hide_cursor()?;
        self.terminal.clear()?;
        self.start_event_polling();
        Ok(())
    }

    pub fn exit(&mut self) -> anyhow::Result<()> {
        terminal::disable_raw_mode()?;
        execute!(std::io::stderr(), LeaveAlternateScreen)?;
        self.terminal.show_cursor()?;
        Ok(())
    }

    fn start_event_polling(&self) {
        let tx = self.event_tx.clone();
        let tick_rate = Duration::from_secs_f64(1.0 / self.frame_rate);

        tokio::spawn(async move {
            let mut reader = event::EventStream::new();
            let mut tick_interval = tokio::time::interval(tick_rate);

            loop {
                tokio::select! {
                    event = reader.next() => {
                        match event {
                            Some(Ok(CrosstermEvent::Key(key))) => {
                                if key.kind == KeyEventKind::Press {
                                    tx.send(TuiEvent::Key(key)).ok();
                                }
                            }
                            Some(Ok(CrosstermEvent::Resize(w, h))) => {
                                tx.send(TuiEvent::Resize(w, h)).ok();
                            }
                            Some(Err(_)) | None => break,
                            _ => {}
                        }
                    }
                    _ = tick_interval.tick() => {
                        tx.send(TuiEvent::Tick).ok();
                    }
                }
            }
        });
    }

    pub fn draw<F>(&mut self, f: F) -> anyhow::Result<()>
    where
        F: FnOnce(&mut ratatui::Frame),
    {
        self.terminal.draw(f)?;
        Ok(())
    }
}
```

### 8.4 App Struct + Event Loop

```rust
// src/app.rs

use std::collections::HashSet;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::{Constraint, Layout};
use tokio::sync::mpsc;

use crate::action::Action;
use crate::api::models::DiscoveryItem;
use crate::api::nts::NtsClient;
use crate::api::soundcloud::SoundCloudClient;
use crate::components::*;
use crate::components::tabs::TabBar;
use crate::components::discovery_list::DiscoveryList;
use crate::components::search_bar::SearchBar;
use crate::components::now_playing::NowPlaying;
use crate::components::play_controls::PlayControls;
use crate::config::Config;
use crate::db::Database;
use crate::player::MpvPlayer;
use crate::tui::{Tui, TuiEvent};

pub struct App {
    running: bool,
    action_tx: mpsc::UnboundedSender<Action>,
    action_rx: mpsc::UnboundedReceiver<Action>,

    // Components
    tab_bar: TabBar,
    discovery_list: DiscoveryList,
    search_bar: SearchBar,
    now_playing: NowPlaying,
    play_controls: PlayControls,

    // State
    nts_client: NtsClient,
    sc_client: SoundCloudClient,
    player: MpvPlayer,
    db: Database,
    config: Config,
    favorites: HashSet<String>, // favorite_key set for O(1) lookup
    show_help: bool,
    error_message: Option<String>,
}

impl App {
    pub fn new(config: Config) -> anyhow::Result<Self> {
        let (action_tx, action_rx) = mpsc::unbounded_channel();
        let db = Database::open()?;
        let favorites: HashSet<String> = db
            .list_favorites(None, 10000, 0)?
            .into_iter()
            .map(|f| f.key)
            .collect();

        let mut tab_bar = TabBar::new();
        let mut discovery_list = DiscoveryList::new();
        let mut search_bar = SearchBar::new();
        let mut now_playing = NowPlaying::new();
        let mut play_controls = PlayControls::new();

        // Register action handlers
        tab_bar.register_action_handler(action_tx.clone());
        discovery_list.register_action_handler(action_tx.clone());
        search_bar.register_action_handler(action_tx.clone());
        now_playing.register_action_handler(action_tx.clone());
        play_controls.register_action_handler(action_tx.clone());

        Ok(Self {
            running: true,
            action_tx,
            action_rx,
            tab_bar,
            discovery_list,
            search_bar,
            now_playing,
            play_controls,
            nts_client: NtsClient::new(),
            sc_client: SoundCloudClient::new(),
            player: MpvPlayer::new(),
            db,
            config,
            favorites,
            show_help: false,
            error_message: None,
        })
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        let mut tui = Tui::new(30.0)?;
        tui.enter()?;

        // Load initial data
        self.action_tx.send(Action::LoadNtsLive)?;

        while self.running {
            // Draw
            let favorites = &self.favorites;
            let discovery_list = &self.discovery_list;
            let tab_bar = &self.tab_bar;
            let search_bar = &self.search_bar;
            let now_playing = &self.now_playing;
            let play_controls = &self.play_controls;
            let show_help = self.show_help;
            let error_message = &self.error_message;

            tui.draw(|frame| {
                let outer = Layout::vertical([
                    Constraint::Min(0),
                    Constraint::Length(3),
                ]).split(frame.area());

                let main = Layout::horizontal([
                    Constraint::Percentage(60),
                    Constraint::Percentage(40),
                ]).split(outer[0]);

                let left = Layout::vertical([
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Min(0),
                    Constraint::Length(3),
                ]).split(main[0]);

                tab_bar.draw(frame, left[0]);
                // sub-tabs rendered by tab coordinator
                discovery_list.draw(frame, left[2]);
                search_bar.draw(frame, left[3]);
                now_playing.draw(frame, main[1]);
                play_controls.draw(frame, outer[1]);
            })?;

            // Process events
            tokio::select! {
                Some(event) = tui.event_rx.recv() => {
                    match event {
                        TuiEvent::Key(key) => self.handle_key(key)?,
                        TuiEvent::Resize(w, h) => {
                            self.action_tx.send(Action::Resize(w, h))?;
                        }
                        TuiEvent::Tick => {}
                    }
                }
                Some(action) = self.action_rx.recv() => {
                    self.handle_action(action).await?;
                }
            }
        }

        tui.exit()?;
        Ok(())
    }

    fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> anyhow::Result<()> {
        use KeyCode::*;

        // Global keys (always active)
        match (key.code, key.modifiers) {
            (Char('q'), KeyModifiers::NONE) if !self.search_bar.is_focused() => {
                self.action_tx.send(Action::Quit)?;
            }
            (Char('?'), KeyModifiers::NONE) if !self.search_bar.is_focused() => {
                self.action_tx.send(if self.show_help {
                    Action::HideHelp
                } else {
                    Action::ShowHelp
                })?;
            }
            (Tab, _) => self.action_tx.send(Action::NextTab)?,
            (BackTab, _) => self.action_tx.send(Action::PrevTab)?,
            (Char(' '), _) if !self.search_bar.is_focused() => {
                self.action_tx.send(Action::TogglePlayPause)?;
            }
            (Char('n'), _) if !self.search_bar.is_focused() => {
                self.action_tx.send(Action::NextTrack)?;
            }
            (Char('p'), _) if !self.search_bar.is_focused() => {
                self.action_tx.send(Action::PrevTrack)?;
            }
            (Char('s'), _) if !self.search_bar.is_focused() => {
                self.action_tx.send(Action::Stop)?;
            }
            (Char('/'), _) if !self.search_bar.is_focused() => {
                self.action_tx.send(Action::FocusSearch)?;
            }
            (Char('f'), _) if !self.search_bar.is_focused() => {
                self.action_tx.send(Action::ToggleFavorite)?;
            }
            (Char('c'), _) if !self.search_bar.is_focused() => {
                self.action_tx.send(Action::ClearQueue)?;
            }
            (Char('A'), _) if !self.search_bar.is_focused() => {
                if let Some(item) = self.discovery_list.selected_item() {
                    self.action_tx.send(Action::AddToQueueNext(item.clone()))?;
                }
            }
            (Char('r'), _) if !self.search_bar.is_focused() && self.error_message.is_some() => {
                // Retry: re-send the last load action for the current view
                self.action_tx.send(Action::LoadNtsLive)?; // simplified; real impl tracks last action
                self.error_message = None;
            }
            (Char(c), _) if !self.search_bar.is_focused() && c.is_ascii_digit() => {
                let idx = c.to_digit(10).unwrap_or(0) as usize;
                if idx >= 1 && idx <= 8 {
                    self.action_tx.send(Action::SwitchSubTab(idx - 1))?;
                }
            }
            (Esc, _) => self.action_tx.send(Action::Back)?,
            _ => {
                // Delegate to focused component
                if self.search_bar.is_focused() {
                    self.search_bar.handle_key_event(key)?;
                } else {
                    self.discovery_list.handle_key_event(key)?;
                }
            }
        }
        Ok(())
    }

    async fn handle_action(&mut self, action: Action) -> anyhow::Result<()> {
        match action {
            Action::Quit => self.running = false,
            Action::PlayItem(ref item) => {
                if let Some(url) = item.playback_url() {
                    self.player.play(&url).await?;
                    self.action_tx.send(Action::PlaybackStarted {
                        title: item.title().to_string(),
                        url,
                    })?;
                    self.action_tx.send(Action::AddToHistory(item.clone()))?;
                }
            }
            Action::TogglePlayPause => {
                self.player.toggle_pause().await?;
            }
            Action::Stop => {
                self.player.stop().await?;
            }
            Action::ToggleFavorite => {
                if let Some(item) = self.discovery_list.selected_item() {
                    let key = item.favorite_key();
                    if self.favorites.contains(&key) {
                        self.db.remove_favorite(&key)?;
                        self.favorites.remove(&key);
                    } else {
                        self.db.add_favorite(&item)?;
                        self.favorites.insert(key);
                    }
                }
            }
            Action::LoadNtsLive => {
                let tx = self.action_tx.clone();
                let client = self.nts_client.clone();
                tokio::spawn(async move {
                    match client.fetch_live().await {
                        Ok(items) => { tx.send(Action::NtsLiveLoaded(items)).ok(); }
                        Err(e) => { tx.send(Action::ShowError(e.to_string())).ok(); }
                    }
                });
            }
            Action::NtsLiveLoaded(items) => {
                self.discovery_list.set_items(items);
            }
            // ... similar patterns for all other Load*/Loaded actions
            Action::ShowError(msg) => {
                self.error_message = Some(msg);
            }
            Action::ShowHelp => self.show_help = true,
            Action::HideHelp => self.show_help = false,
            _ => {
                // Forward to components for any action they care about
                self.tab_bar.update(&action)?;
                self.discovery_list.update(&action)?;
                self.now_playing.update(&action)?;
                self.play_controls.update(&action)?;
            }
        }
        Ok(())
    }
}
```

### 8.5 Tabs Component

```rust
// src/components/tabs.rs

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Line,
    widgets::Tabs,
    Frame,
};
use tokio::sync::mpsc::UnboundedSender;

use crate::action::Action;
use crate::components::Component;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PrimaryTab {
    Nts,
    SoundCloud,
}

pub struct TabBar {
    action_tx: Option<UnboundedSender<Action>>,
    pub active: PrimaryTab,
}

impl TabBar {
    pub fn new() -> Self {
        Self { action_tx: None, active: PrimaryTab::Nts }
    }
}

impl Component for TabBar {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) {
        self.action_tx = Some(tx);
    }

    fn update(&mut self, action: &Action) -> anyhow::Result<Vec<Action>> {
        match action {
            Action::NextTab => {
                self.active = match self.active {
                    PrimaryTab::Nts => PrimaryTab::SoundCloud,
                    PrimaryTab::SoundCloud => PrimaryTab::Nts,
                };
            }
            Action::PrevTab => {
                self.active = match self.active {
                    PrimaryTab::Nts => PrimaryTab::SoundCloud,
                    PrimaryTab::SoundCloud => PrimaryTab::Nts,
                };
            }
            _ => {}
        }
        Ok(vec![])
    }

    fn draw(&self, frame: &mut Frame, area: Rect) {
        let titles = vec![Line::from(" NTS "), Line::from(" SoundCloud ")];
        let selected = match self.active {
            PrimaryTab::Nts => 0,
            PrimaryTab::SoundCloud => 1,
        };
        let tabs = Tabs::new(titles)
            .select(selected)
            .highlight_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .divider("|");
        frame.render_widget(tabs, area);
    }
}
```

### 8.6 Discovery List Component

```rust
// src/components/discovery_list.rs

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};
use tokio::sync::mpsc::UnboundedSender;

use crate::action::Action;
use crate::api::models::DiscoveryItem;
use crate::components::Component;

pub struct DiscoveryList {
    action_tx: Option<UnboundedSender<Action>>,
    items: Vec<DiscoveryItem>,
    state: ListState,
    favorites: std::collections::HashSet<String>, // updated externally
}

impl DiscoveryList {
    pub fn new() -> Self {
        Self {
            action_tx: None,
            items: vec![],
            state: ListState::default(),
            favorites: std::collections::HashSet::new(),
        }
    }

    pub fn set_items(&mut self, items: Vec<DiscoveryItem>) {
        self.items = items;
        self.state.select(if self.items.is_empty() { None } else { Some(0) });
    }

    pub fn selected_item(&self) -> Option<&DiscoveryItem> {
        self.state.selected().and_then(|i| self.items.get(i))
    }

    pub fn set_favorites(&mut self, favorites: std::collections::HashSet<String>) {
        self.favorites = favorites;
    }

    fn next(&mut self) {
        if self.items.is_empty() { return; }
        let i = match self.state.selected() {
            Some(i) => (i + 1).min(self.items.len() - 1),
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn prev(&mut self) {
        if self.items.is_empty() { return; }
        let i = match self.state.selected() {
            Some(i) => i.saturating_sub(1),
            None => 0,
        };
        self.state.select(Some(i));
    }
}

impl Component for DiscoveryList {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) {
        self.action_tx = Some(tx);
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> anyhow::Result<bool> {
        let tx = self.action_tx.as_ref().unwrap();
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => { self.next(); Ok(true) }
            KeyCode::Up | KeyCode::Char('k') => { self.prev(); Ok(true) }
            KeyCode::Enter => {
                if let Some(item) = self.selected_item() {
                    match item {
                        DiscoveryItem::NtsShow { show_alias, .. } => {
                            tx.send(Action::LoadNtsShowEpisodes {
                                show_alias: show_alias.clone(),
                            })?;
                        }
                        _ => {
                            tx.send(Action::PlayItem(item.clone()))?;
                        }
                    }
                }
                Ok(true)
            }
            KeyCode::Char('a') => {
                if let Some(item) = self.selected_item() {
                    tx.send(Action::AddToQueue(item.clone()))?;
                }
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn draw(&self, frame: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self.items.iter().map(|item| {
            let is_fav = self.favorites.contains(&item.favorite_key());
            let heart = if is_fav { " ♥" } else { "" };
            let line = Line::from(vec![
                Span::styled(item.title(), Style::default().fg(Color::White)),
                Span::styled(heart, Style::default().fg(Color::Red)),
            ]);
            let subtitle = Line::from(Span::styled(
                item.subtitle(),
                Style::default().fg(Color::DarkGray),
            ));
            ListItem::new(vec![line, subtitle])
        }).collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::RIGHT))
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");

        frame.render_stateful_widget(list, area, &mut self.state.clone());
    }
}
```

### 8.7 Search Bar Component

```rust
// src/components/search_bar.rs

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use tokio::sync::mpsc::UnboundedSender;

use crate::action::Action;
use crate::components::Component;

pub struct SearchBar {
    action_tx: Option<UnboundedSender<Action>>,
    input: String,
    focused: bool,
}

impl SearchBar {
    pub fn new() -> Self {
        Self { action_tx: None, input: String::new(), focused: false }
    }

    pub fn is_focused(&self) -> bool {
        self.focused
    }
}

impl Component for SearchBar {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) {
        self.action_tx = Some(tx);
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> anyhow::Result<bool> {
        if !self.focused { return Ok(false); }
        let tx = self.action_tx.as_ref().unwrap();
        match key.code {
            KeyCode::Char(c) => {
                self.input.push(c);
                Ok(true)
            }
            KeyCode::Backspace => {
                self.input.pop();
                Ok(true)
            }
            KeyCode::Enter => {
                if !self.input.is_empty() {
                    tx.send(Action::SearchSubmit)?;
                }
                Ok(true)
            }
            KeyCode::Esc => {
                self.focused = false;
                self.input.clear();
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn update(&mut self, action: &Action) -> anyhow::Result<Vec<Action>> {
        match action {
            Action::FocusSearch => { self.focused = true; }
            Action::Back => { self.focused = false; self.input.clear(); }
            Action::SearchClear => { self.input.clear(); }
            _ => {}
        }
        Ok(vec![])
    }

    fn draw(&self, frame: &mut Frame, area: Rect) {
        let style = if self.focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let display = if self.input.is_empty() && !self.focused {
            "/ Search...".to_string()
        } else {
            format!("/ {}_", self.input)
        };

        let paragraph = Paragraph::new(display)
            .style(style)
            .block(Block::default().borders(Borders::TOP));
        frame.render_widget(paragraph, area);
    }
}
```

### 8.8 Now Playing Component

```rust
// src/components/now_playing.rs

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use tokio::sync::mpsc::UnboundedSender;

use crate::action::Action;
use crate::api::models::DiscoveryItem;
use crate::components::Component;

pub struct NowPlaying {
    action_tx: Option<UnboundedSender<Action>>,
    current_item: Option<DiscoveryItem>,
    position_secs: f64,
    paused: bool,
}

impl NowPlaying {
    pub fn new() -> Self {
        Self {
            action_tx: None,
            current_item: None,
            position_secs: 0.0,
            paused: false,
        }
    }
}

impl Component for NowPlaying {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) {
        self.action_tx = Some(tx);
    }

    fn update(&mut self, action: &Action) -> anyhow::Result<Vec<Action>> {
        match action {
            Action::PlayItem(item) => {
                self.current_item = Some(item.clone());
                self.position_secs = 0.0;
                self.paused = false;
            }
            Action::PlaybackPosition(pos) => {
                self.position_secs = *pos;
            }
            Action::TogglePlayPause => {
                self.paused = !self.paused;
            }
            Action::Stop | Action::PlaybackFinished => {
                self.current_item = None;
                self.position_secs = 0.0;
            }
            _ => {}
        }
        Ok(vec![])
    }

    fn draw(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(" Now Playing ")
            .borders(Borders::ALL);

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let Some(item) = &self.current_item else {
            let empty = Paragraph::new("Nothing playing")
                .style(Style::default().fg(Color::DarkGray));
            frame.render_widget(empty, inner);
            return;
        };

        let mins = self.position_secs as u64 / 60;
        let secs = self.position_secs as u64 % 60;
        let status = if self.paused { "⏸" } else { "▶" };

        let mut lines = vec![
            Line::from(Span::styled(
                item.title(),
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                item.subtitle(),
                Style::default().fg(Color::Cyan),
            )),
            Line::from(""),
            Line::from(format!("{} {}:{:02}", status, mins, secs)),
        ];

        // Add genre tags if available
        match item {
            DiscoveryItem::NtsEpisode { genres, description, .. }
            | DiscoveryItem::NtsLiveChannel { genres, .. } => {
                if !genres.is_empty() {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        format!("Tags: {}", genres.join(", ")),
                        Style::default().fg(Color::DarkGray),
                    )));
                }
                if let DiscoveryItem::NtsEpisode { description: Some(desc), .. } = item {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        desc.chars().take(200).collect::<String>(),
                        Style::default().fg(Color::DarkGray),
                    )));
                }
            }
            _ => {}
        }

        let paragraph = Paragraph::new(lines).wrap(Wrap { trim: true });
        frame.render_widget(paragraph, inner);
    }
}
```

### 8.9 Play Controls Component

```rust
// src/components/play_controls.rs

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use tokio::sync::mpsc::UnboundedSender;

use crate::action::Action;
use crate::components::Component;

pub struct PlayControls {
    action_tx: Option<UnboundedSender<Action>>,
    playing: bool,
    paused: bool,
    queue_pos: Option<usize>,
    queue_len: usize,
}

impl PlayControls {
    pub fn new() -> Self {
        Self {
            action_tx: None,
            playing: false,
            paused: false,
            queue_pos: None,
            queue_len: 0,
        }
    }
}

impl Component for PlayControls {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) {
        self.action_tx = Some(tx);
    }

    fn update(&mut self, action: &Action) -> anyhow::Result<Vec<Action>> {
        match action {
            Action::PlaybackStarted { .. } => { self.playing = true; self.paused = false; }
            Action::PlaybackFinished | Action::Stop => { self.playing = false; self.paused = false; }
            Action::TogglePlayPause => { self.paused = !self.paused; }
            _ => {}
        }
        Ok(vec![])
    }

    fn draw(&self, frame: &mut Frame, area: Rect) {
        let status = if self.paused {
            "⏸"
        } else if self.playing {
            "▶"
        } else {
            "■"
        };

        let queue_info = if self.queue_len > 0 {
            format!("Track {}/{}", self.queue_pos.unwrap_or(0) + 1, self.queue_len)
        } else {
            String::new()
        };

        let line1 = Line::from(vec![
            Span::styled(format!(" {} ", status), Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw(" Space "),
            Span::styled("Play/Pause", Style::default().fg(Color::DarkGray)),
            Span::raw("  n "),
            Span::styled("Next", Style::default().fg(Color::DarkGray)),
            Span::raw("  p "),
            Span::styled("Prev", Style::default().fg(Color::DarkGray)),
            Span::raw("  f "),
            Span::styled("Fav", Style::default().fg(Color::DarkGray)),
            Span::raw("  / "),
            Span::styled("Search", Style::default().fg(Color::DarkGray)),
        ]);

        let line2 = Line::from(vec![
            Span::raw("   Tab "),
            Span::styled("Switch", Style::default().fg(Color::DarkGray)),
            Span::raw("  q "),
            Span::styled("Quit", Style::default().fg(Color::DarkGray)),
            Span::raw("  ? "),
            Span::styled("Help", Style::default().fg(Color::DarkGray)),
            Span::raw("                    "),
            Span::styled(queue_info, Style::default().fg(Color::Cyan)),
        ]);

        let paragraph = Paragraph::new(vec![line1, line2])
            .block(Block::default().borders(Borders::TOP));
        frame.render_widget(paragraph, area);
    }
}
```

### 8.10 NTS Tab Coordinator

```rust
// src/components/nts/mod.rs

pub mod live;
pub mod collections;
pub mod shows;
pub mod mixtapes;
pub mod schedule;

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Line,
    widgets::Tabs,
    Frame,
};
use strum::{Display, EnumIter, IntoEnumIterator};
use tokio::sync::mpsc::UnboundedSender;

use crate::action::Action;
use crate::api::models::DiscoveryItem;
use crate::components::Component;

#[derive(Debug, Clone, Copy, PartialEq, EnumIter, Display)]
pub enum NtsSubTab {
    Live,
    Picks,
    Recent,
    Mixtapes,
    Shows,
    Schedule,
    Favorites,
    History,
}

pub struct NtsTab {
    action_tx: Option<UnboundedSender<Action>>,
    pub active_sub: NtsSubTab,
    loaded: std::collections::HashSet<String>, // track which sub-tabs have been loaded
}

impl NtsTab {
    pub fn new() -> Self {
        Self {
            action_tx: None,
            active_sub: NtsSubTab::Live,
            loaded: std::collections::HashSet::new(),
        }
    }

    /// Switch to a sub-tab by index (0-based).
    pub fn switch_sub_tab(&mut self, index: usize) -> Vec<Action> {
        let tabs: Vec<NtsSubTab> = NtsSubTab::iter().collect();
        if let Some(&tab) = tabs.get(index) {
            self.active_sub = tab;
            self.load_if_needed()
        } else {
            vec![]
        }
    }

    /// Return the load action if this sub-tab hasn't been loaded yet.
    fn load_if_needed(&mut self) -> Vec<Action> {
        let key = format!("{:?}", self.active_sub);
        if self.loaded.contains(&key) {
            return vec![];
        }
        self.loaded.insert(key);

        match self.active_sub {
            NtsSubTab::Live => vec![Action::LoadNtsLive],
            NtsSubTab::Picks => vec![Action::LoadNtsPicks],
            NtsSubTab::Recent => vec![Action::LoadNtsRecent { offset: 0 }],
            NtsSubTab::Mixtapes => vec![Action::LoadNtsMixtapes],
            NtsSubTab::Shows => vec![Action::LoadNtsShows { offset: 0 }],
            NtsSubTab::Schedule => vec![Action::LoadNtsLive], // schedule from live endpoint
            NtsSubTab::Favorites => vec![], // loaded from DB, not API
            NtsSubTab::History => vec![],
        }
    }
}

impl Component for NtsTab {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) {
        self.action_tx = Some(tx);
    }

    fn update(&mut self, action: &Action) -> anyhow::Result<Vec<Action>> {
        match action {
            Action::SwitchSubTab(idx) => {
                return Ok(self.switch_sub_tab(*idx));
            }
            _ => {}
        }
        Ok(vec![])
    }

    fn draw(&self, frame: &mut Frame, area: Rect) {
        let titles: Vec<Line> = NtsSubTab::iter()
            .map(|t| Line::from(format!(" {} ", t)))
            .collect();
        let selected = NtsSubTab::iter()
            .position(|t| t == self.active_sub)
            .unwrap_or(0);

        let tabs = Tabs::new(titles)
            .select(selected)
            .highlight_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .divider("|");
        frame.render_widget(tabs, area);
    }
}
```

### 8.11 NTS Live View

```rust
// src/components/nts/live.rs

use crate::action::Action;
use crate::api::models::DiscoveryItem;

/// Convert NTS live data to discovery items.
/// This is a data-only module — the DiscoveryList component handles rendering.
/// Live items include both channels with their current shows.
pub fn live_items_from_action(action: &Action) -> Option<Vec<DiscoveryItem>> {
    match action {
        Action::NtsLiveLoaded(items) => Some(items.clone()),
        _ => None,
    }
}
```

### 8.12 NTS Collections View

```rust
// src/components/nts/collections.rs

use crate::action::Action;
use crate::api::models::DiscoveryItem;

/// Handle Picks/Recent data arriving.
/// These are data-only — DiscoveryList renders them.
pub fn picks_items_from_action(action: &Action) -> Option<Vec<DiscoveryItem>> {
    match action {
        Action::NtsPicksLoaded(items) => Some(items.clone()),
        _ => None,
    }
}

pub fn recent_items_from_action(action: &Action) -> Option<Vec<DiscoveryItem>> {
    match action {
        Action::NtsRecentLoaded(items) => Some(items.clone()),
        _ => None,
    }
}
```

### 8.13 NTS Shows View

```rust
// src/components/nts/shows.rs

use crate::action::Action;
use crate::api::models::DiscoveryItem;

/// Shows view state — supports drill-down into a show's episodes.
pub struct ShowsView {
    /// When Some, we're viewing a show's episodes. When None, viewing the shows list.
    pub drill_down_alias: Option<String>,
    /// The top-level shows list (cached).
    pub shows: Vec<DiscoveryItem>,
    /// Episodes for the drilled-down show.
    pub episodes: Vec<DiscoveryItem>,
}

impl ShowsView {
    pub fn new() -> Self {
        Self {
            drill_down_alias: None,
            shows: vec![],
            episodes: vec![],
        }
    }

    /// Returns the currently visible items (shows or episodes).
    pub fn visible_items(&self) -> &[DiscoveryItem] {
        if self.drill_down_alias.is_some() {
            &self.episodes
        } else {
            &self.shows
        }
    }

    /// Handle actions related to shows.
    pub fn update(&mut self, action: &Action) -> Vec<Action> {
        match action {
            Action::NtsShowsLoaded(items) => {
                self.shows = items.clone();
            }
            Action::NtsShowEpisodesLoaded { show_alias, episodes } => {
                self.drill_down_alias = Some(show_alias.clone());
                self.episodes = episodes.clone();
            }
            Action::Back if self.drill_down_alias.is_some() => {
                self.drill_down_alias = None;
                self.episodes.clear();
                // Return the shows list to the discovery list
            }
            _ => {}
        }
        vec![]
    }
}
```

### 8.14 NTS Mixtapes View

```rust
// src/components/nts/mixtapes.rs

use crate::action::Action;
use crate::api::models::DiscoveryItem;

/// Handle mixtape data. Mixtapes are direct streams — no drill-down needed.
pub fn mixtape_items_from_action(action: &Action) -> Option<Vec<DiscoveryItem>> {
    match action {
        Action::NtsMixtapesLoaded(items) => Some(items.clone()),
        _ => None,
    }
}
```

### 8.15 NTS Schedule View

```rust
// src/components/nts/schedule.rs

use crate::api::models::{DiscoveryItem, NtsChannel};

/// Build schedule items from the live endpoint's "next" through "next17" broadcasts.
/// Both channels' upcoming broadcasts are interleaved and sorted by start time.
pub fn schedule_from_channels(channels: &[NtsChannel]) -> Vec<DiscoveryItem> {
    let mut items = Vec::new();

    for channel in channels {
        let ch_num: u8 = channel.channel_name.parse().unwrap_or(1);
        for broadcast in channel.upcoming() {
            let detail = broadcast.embeds.as_ref()
                .and_then(|e| e.details.as_ref());

            items.push(DiscoveryItem::NtsLiveChannel {
                channel: ch_num,
                show_name: detail.map_or_else(
                    || broadcast.broadcast_title.clone(),
                    |d| d.name.clone(),
                ),
                broadcast_title: broadcast.broadcast_title.clone(),
                genres: detail
                    .and_then(|d| d.genres.as_ref())
                    .map_or_else(Vec::new, |g| g.iter().map(|g| g.value.clone()).collect()),
                start: broadcast.start_timestamp.clone(),
                end: broadcast.end_timestamp.clone(),
            });
        }
    }

    // Sort by start timestamp
    items.sort_by(|a, b| {
        let a_start = match a { DiscoveryItem::NtsLiveChannel { start, .. } => start, _ => "" };
        let b_start = match b { DiscoveryItem::NtsLiveChannel { start, .. } => start, _ => "" };
        a_start.cmp(b_start)
    });

    items
}
```

### 8.16 SoundCloud Tab Coordinator

```rust
// src/components/soundcloud/mod.rs

pub mod search;
pub mod favorites;

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Line,
    widgets::Tabs,
    Frame,
};
use tokio::sync::mpsc::UnboundedSender;

use crate::action::Action;
use crate::components::Component;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScSubTab {
    Search,
    Favorites,
}

pub struct SoundCloudTab {
    action_tx: Option<UnboundedSender<Action>>,
    pub active_sub: ScSubTab,
    pub initialized: bool,
}

impl SoundCloudTab {
    pub fn new() -> Self {
        Self {
            action_tx: None,
            active_sub: ScSubTab::Search,
            initialized: false,
        }
    }
}

impl Component for SoundCloudTab {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) {
        self.action_tx = Some(tx);
    }

    fn update(&mut self, action: &Action) -> anyhow::Result<Vec<Action>> {
        match action {
            Action::SwitchSubTab(0) => { self.active_sub = ScSubTab::Search; }
            Action::SwitchSubTab(1) => {
                self.active_sub = ScSubTab::Favorites;
                // Trigger likes load if authenticated
                if let Some(tx) = &self.action_tx {
                    tx.send(Action::LoadSoundCloudLikes { offset: 0 }).ok();
                }
            }
            _ => {}
        }
        Ok(vec![])
    }

    fn draw(&self, frame: &mut Frame, area: Rect) {
        let titles = vec![Line::from(" Search "), Line::from(" Favorites ")];
        let selected = match self.active_sub {
            ScSubTab::Search => 0,
            ScSubTab::Favorites => 1,
        };
        let tabs = Tabs::new(titles)
            .select(selected)
            .highlight_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .divider("|");
        frame.render_widget(tabs, area);
    }
}
```

### 8.17 SoundCloud Search View

```rust
// src/components/soundcloud/search.rs

use crate::action::Action;
use crate::api::models::DiscoveryItem;

/// Handle SoundCloud search results arriving.
/// Search is triggered by SearchSubmit when the SoundCloud tab is active.
pub fn search_items_from_action(action: &Action) -> Option<Vec<DiscoveryItem>> {
    match action {
        Action::SoundCloudSearchLoaded(items) => Some(items.clone()),
        _ => None,
    }
}

/// Convert SoundCloud tracks to DiscoveryItems.
pub fn tracks_to_discovery(tracks: Vec<crate::api::models::SoundCloudTrack>) -> Vec<DiscoveryItem> {
    tracks.into_iter().map(|t| {
        DiscoveryItem::SoundCloudTrack {
            title: t.title,
            artist: t.user.username,
            permalink_url: t.permalink_url,
            duration_ms: t.duration,
            genre: t.genre,
            playback_count: t.playback_count,
        }
    }).collect()
}
```

### 8.18 SoundCloud Favorites View

```rust
// src/components/soundcloud/favorites.rs

use crate::action::Action;
use crate::api::models::DiscoveryItem;

/// Handle SoundCloud likes data arriving.
/// If no OAuth token is set, the app shows a message instead of items.
pub fn favorites_items_from_action(action: &Action) -> Option<Vec<DiscoveryItem>> {
    match action {
        Action::SoundCloudLikesLoaded(items) => Some(items.clone()),
        _ => None,
    }
}

/// Message to show when no OAuth token is available.
pub const NO_AUTH_MESSAGE: &str = "Run `clisten auth soundcloud` to see your liked tracks.";
```

---
