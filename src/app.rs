use std::collections::HashSet;
use crossterm::event::{KeyCode, KeyModifiers};
use tokio::sync::mpsc;

use crate::action::Action;
use crate::api::models::DiscoveryItem;
use crate::api::nts::{NtsClient, TOP_GENRES};
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
use crate::ui;

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
    /// True when viewing genre search results (not the genre list itself).
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

        for component in [
            &mut nts_tab as &mut dyn Component,
            &mut discovery_list,
            &mut search_bar,
            &mut now_playing,
            &mut play_controls,
            &mut direct_play_modal,
        ] {
            component.register_action_handler(action_tx.clone());
        }

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

        self.action_tx.send(Action::LoadNtsLive)?;

        while self.running {
            let state = ui::DrawState {
                nts_tab: &self.nts_tab,
                discovery_list: &self.discovery_list,
                search_bar: &self.search_bar,
                now_playing: &self.now_playing,
                play_controls: &self.play_controls,
                direct_play_modal: &self.direct_play_modal,
                error_message: &self.error_message,
                show_help: self.show_help,
            };
            tui.draw(|frame| ui::draw(frame, &state))?;

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

        if self.show_help {
            self.action_tx.send(Action::HideHelp)?;
            return Ok(());
        }

        if self.direct_play_modal.is_visible() {
            self.direct_play_modal.handle_key_event(key)?;
            return Ok(());
        }

        let in_search = self.search_bar.is_focused();

        match (key.code, key.modifiers) {
            (Char('q'), KeyModifiers::NONE) if !in_search => {
                self.action_tx.send(Action::Quit)?;
            }
            (Char('?'), KeyModifiers::NONE) if !in_search => {
                self.action_tx.send(if self.show_help { Action::HideHelp } else { Action::ShowHelp })?;
            }
            (Char('o'), KeyModifiers::NONE) if !in_search => {
                self.action_tx.send(Action::OpenDirectPlay)?;
            }
            (Tab, _) => {
                self.action_tx.send(Action::SwitchSubTab((self.nts_tab.active_index() + 1) % 3))?;
            }
            (BackTab, _) => {
                self.action_tx.send(Action::SwitchSubTab((self.nts_tab.active_index() + 2) % 3))?;
            }
            (Char(' '), KeyModifiers::NONE) if !in_search => {
                self.action_tx.send(Action::TogglePlayPause)?;
            }
            (Char('n'), KeyModifiers::NONE) if !in_search => {
                self.action_tx.send(Action::NextTrack)?;
            }
            (Char('p'), KeyModifiers::NONE) if !in_search => {
                self.action_tx.send(Action::PrevTrack)?;
            }
            (Char('s'), KeyModifiers::NONE) if !in_search => {
                self.action_tx.send(Action::Stop)?;
            }
            (Char('/'), KeyModifiers::NONE) if !in_search => {
                self.action_tx.send(Action::FocusSearch)?;
            }
            (Char('f'), KeyModifiers::NONE) if !in_search => {
                self.action_tx.send(Action::ToggleFavorite)?;
            }
            (Char('c'), KeyModifiers::NONE) if !in_search => {
                self.action_tx.send(Action::ClearQueue)?;
            }
            (Char('a'), KeyModifiers::NONE) if !in_search => {
                if let Some(item) = self.discovery_list.selected_item() {
                    self.action_tx.send(Action::AddToQueue(item.clone()))?;
                }
            }
            (Char('A'), _) if !in_search => {
                if let Some(item) = self.discovery_list.selected_item() {
                    self.action_tx.send(Action::AddToQueueNext(item.clone()))?;
                }
            }
            (Char(']'), KeyModifiers::NONE) if !in_search => {
                self.action_tx.send(Action::VolumeUp)?;
            }
            (Char('['), KeyModifiers::NONE) if !in_search => {
                self.action_tx.send(Action::VolumeDown)?;
            }
            (Char('r'), KeyModifiers::NONE) if !in_search && self.error_message.is_some() => {
                self.action_tx.send(Action::LoadNtsLive)?;
                self.error_message = None;
            }
            (Char(c), KeyModifiers::NONE) if !in_search && c.is_ascii_digit() => {
                let idx = c.to_digit(10).unwrap_or(0) as usize;
                if idx >= 1 && idx <= 3 {
                    self.action_tx.send(Action::SwitchSubTab(idx - 1))?;
                }
            }
            (Esc, _) => self.action_tx.send(Action::Back)?,
            _ => {
                if in_search {
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
            // Lifecycle
            Action::Quit => {
                let _ = self.player.stop().await;
                self.running = false;
            }

            // Playback
            Action::PlayItem(ref item) => self.play_item(item.clone()).await?,
            Action::TogglePlayPause => { let _ = self.player.toggle_pause().await; }
            Action::Stop => { let _ = self.player.stop().await; }
            Action::NextTrack => { self.play_queue_track(Queue::next).await?; }
            Action::PrevTrack => { self.play_queue_track(Queue::prev).await?; }

            // Queue
            Action::AddToQueue(ref item) => self.enqueue(item.clone(), false),
            Action::AddToQueueNext(ref item) => self.enqueue(item.clone(), true),
            Action::ClearQueue => {
                self.queue.clear();
                self.play_controls.queue_pos = None;
                self.play_controls.queue_len = 0;
                self.sync_queue_to_now_playing();
            }

            // Favorites & history
            Action::ToggleFavorite => self.toggle_favorite()?,
            Action::AddToHistory(ref item) => { let _ = self.db.add_to_history(item); }

            // Data loading
            Action::LoadNtsLive => self.spawn_fetch_live(),
            Action::NtsLiveLoaded(items) => self.discovery_list.set_items(items),
            Action::LoadNtsPicks => self.spawn_fetch_picks(),
            Action::NtsPicksLoaded(items) => self.discovery_list.set_items(items),
            Action::LoadGenres => self.load_genres()?,
            Action::GenresLoaded(items) => {
                self.discovery_list.set_items(items);
                self.viewing_genre_results = false;
            }

            // Genre search
            Action::SearchByGenre { genre_id } => self.search_by_genre(genre_id)?,
            Action::SearchResultsPartial { search_id, items, done } => {
                if search_id == self.search_id {
                    if !items.is_empty() { self.discovery_list.append_items(items); }
                    if done { self.discovery_list.loading = false; }
                }
            }

            // Tab switching
            Action::SwitchSubTab(idx) => self.switch_sub_tab(idx)?,

            // Search / filter
            Action::SearchSubmit => {
                let query = self.search_bar.input().to_string();
                self.action_tx.send(if query.is_empty() {
                    Action::ClearFilter
                } else {
                    Action::FilterList(query)
                })?;
            }
            Action::FilterList(query) => self.discovery_list.set_filter(Some(query)),
            Action::ClearFilter => self.discovery_list.set_filter(None),

            // Direct play modal
            Action::OpenDirectPlay => self.direct_play_modal.show(),
            Action::CloseDirectPlay => self.direct_play_modal.hide(),

            // Playback state updates (forwarded to display components)
            Action::PlaybackStarted { .. } | Action::PlaybackLoading | Action::PlaybackPosition(_) => {
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
                self.play_queue_track(Queue::next).await?;
            }

            // Errors & help
            Action::ShowError(msg) => {
                self.error_message = Some(msg);
                let tx = self.action_tx.clone();
                tokio::spawn(async move {
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    tx.send(Action::ClearError).ok();
                });
            }
            Action::ClearError => self.error_message = None,
            Action::ShowHelp => self.show_help = true,
            Action::HideHelp => self.show_help = false,

            // Volume
            Action::VolumeUp => self.adjust_volume(5.0).await?,
            Action::VolumeDown => self.adjust_volume(-5.0).await?,
            Action::VolumeChanged(vol) => { self.play_controls.update(&Action::VolumeChanged(vol))?; }

            // Navigation
            Action::Back => {
                if self.nts_tab.active_sub == NtsSubTab::Search && self.viewing_genre_results {
                    self.nts_tab.loaded.remove("Search");
                    self.action_tx.send(Action::LoadGenres)?;
                } else {
                    self.discovery_list.set_filter(None);
                }
                self.search_bar.update(&Action::Back)?;
            }

            // Forward anything unhandled to components
            ref action => {
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

    // ── Playback helpers ──

    /// Start playing an item: enqueue it, and if nothing is playing, start playback.
    async fn play_item(&mut self, item: DiscoveryItem) -> anyhow::Result<()> {
        let Some(url) = item.playback_url() else { return Ok(()) };
        let nothing_playing = self.now_playing.current_item.is_none();
        let new_index = self.queue.len();

        self.queue.add(QueueItem { item: item.clone(), url: url.clone(), stream_title: None });
        self.sync_play_controls();
        self.sync_queue_to_now_playing();

        if nothing_playing {
            self.queue.play_at(new_index);
            self.sync_play_controls();
            self.now_playing.update(&Action::PlayItem(item.clone()))?;
            self.player.play(&url).await?;
            self.action_tx.send(Action::PlaybackStarted { title: item.display_title(), url })?;
            self.action_tx.send(Action::AddToHistory(item))?;
            self.sync_queue_to_now_playing();
        }
        Ok(())
    }

    /// Advance to the next or previous track in the queue and play it.
    async fn play_queue_track(&mut self, advance: fn(&mut Queue) -> Option<&QueueItem>) -> anyhow::Result<()> {
        let Some(track) = advance(&mut self.queue) else { return Ok(()) };
        let url = track.url.clone();
        let title = track.item.display_title();
        let item = track.item.clone();

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
        Ok(())
    }

    async fn adjust_volume(&mut self, delta: f64) -> anyhow::Result<()> {
        let _ = self.player.set_volume(delta).await;
        if let Ok(vol) = self.player.get_volume().await {
            self.action_tx.send(Action::VolumeChanged(vol.round().clamp(0.0, 100.0) as u8))?;
        }
        Ok(())
    }

    // ── Queue helpers ──

    fn enqueue(&mut self, item: DiscoveryItem, insert_next: bool) {
        let url = item.playback_url().unwrap_or_default();
        let qi = QueueItem { item, url, stream_title: None };
        if insert_next { self.queue.add_next(qi); } else { self.queue.add(qi); }
        self.sync_play_controls();
        self.sync_queue_to_now_playing();
    }

    fn sync_play_controls(&mut self) {
        self.play_controls.queue_pos = self.queue.current_index();
        self.play_controls.queue_len = self.queue.len();
    }

    fn sync_queue_to_now_playing(&mut self) {
        let items: Vec<(String, String)> = self
            .queue.items().iter()
            .map(|qi| (qi.item.display_title(), qi.item.subtitle().to_string()))
            .collect();
        self.now_playing.set_queue(items, self.queue.current_index());
    }

    // ── Favorite helpers ──

    fn toggle_favorite(&mut self) -> anyhow::Result<()> {
        let Some(item) = self.discovery_list.selected_item().cloned() else { return Ok(()) };
        let key = item.favorite_key();
        if self.favorites.contains(&key) {
            self.db.remove_favorite(&key)?;
            self.favorites.remove(&key);
        } else {
            self.db.add_favorite(&item)?;
            self.favorites.insert(key);
        }
        self.discovery_list.set_favorites(self.favorites.clone());
        Ok(())
    }

    // ── Data loading helpers ──

    fn spawn_fetch_live(&self) {
        let tx = self.action_tx.clone();
        let client = self.nts_client.clone();
        tokio::spawn(async move {
            match client.fetch_live().await {
                Ok(items) => { tx.send(Action::NtsLiveLoaded(items)).ok(); }
                Err(e) => { tx.send(Action::ShowError(e.to_string())).ok(); }
            }
        });
    }

    fn spawn_fetch_picks(&self) {
        let tx = self.action_tx.clone();
        let client = self.nts_client.clone();
        tokio::spawn(async move {
            match client.fetch_picks().await {
                Ok(items) => { tx.send(Action::NtsPicksLoaded(items)).ok(); }
                Err(e) => { tx.send(Action::ShowError(e.to_string())).ok(); }
            }
        });
    }

    fn load_genres(&mut self) -> anyhow::Result<()> {
        let fav_count = self.db.list_favorites(None, 1000, 0)?.len();
        let hist_count = self.db.list_history(1000, 0)?.len();

        let mut items: Vec<DiscoveryItem> = Vec::with_capacity(TOP_GENRES.len() + 2);
        items.push(DiscoveryItem::NtsGenre {
            name: format!("My Favorites ({})", fav_count),
            genre_id: "_favorites".to_string(),
        });
        items.push(DiscoveryItem::NtsGenre {
            name: format!("History ({})", hist_count),
            genre_id: "_history".to_string(),
        });
        for &(id, name) in TOP_GENRES {
            items.push(DiscoveryItem::NtsGenre {
                name: name.to_string(),
                genre_id: id.to_string(),
            });
        }

        self.action_tx.send(Action::GenresLoaded(items))?;
        self.viewing_genre_results = false;
        Ok(())
    }

    fn search_by_genre(&mut self, genre_id: String) -> anyhow::Result<()> {
        self.search_id += 1;
        let sid = self.search_id;
        self.discovery_list.set_items(vec![]);
        self.discovery_list.loading = true;
        self.viewing_genre_results = true;

        // Special local-DB genres
        if genre_id == "_favorites" || genre_id == "_history" {
            let items: Vec<DiscoveryItem> = if genre_id == "_favorites" {
                self.db.list_favorites(None, 1000, 0)?.iter().map(|r| r.to_discovery_item()).collect()
            } else {
                self.db.list_history(1000, 0)?.iter().map(|r| r.to_discovery_item()).collect()
            };
            self.action_tx.send(Action::SearchResultsPartial { search_id: sid, items, done: true })?;
            return Ok(());
        }

        // Remote paginated search
        let tx = self.action_tx.clone();
        let client = self.nts_client.clone();
        tokio::spawn(async move {
            let page_size = 12u64;
            let max_offset = 240u64;
            let mut buf = Vec::new();
            let mut offset = 0u64;

            while offset <= max_offset {
                match client.search_episodes(&genre_id, offset, page_size).await {
                    Ok(items) => {
                        let got = items.len();
                        buf.extend(items);
                        if (got as u64) < page_size { break; }
                    }
                    Err(_) => break,
                }
                offset += page_size;

                if buf.len() >= 48 || offset > max_offset {
                    let batch = std::mem::take(&mut buf);
                    let done = offset > max_offset;
                    tx.send(Action::SearchResultsPartial { search_id: sid, items: batch, done }).ok();
                }
            }

            // Flush remaining
            tx.send(Action::SearchResultsPartial {
                search_id: sid,
                items: buf,
                done: true,
            }).ok();
        });

        Ok(())
    }

    fn switch_sub_tab(&mut self, idx: usize) -> anyhow::Result<()> {
        self.discovery_list.set_items(vec![]);
        self.discovery_list.loading = true;
        self.viewing_genre_results = false;
        self.discovery_list.set_filter(None);
        self.search_bar.update(&Action::Back)?;

        let actions = self.nts_tab.switch_sub_tab(idx);
        if actions.is_empty() {
            // Already visited — force reload
            match self.nts_tab.active_sub {
                NtsSubTab::Live => self.action_tx.send(Action::LoadNtsLive)?,
                NtsSubTab::Picks => self.action_tx.send(Action::LoadNtsPicks)?,
                NtsSubTab::Search => self.action_tx.send(Action::LoadGenres)?,
            }
        } else {
            for a in actions { self.action_tx.send(a)?; }
        }
        Ok(())
    }

    /// Drain all pending actions. Used in tests.
    #[allow(dead_code)]
    pub async fn flush_actions(&mut self) {
        while let Ok(action) = self.action_rx.try_recv() {
            let _ = self.handle_action(action).await;
        }
    }
}
