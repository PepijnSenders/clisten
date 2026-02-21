// src/app.rs

use std::collections::HashSet;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders};
use tokio::sync::mpsc;

use crate::action::Action;
use crate::api::nts::NtsClient;
use crate::components::Component;
use crate::components::discovery_list::DiscoveryList;
use crate::components::search_bar::SearchBar;
use crate::components::now_playing::NowPlaying;
use crate::components::play_controls::PlayControls;
use crate::components::nts::{NtsTab, NtsSubTab};
use crate::components::direct_play_modal::DirectPlayModal;
use crate::config::Config;
use crate::db::Database;
use crate::player::MpvPlayer;
use crate::player::queue::{Queue, QueueItem};
use crate::tui::{Tui, TuiEvent};

pub struct App {
    running: bool,
    action_tx: mpsc::UnboundedSender<Action>,
    action_rx: mpsc::UnboundedReceiver<Action>,

    // Components
    pub nts_tab: NtsTab,
    pub discovery_list: DiscoveryList,
    pub search_bar: SearchBar,
    pub now_playing: NowPlaying,
    pub play_controls: PlayControls,
    pub direct_play_modal: DirectPlayModal,

    // State
    nts_client: NtsClient,
    player: MpvPlayer,
    db: Database,
    pub config: Config,
    pub favorites: HashSet<String>,
    pub queue: Queue,
    pub show_help: bool,
    pub error_message: Option<String>,
    pub search_id: u64,
    /// true when viewing genre search results (not the genre list)
    pub viewing_genre_results: bool,
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

        let mut nts_tab = NtsTab::new();
        let mut discovery_list = DiscoveryList::new();
        let mut search_bar = SearchBar::new();
        let mut now_playing = NowPlaying::new();
        let mut play_controls = PlayControls::new();
        let mut direct_play_modal = DirectPlayModal::new();

        // Register action handlers
        nts_tab.register_action_handler(action_tx.clone());
        discovery_list.register_action_handler(action_tx.clone());
        search_bar.register_action_handler(action_tx.clone());
        now_playing.register_action_handler(action_tx.clone());
        play_controls.register_action_handler(action_tx.clone());
        direct_play_modal.register_action_handler(action_tx.clone());

        // Sync initial favorites to DiscoveryList for ♥ rendering
        discovery_list.set_favorites(favorites.clone());

        let mut player = MpvPlayer::new();
        player.set_action_tx(action_tx.clone());

        Ok(Self {
            running: true,
            action_tx,
            action_rx,
            nts_tab,
            discovery_list,
            search_bar,
            now_playing,
            play_controls,
            direct_play_modal,
            nts_client: NtsClient::new(),
            player,
            db,
            config,
            favorites,
            queue: Queue::new(),
            show_help: false,
            error_message: None,
            search_id: 0,
            viewing_genre_results: false,
        })
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        let mut tui = Tui::new(self.config.general.frame_rate)?;
        tui.enter()?;

        // Load initial data
        self.action_tx.send(Action::LoadNtsLive)?;

