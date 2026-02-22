// Central coordinator: owns all components, the player, and the database.
// Runs the event loop (key → Action → handle_action → component updates → draw).

mod fetch;
mod input;
mod playback;

use std::collections::HashSet;
use tokio::sync::mpsc;

use crate::action::Action;
use crate::api::nts::NtsClient;
use crate::components::direct_play_modal::DirectPlayModal;
use crate::components::discovery_list::DiscoveryList;
use crate::components::now_playing::NowPlaying;
use crate::components::nts::{NtsSubTab, NtsTab};
use crate::components::play_controls::PlayControls;
use crate::components::search_bar::SearchBar;
use crate::components::Component;
use crate::config::Config;
use crate::db::Database;
use crate::player::queue::Queue;
use crate::player::MpvPlayer;
use crate::tui::{Tui, TuiEvent};
use crate::ui;

/// Top-level coordinator: owns every component, the mpv player, and the
/// database. Runs the main event loop (key → action → component update → draw).
pub struct App {
    running: bool,
    action_tx: mpsc::UnboundedSender<Action>,
    action_rx: mpsc::UnboundedReceiver<Action>,

    // Components
    pub nts_tab: NtsTab,
    pub discovery_list: DiscoveryList,
    pub(crate) search_bar: SearchBar,
    pub(crate) now_playing: NowPlaying,
    pub(crate) play_controls: PlayControls,
    pub(crate) direct_play_modal: DirectPlayModal,

    // State
    pub(crate) nts_client: NtsClient,
    player: MpvPlayer,
    pub(crate) db: Database,
    pub(crate) config: Config,
    pub(crate) favorites: HashSet<String>,
    pub queue: Queue,
    pub show_help: bool,
    pub error_message: Option<String>,
    pub(crate) search_id: u64,
    /// True when viewing genre search results (not the genre list itself).
    pub(crate) viewing_genre_results: bool,
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
                        TuiEvent::Resize => {} // ratatui redraws at correct size automatically
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

    pub async fn handle_action(&mut self, action: Action) -> anyhow::Result<()> {
        match action {
            // Lifecycle
            Action::Quit => {
                let _ = self.player.stop().await;
                self.running = false;
            }

            // Playback
            Action::PlayItem(ref item) => self.play_item(item.clone()).await?,
            Action::TogglePlayPause => {
                let _ = self.player.toggle_pause().await;
            }
            Action::Stop => {
                let _ = self.player.stop().await;
            }
            Action::NextTrack => {
                self.play_queue_track(Queue::advance).await?;
            }
            Action::PrevTrack => {
                self.play_queue_track(Queue::prev).await?;
            }

            // Queue
            Action::AddToQueue(ref item) => self.enqueue(item.clone(), false),
            Action::AddToQueueNext(ref item) => self.enqueue(item.clone(), true),
            Action::ClearQueue => {
                self.queue.clear();
                self.play_controls.set_queue_info(None, 0);
                self.sync_queue_to_now_playing();
            }

            // Favorites & history
            Action::ToggleFavorite => self.toggle_favorite()?,
            Action::AddToHistory(ref item) => {
                let _ = self.db.add_to_history(item);
            }

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
            Action::SearchResultsPartial {
                search_id,
                items,
                done,
            } => {
                if search_id == self.search_id {
                    if !items.is_empty() {
                        self.discovery_list.append_items(items);
                    }
                    if done {
                        self.discovery_list.set_loading(false);
                    }
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
            Action::AudioLevels { .. } => {
                self.now_playing.update(&action)?;
            }
            Action::PlaybackStarted { .. } | Action::PlaybackPosition(_) => {
                self.now_playing.update(&action)?;
                self.play_controls.update(&action)?;
            }
            Action::PlaybackLoading => {
                self.play_controls.update(&action)?;
            }
            Action::StreamMetadataChanged(ref metadata) => {
                self.queue.set_current_stream_metadata(metadata.clone());
                self.now_playing.update(&action)?;
                self.play_controls.update(&action)?;
                self.sync_queue_to_now_playing();
            }
            Action::PlaybackFinished => {
                self.now_playing.update(&action)?;
                self.play_controls.update(&action)?;
                self.play_queue_track(Queue::advance).await?;
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
            Action::VolumeChanged(vol) => {
                self.play_controls.update(&Action::VolumeChanged(vol))?;
            }

            // Navigation
            Action::Back => {
                if self.nts_tab.active_sub == NtsSubTab::Search && self.viewing_genre_results {
                    self.nts_tab.mark_unloaded(NtsSubTab::Search);
                    self.action_tx.send(Action::LoadGenres)?;
                } else {
                    self.discovery_list.set_filter(None);
                }
                self.search_bar.update(&Action::Back)?;
            }

            // Forward anything unhandled to components
            ref action => {
                let results = self.nts_tab.update(action)?;
                for a in results {
                    self.action_tx.send(a)?;
                }
                let results = self.discovery_list.update(action)?;
                for a in results {
                    self.action_tx.send(a)?;
                }
                self.search_bar.update(action)?;
                self.now_playing.update(action)?;
                self.play_controls.update(action)?;
            }
        }
        Ok(())
    }

    fn toggle_favorite(&mut self) -> anyhow::Result<()> {
        let Some(item) = self.discovery_list.selected_item().cloned() else {
            return Ok(());
        };
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

    fn switch_sub_tab(&mut self, idx: usize) -> anyhow::Result<()> {
        self.discovery_list.set_items(vec![]);
        self.discovery_list.set_loading(true);
        self.viewing_genre_results = false;
        self.discovery_list.set_filter(None);
        self.search_bar.update(&Action::Back)?;

        let actions = self.nts_tab.switch_sub_tab(idx);
        if actions.is_empty() {
            match self.nts_tab.active_sub {
                NtsSubTab::Live => self.action_tx.send(Action::LoadNtsLive)?,
                NtsSubTab::Picks => self.action_tx.send(Action::LoadNtsPicks)?,
                NtsSubTab::Search => self.action_tx.send(Action::LoadGenres)?,
            }
        } else {
            for a in actions {
                self.action_tx.send(a)?;
            }
        }
        Ok(())
    }

    #[allow(dead_code)] // used by integration tests
    pub async fn flush_actions(&mut self) {
        while let Ok(action) = self.action_rx.try_recv() {
            let _ = self.handle_action(action).await;
        }
    }
}