        while self.running {
            // Draw
            let discovery_list = &self.discovery_list;
            let nts_tab = &self.nts_tab;
            let search_bar = &self.search_bar;
            let now_playing = &self.now_playing;
            let play_controls = &self.play_controls;
            let direct_play_modal = &self.direct_play_modal;

            let error_msg = self.error_message.clone();
            let show_help = self.show_help;
            tui.draw(|frame| {
                let error_height = if error_msg.is_some() { 1 } else { 0 };
                let outer = Layout::vertical([
                    Constraint::Min(0),
                    Constraint::Length(error_height),
                    Constraint::Length(4),
                ]).split(frame.area());

                // Draw outer frame around main content area
                let outer_block = Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray));
                let content_area = outer_block.inner(outer[0]);
                frame.render_widget(outer_block, outer[0]);

                let main = Layout::horizontal([
                    Constraint::Percentage(60),
                    Constraint::Percentage(40),
                ]).split(content_area);

                let left = Layout::vertical([
                    Constraint::Length(2),
                    Constraint::Min(0),
                    Constraint::Length(2),
                ]).split(main[0]);

                // Sub-tab bar
                nts_tab.draw(frame, left[0]);
                discovery_list.draw(frame, left[1]);
                // Search bar: first row is separator, second row is input
                let search_input_area = Rect {
                    x: left[2].x,
                    y: left[2].y + 1,
                    width: left[2].width,
                    height: 1,
                };
                search_bar.draw(frame, search_input_area);
                now_playing.draw(frame, main[1]);

                // Draw manual separators with proper box-drawing corners
                let buf = frame.buffer_mut();

                // Vertical divider between left and right panels
                let divider_x = main[0].x + main[0].width;
                if divider_x < content_area.x + content_area.width {
                    let top_y = content_area.y;
                    let bottom_y = content_area.y + content_area.height;
                    // Top corner: ┬ (connects to outer frame top border)
                    if let Some(cell) = buf.cell_mut((divider_x, top_y.saturating_sub(1))) {
                        cell.set_char('┬');
                        cell.set_fg(Color::DarkGray);
                    }
                    // Vertical line
                    for y in top_y..bottom_y {
                        if let Some(cell) = buf.cell_mut((divider_x, y)) {
                            cell.set_char('│');
                            cell.set_fg(Color::DarkGray);
                        }
                    }
                    // Bottom corner: ┴ (connects to outer frame bottom border)
                    if let Some(cell) = buf.cell_mut((divider_x, bottom_y)) {
                        cell.set_char('┴');
                        cell.set_fg(Color::DarkGray);
                    }
                }

                // Horizontal divider above search bar
                let sep_y = left[2].y;
                {
                    // Left corner: ├
                    let left_x = content_area.x.saturating_sub(1);
                    if let Some(cell) = buf.cell_mut((left_x, sep_y)) {
                        cell.set_char('├');
                        cell.set_fg(Color::DarkGray);
                    }
                    // Horizontal line
                    for x in content_area.x..main[0].x + main[0].width {
                        if let Some(cell) = buf.cell_mut((x, sep_y)) {
                            cell.set_char('─');
                            cell.set_fg(Color::DarkGray);
                        }
                    }
                    // Right corner: ┤ at vertical divider
                    if divider_x < content_area.x + content_area.width {
                        if let Some(cell) = buf.cell_mut((divider_x, sep_y)) {
                            cell.set_char('┤');
                            cell.set_fg(Color::DarkGray);
                        }
                    }
                }

                // Error line above play controls
                if let Some(ref msg) = error_msg {
                    use ratatui::widgets::Paragraph;
                    use ratatui::text::{Line, Span};
                    let error_line = Line::from(vec![
                        Span::styled(" ⚠ ", Style::default().fg(Color::Red)),
                        Span::styled(msg.as_str(), Style::default().fg(Color::Yellow)),
                        Span::styled("  Press r to retry.", Style::default().fg(Color::DarkGray)),
                    ]);
                    frame.render_widget(Paragraph::new(error_line), outer[1]);
                }

                play_controls.draw(frame, outer[2]);

                // Direct play modal overlay
                if direct_play_modal.is_visible() {
                    direct_play_modal.draw(frame, frame.area());
                }

                // Help overlay
                if show_help {
                    use ratatui::widgets::{Block, Borders, Clear, Paragraph};
                    use ratatui::style::{Color, Style, Modifier};
                    use ratatui::text::{Line, Span};
                    use ratatui::layout::Alignment;

                    let area = frame.area();
                    let overlay_width = 58u16;
                    let overlay_height = 24u16;
                    let x = area.width.saturating_sub(overlay_width) / 2;
                    let y = area.height.saturating_sub(overlay_height) / 2;
                    let overlay_area = ratatui::layout::Rect::new(x, y, overlay_width.min(area.width), overlay_height.min(area.height));

                    frame.render_widget(Clear, overlay_area);

                    let keybindings = vec![
                        ("q",         "Quit"),
                        ("1–3",       "Switch sub-tab"),
                        ("Tab",       "Next sub-tab"),
                        ("Shift+Tab", "Previous sub-tab"),
                        ("j / Down",  "Scroll down"),
                        ("k / Up",    "Scroll up"),
                        ("Enter",     "Play / select genre"),
                        ("a",         "Add to queue"),
                        ("A",         "Add to queue next (after current)"),
                        ("Space",     "Toggle play/pause"),
                        ("n",         "Next track in queue"),
                        ("p",         "Previous track in queue"),
                        ("s",         "Stop playback"),
                        ("o",         "Open URL (direct play)"),
                        ("/",         "Focus search bar"),
                        ("Escape",    "Unfocus search / go back"),
                        ("f",         "Toggle favorite"),
                        ("c",         "Clear queue"),
                        ("[ ]",       "Volume down/up"),
                        ("?",         "Toggle this help overlay"),
                        ("r",         "Retry failed request"),
                    ];

                    let mut lines: Vec<Line> = vec![
                        Line::from(Span::styled(" Keybindings ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
                        Line::from(""),
                    ];
                    for (key, desc) in &keybindings {
                        lines.push(Line::from(vec![
                            Span::styled(format!("  {:12}", key), Style::default().fg(Color::Yellow)),
                            Span::raw(*desc),
                        ]));
                    }
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled("  Press any key to close", Style::default().fg(Color::DarkGray))));

                    let block = Block::default()
                        .borders(Borders::ALL)
                        .title(" Help ")
                        .title_alignment(Alignment::Center);
                    let paragraph = Paragraph::new(lines).block(block);
                    frame.render_widget(paragraph, overlay_area);
                }
            })?;

            // Process events
            tokio::select! {
                Some(event) = tui.event_rx.recv() => {
                    match event {
                        TuiEvent::Key(key) => self.handle_key(key)?,
                        TuiEvent::Resize(w, h) => {
                            self.action_tx.send(Action::Resize(w, h))?;
                        }
                        TuiEvent::Tick => { self.action_tx.send(Action::Tick)?; }
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

    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> anyhow::Result<()> {
        use KeyCode::*;

        // Any key dismisses help overlay
        if self.show_help {
            self.action_tx.send(Action::HideHelp)?;
            return Ok(());
        }

        // Direct play modal intercepts all keys when visible
        if self.direct_play_modal.is_visible() {
            self.direct_play_modal.handle_key_event(key)?;
            return Ok(());
        }

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
            (Char('o'), KeyModifiers::NONE) if !self.search_bar.is_focused() => {
                self.action_tx.send(Action::OpenDirectPlay)?;
            }
            (Tab, _) => {
                let next = (self.nts_tab.active_index() + 1) % 3;
                self.action_tx.send(Action::SwitchSubTab(next))?;
            }
            (BackTab, _) => {
                let prev = (self.nts_tab.active_index() + 2) % 3;
                self.action_tx.send(Action::SwitchSubTab(prev))?;
            }
            (Char(' '), KeyModifiers::NONE) if !self.search_bar.is_focused() => {
                self.action_tx.send(Action::TogglePlayPause)?;
            }
            (Char('n'), KeyModifiers::NONE) if !self.search_bar.is_focused() => {
                self.action_tx.send(Action::NextTrack)?;
            }
            (Char('p'), KeyModifiers::NONE) if !self.search_bar.is_focused() => {
                self.action_tx.send(Action::PrevTrack)?;
            }
            (Char('s'), KeyModifiers::NONE) if !self.search_bar.is_focused() => {
                self.action_tx.send(Action::Stop)?;
            }
            (Char('/'), KeyModifiers::NONE) if !self.search_bar.is_focused() => {
                self.action_tx.send(Action::FocusSearch)?;
            }
            (Char('f'), KeyModifiers::NONE) if !self.search_bar.is_focused() => {
                self.action_tx.send(Action::ToggleFavorite)?;
            }
            (Char('c'), KeyModifiers::NONE) if !self.search_bar.is_focused() => {
                self.action_tx.send(Action::ClearQueue)?;
            }
            (Char('a'), KeyModifiers::NONE) if !self.search_bar.is_focused() => {
                if let Some(item) = self.discovery_list.selected_item() {
                    self.action_tx.send(Action::AddToQueue(item.clone()))?;
                }
            }
            (Char('A'), _) if !self.search_bar.is_focused() => {
                if let Some(item) = self.discovery_list.selected_item() {
                    self.action_tx.send(Action::AddToQueueNext(item.clone()))?;
                }
            }
            (Char(']'), KeyModifiers::NONE) if !self.search_bar.is_focused() => {
                self.action_tx.send(Action::VolumeUp)?;
            }
            (Char('['), KeyModifiers::NONE) if !self.search_bar.is_focused() => {
                self.action_tx.send(Action::VolumeDown)?;
            }
            (Char('r'), KeyModifiers::NONE) if !self.search_bar.is_focused() && self.error_message.is_some() => {
                self.action_tx.send(Action::LoadNtsLive)?;
                self.error_message = None;
            }
            (Char(c), KeyModifiers::NONE) if !self.search_bar.is_focused() && c.is_ascii_digit() => {
                let idx = c.to_digit(10).unwrap_or(0) as usize;
                if idx >= 1 && idx <= 3 {
                    self.action_tx.send(Action::SwitchSubTab(idx - 1))?;
                }
            }
            (Esc, _) => self.action_tx.send(Action::Back)?,
            _ => {
                if self.search_bar.is_focused() {
                    self.search_bar.handle_key_event(key)?;
                } else {
                    self.discovery_list.handle_key_event(key)?;
                }
            }
        }
        Ok(())
    }

    pub async fn handle_action(&mut self, action: Action) -> anyhow::Result<()> {
        match action {
            Action::Quit => {
                let _ = self.player.stop().await;
                self.running = false;
            }
            Action::PlayItem(ref item) => {
                if let Some(url) = item.playback_url() {
                    let nothing_playing = self.now_playing.current_item.is_none();
                    let new_index = self.queue.len();
                    self.queue.add(QueueItem { item: item.clone(), url: url.clone(), stream_title: None });
                    self.play_controls.queue_pos = self.queue.current_index();
                    self.play_controls.queue_len = self.queue.len();
                    self.sync_queue_to_now_playing();

                    if nothing_playing {
                        self.queue.play_at(new_index);
                        self.play_controls.queue_pos = self.queue.current_index();
                        self.now_playing.update(&action)?;
                        self.player.play(&url).await?;
                        self.action_tx.send(Action::PlaybackStarted {
                            title: item.display_title(),
                            url,
                        })?;
                        self.action_tx.send(Action::AddToHistory(item.clone()))?;
                        self.sync_queue_to_now_playing();
                    }
                }
            }
            Action::TogglePlayPause => {
                let _ = self.player.toggle_pause().await;
            }
            Action::Stop => {
                let _ = self.player.stop().await;
            }
            Action::ToggleFavorite => {
                if let Some(item) = self.discovery_list.selected_item().cloned() {
                    let key = item.favorite_key();
                    if self.favorites.contains(&key) {
                        self.db.remove_favorite(&key)?;
                        self.favorites.remove(&key);
                    } else {
                        self.db.add_favorite(&item)?;
                        self.favorites.insert(key);
                    }
                    self.discovery_list.set_favorites(self.favorites.clone());
                }
            }
            Action::AddToHistory(ref item) => {
                let _ = self.db.add_to_history(item);
            }
            Action::AddToQueue(ref item) => {
                let url = item.playback_url().unwrap_or_default();
                self.queue.add(QueueItem { item: item.clone(), url, stream_title: None });
                self.play_controls.queue_pos = self.queue.current_index();
                self.play_controls.queue_len = self.queue.len();
                self.sync_queue_to_now_playing();
            }
            Action::AddToQueueNext(ref item) => {
                let url = item.playback_url().unwrap_or_default();
                self.queue.add_next(QueueItem { item: item.clone(), url, stream_title: None });
                self.play_controls.queue_pos = self.queue.current_index();
                self.play_controls.queue_len = self.queue.len();
                self.sync_queue_to_now_playing();
            }
            Action::ClearQueue => {
                self.queue.clear();
                self.play_controls.queue_pos = None;
                self.play_controls.queue_len = 0;
                self.sync_queue_to_now_playing();
            }
            Action::NextTrack => {
                if let Some(next) = self.queue.next() {
                    let url = next.url.clone();
                    let title = next.item.display_title();
                    let item = next.item.clone();
                    self.play_controls.queue_pos = self.queue.current_index();
                    self.now_playing.buffering = true;
                    self.now_playing.stream_metadata = None;
                    self.now_playing.current_item = Some(item);
                    self.play_controls.buffering = true;
                    self.sync_queue_to_now_playing();
                    if let Err(e) = self.player.play(&url).await {
                        self.action_tx.send(Action::ShowError(e.to_string()))?;
                    } else {
                        self.action_tx.send(Action::PlaybackStarted { title, url })?;
                    }
                }
            }
            Action::PrevTrack => {
                if let Some(prev) = self.queue.prev() {
                    let url = prev.url.clone();
                    let title = prev.item.display_title();
                    let item = prev.item.clone();
                    self.play_controls.queue_pos = self.queue.current_index();
                    self.now_playing.buffering = true;
                    self.now_playing.stream_metadata = None;
                    self.now_playing.current_item = Some(item);
                    self.play_controls.buffering = true;
                    self.sync_queue_to_now_playing();
                    if let Err(e) = self.player.play(&url).await {
                        self.action_tx.send(Action::ShowError(e.to_string()))?;
                    } else {
                        self.action_tx.send(Action::PlaybackStarted { title, url })?;
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
            Action::LoadNtsPicks => {
                let tx = self.action_tx.clone();
                let client = self.nts_client.clone();
                tokio::spawn(async move {
                    match client.fetch_picks().await {
                        Ok(items) => { tx.send(Action::NtsPicksLoaded(items)).ok(); }
                        Err(e) => { tx.send(Action::ShowError(e.to_string())).ok(); }
                    }
                });
            }
            Action::NtsPicksLoaded(items) => {
                self.discovery_list.set_items(items);
            }
            Action::LoadGenres => {
                use crate::api::models::DiscoveryItem;
                use crate::api::nts::TOP_GENRES;
                let db_favorites = self.db.list_favorites(None, 1000, 0)?;
                let db_history = self.db.list_history(1000, 0)?;
                let fav_count = db_favorites.len();
                let hist_count = db_history.len();
                let mut items: Vec<DiscoveryItem> = Vec::new();
                // Prepend special items
                items.push(DiscoveryItem::NtsGenre {
                    name: format!("My Favorites ({})", fav_count),
                    genre_id: "_favorites".to_string(),
                });
                items.push(DiscoveryItem::NtsGenre {
                    name: format!("History ({})", hist_count),
                    genre_id: "_history".to_string(),
                });
                // Static genre list (500+ episodes each)
                for &(id, name) in TOP_GENRES {
                    items.push(DiscoveryItem::NtsGenre {
                        name: name.to_string(),
                        genre_id: id.to_string(),
                    });
                }
                self.action_tx.send(Action::GenresLoaded(items))?;
                self.viewing_genre_results = false;
            }
            Action::GenresLoaded(items) => {
                self.discovery_list.set_items(items);
                self.viewing_genre_results = false;
            }
            Action::SearchByGenre { genre_id, genre_name: _ } => {
                self.search_id += 1;
                let current_search_id = self.search_id;
                self.discovery_list.set_items(vec![]);
                self.discovery_list.loading = true;
                self.viewing_genre_results = true;

                if genre_id == "_favorites" {
                    // Load favorites from DB
                    let records = self.db.list_favorites(None, 1000, 0)?;
                    let items: Vec<_> = records.iter().map(|r| r.to_discovery_item()).collect();
                    self.action_tx.send(Action::SearchResultsPartial {
                        search_id: current_search_id,
                        items,
                        done: true,
                    })?;
                } else if genre_id == "_history" {
                    // Load history from DB
                    let records = self.db.list_history(1000, 0)?;
                    let items: Vec<_> = records.iter().map(|r| r.to_discovery_item()).collect();
                    self.action_tx.send(Action::SearchResultsPartial {
                        search_id: current_search_id,
                        items,
                        done: true,
                    })?;
                } else {
                    // Server-side genre search via /api/v2/search/episodes
                    let tx = self.action_tx.clone();
                    let client = self.nts_client.clone();
                    let gid = genre_id.clone();
                    tokio::spawn(async move {
                        let page_size = 12u64;
                        let max_offset = 240u64; // API caps at offset 240
                        let mut all_items = Vec::new();

                        let mut offset = 0u64;
                        while offset <= max_offset {
                            match client.search_episodes(&gid, offset, page_size).await {
                                Ok(items) => {
                                    let got = items.len();
                                    all_items.extend(items);
                                    if (got as u64) < page_size {
                                        break; // no more results
                                    }
                                }
                                Err(_) => break,
                            }
                            offset += page_size;

                            // Send partial results every few pages
                            if all_items.len() >= 48 || offset > max_offset {
                                let batch = std::mem::take(&mut all_items);
                                let done = offset > max_offset;
                                tx.send(Action::SearchResultsPartial {
                                    search_id: current_search_id,
                                    items: batch,
                                    done,
                                }).ok();
                            }
                        }

                        // Send any remaining items
                        if !all_items.is_empty() {
                            tx.send(Action::SearchResultsPartial {
                                search_id: current_search_id,
                                items: all_items,
                                done: true,
                            }).ok();
                        } else {
                            // Ensure done signal is sent
                            tx.send(Action::SearchResultsPartial {
                                search_id: current_search_id,
                                items: vec![],
                                done: true,
                            }).ok();
                        }
                    });
                }
            }
            Action::SearchResultsPartial { search_id, items, done } => {
                if search_id != self.search_id {
                    return Ok(()); // stale results
                }
                if !items.is_empty() {
                    self.discovery_list.append_items(items);
                }
                if done {
                    self.discovery_list.loading = false;
                }
            }
            Action::SwitchSubTab(idx) => {
                // Clear current list immediately so old data doesn't show
                self.discovery_list.set_items(vec![]);
                self.discovery_list.loading = true;
                self.viewing_genre_results = false;
                // Clear any active filter
                self.discovery_list.set_filter(None);
                self.search_bar.update(&Action::Back)?;
                // Route through NtsTab coordinator
                let actions = self.nts_tab.switch_sub_tab(idx);
                if actions.is_empty() {
                    // Already loaded before — force reload
                    match self.nts_tab.active_sub {
                        NtsSubTab::Live => self.action_tx.send(Action::LoadNtsLive)?,
                        NtsSubTab::Picks => self.action_tx.send(Action::LoadNtsPicks)?,
                        NtsSubTab::Search => self.action_tx.send(Action::LoadGenres)?,
                    }
                } else {
                    for a in actions { self.action_tx.send(a)?; }
                }
            }
            Action::SearchSubmit => {
                let query = self.search_bar.input().to_string();
                if !query.is_empty() {
                    self.action_tx.send(Action::FilterList(query))?;
                } else {
                    self.action_tx.send(Action::ClearFilter)?;
                }
            }
            Action::OpenDirectPlay => {
                self.direct_play_modal.show();
            }
            Action::CloseDirectPlay => {
                self.direct_play_modal.hide();
            }
            // ─────────────────────────────────────────────────────────────────
            Action::PlaybackStarted { .. } => {
                self.now_playing.update(&action)?;
                self.play_controls.update(&action)?;
            }
            Action::PlaybackLoading => {
                self.now_playing.update(&action)?;
                self.play_controls.update(&action)?;
            }
            Action::PlaybackPosition(_) => {
                self.now_playing.update(&action)?;
                self.play_controls.update(&action)?;
            }
            Action::StreamMetadataChanged(ref title) => {
                self.queue.set_current_stream_title(title.clone());
                self.now_playing.update(&action)?;
                self.sync_queue_to_now_playing();
            }
            Action::PlaybackFinished => {
                self.now_playing.update(&action)?;
                self.play_controls.update(&action)?;
                // Auto-advance queue if there's a next track
                if let Some(next) = self.queue.next() {
                    let url = next.url.clone();
                    let title = next.item.display_title();
                    let item = next.item.clone();
                    self.play_controls.queue_pos = self.queue.current_index();
                    self.now_playing.buffering = true;
                    self.now_playing.stream_metadata = None;
                    self.now_playing.current_item = Some(item);
                    self.play_controls.buffering = true;
                    self.sync_queue_to_now_playing();
                    if let Err(e) = self.player.play(&url).await {
                        self.action_tx.send(Action::ShowError(e.to_string()))?;
                    } else {
                        self.action_tx.send(Action::PlaybackStarted { title, url })?;
                    }
                }
            }
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
            Action::VolumeUp => {
                let _ = self.player.set_volume(5.0).await;
                if let Ok(vol) = self.player.get_volume().await {
                    self.action_tx.send(Action::VolumeChanged(vol.round().clamp(0.0, 100.0) as u8))?;
                }
            }
            Action::VolumeDown => {
                let _ = self.player.set_volume(-5.0).await;
                if let Ok(vol) = self.player.get_volume().await {
                    self.action_tx.send(Action::VolumeChanged(vol.round().clamp(0.0, 100.0) as u8))?;
                }
            }
            Action::VolumeChanged(vol) => {
                self.play_controls.update(&Action::VolumeChanged(vol))?;
            }
            Action::FilterList(query) => {
                self.discovery_list.set_filter(Some(query.clone()));
            }
            Action::ClearFilter => {
                self.discovery_list.set_filter(None);
            }
            Action::Back => {
                // On Search tab viewing genre results, go back to genre list
                if self.nts_tab.active_sub == NtsSubTab::Search && self.viewing_genre_results {
                    self.nts_tab.loaded.remove("Search");
                    self.action_tx.send(Action::LoadGenres)?;
                } else {
                    // Clear filter when going back
                    self.discovery_list.set_filter(None);
                }
                // Also forward to components (search bar, etc.)
                self.search_bar.update(&Action::Back)?;
            }
            Action::ShowHelp => self.show_help = true,
            Action::HideHelp => self.show_help = false,
            ref action => {
                // Forward to components
                let results = self.nts_tab.update(action)?;
                for a in results { self.action_tx.send(a)?; }
                let results = self.discovery_list.update(action)?;
                for a in results { self.action_tx.send(a)?; }
                self.search_bar.update(action)?;
                self.now_playing.update(action)?;
                self.play_controls.update(action)?;
            }
        }
        Ok(())
    }

    fn sync_queue_to_now_playing(&mut self) {
        let items: Vec<(String, String)> = self
            .queue
            .items()
            .iter()
            .map(|qi| (qi.item.display_title(), qi.item.subtitle().to_string()))
            .collect();
        let current = self.queue.current_index();
        self.now_playing.set_queue(items, current);
    }

    /// Drain all pending actions from the channel and process them. Used in tests.
    #[allow(dead_code)]
    pub async fn flush_actions(&mut self) {
        while let Ok(action) = self.action_rx.try_recv() {
            let _ = self.handle_action(action).await;
        }
    }
}
