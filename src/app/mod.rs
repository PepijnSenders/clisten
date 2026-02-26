// Central coordinator: owns all components, the player, and the database.
// Runs the event loop (key → Action → handle_action → component updates → draw).

mod actions;
mod fetch;
mod input;
mod playback;

use std::time::Instant;

use tokio::sync::mpsc;

use crate::action::Action;
use crate::api::nts::NtsClient;
use crate::components::direct_play_modal::DirectPlayModal;
use crate::components::discovery_list::DiscoveryList;
use crate::components::now_playing::NowPlaying;
use crate::components::nts::NtsTab;
use crate::components::onboarding::Onboarding;
use crate::components::play_controls::PlayControls;
use crate::components::search_bar::SearchBar;
use crate::components::seek_modal::SeekModal;
use crate::components::Component;
use crate::config::Config;
use crate::db::Database;
use crate::player::queue::Queue;
use crate::player::MpvPlayer;
use crate::theme::Theme;
use crate::tui::{Tui, TuiEvent};
use crate::ui;

/// Tracks accelerating seek behavior and pending intro skip.
#[derive(Default)]
pub(crate) struct SeekState {
    pub(crate) is_seekable: bool,
    pub(crate) duration_secs: Option<f64>,
    pub(crate) last_seek_time: Option<Instant>,
    pub(crate) seek_streak: u32,
    pub(crate) pending_intro_skip: bool,
}

impl SeekState {
    pub(crate) fn reset(&mut self) {
        *self = Self::default();
    }

    /// Calculate the seek step size, accelerating on rapid presses.
    pub(crate) fn step(&mut self) -> f64 {
        let now = Instant::now();
        if let Some(last) = self.last_seek_time {
            if now.duration_since(last).as_millis() < 400 {
                self.seek_streak += 1;
            } else {
                self.seek_streak = 0;
            }
        } else {
            self.seek_streak = 0;
        }
        self.last_seek_time = Some(now);
        match self.seek_streak {
            0..=2 => 5.0,
            3..=7 => 15.0,
            _ => 30.0,
        }
    }
}

/// Top-level coordinator: owns every component, the mpv player, and the
/// database. Runs the main event loop (key → action → component update → draw).
pub struct App {
    pub(crate) running: bool,
    pub(crate) action_tx: mpsc::UnboundedSender<Action>,
    action_rx: mpsc::UnboundedReceiver<Action>,

    // Components
    pub nts_tab: NtsTab,
    pub discovery_list: DiscoveryList,
    pub(crate) search_bar: SearchBar,
    pub(crate) now_playing: NowPlaying,
    pub(crate) play_controls: PlayControls,
    pub(crate) direct_play_modal: DirectPlayModal,
    pub(crate) seek_modal: SeekModal,
    pub onboarding: Onboarding,

    // State
    pub(crate) nts_client: NtsClient,
    pub(crate) player: MpvPlayer,
    pub(crate) db: Database,
    pub(crate) config: Config,
    pub queue: Queue,
    pub show_help: bool,
    pub error_message: Option<String>,
    pub(crate) search_id: u64,
    /// True when viewing genre search results (not the genre list itself).
    pub(crate) viewing_genre_results: bool,
    /// True when viewing text query search results.
    pub(crate) viewing_query_results: bool,
    pub(crate) theme: Theme,
    pub(crate) seek: SeekState,
    /// Tick counter for periodic live metadata refresh.
    pub(crate) live_refresh_ticks: u32,
}

impl App {
    pub fn new(config: Config) -> anyhow::Result<Self> {
        let db = Database::open()?;
        Self::with_db(config, db)
    }

    /// Create an App with a custom database (used by integration tests to avoid
    /// polluting the production database).
    pub fn with_db(config: Config, db: Database) -> anyhow::Result<Self> {
        let (action_tx, action_rx) = mpsc::unbounded_channel();
        let queue = Self::restore_queue(&db);
        let theme = Theme::from_name(&config.general.theme);

        let mut nts_tab = NtsTab::new();
        let mut discovery_list = DiscoveryList::new();
        let mut search_bar = SearchBar::new();
        let mut now_playing = NowPlaying::new(config.general.visualizer);
        let mut play_controls = PlayControls::new();
        play_controls.set_skip_nts_intro(config.general.skip_nts_intro);
        let mut direct_play_modal = DirectPlayModal::new();
        let mut seek_modal = SeekModal::new();
        let mut onboarding = Onboarding::new();

        for component in [
            &mut nts_tab as &mut dyn Component,
            &mut discovery_list,
            &mut search_bar,
            &mut now_playing,
            &mut play_controls,
            &mut direct_play_modal,
            &mut seek_modal,
            &mut onboarding,
        ] {
            component.register_action_handler(action_tx.clone());
        }

        let mut player = MpvPlayer::new();
        player.set_action_tx(action_tx.clone());

        // Sync restored queue to UI components
        play_controls.set_queue_info(queue.current_index(), queue.len());
        let queue_display: Vec<(String, String)> = queue
            .items()
            .iter()
            .map(|qi| (qi.item.display_title(), qi.item.subtitle()))
            .collect();
        now_playing.set_queue(queue_display, queue.current_index());

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
            seek_modal,
            onboarding,
            nts_client: NtsClient::new(),
            player,
            db,
            config,
            queue,
            show_help: false,
            error_message: None,
            search_id: 0,
            viewing_genre_results: false,
            viewing_query_results: false,
            theme,
            seek: SeekState::default(),
            live_refresh_ticks: 0,
        })
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        let mut tui = Tui::new(self.config.general.frame_rate)?;
        tui.enter()?;

        // Only load NTS data if onboarding is not active
        if !self.onboarding.is_active() {
            self.action_tx.send(Action::LoadNtsLive)?;
        }

        while self.running {
            let state = ui::DrawState {
                nts_tab: &self.nts_tab,
                discovery_list: &self.discovery_list,
                search_bar: &self.search_bar,
                now_playing: &self.now_playing,
                play_controls: &self.play_controls,
                direct_play_modal: &self.direct_play_modal,
                seek_modal: &self.seek_modal,
                onboarding: &self.onboarding,
                error_message: &self.error_message,
                show_help: self.show_help,
                theme: &self.theme,
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

            // Drain any buffered actions so the consumer catches up each frame
            while let Ok(action) = self.action_rx.try_recv() {
                self.handle_action(action).await?;
            }
        }

        tui.exit()?;
        Ok(())
    }

    pub(super) fn persist_queue(&self) {
        let _ = self
            .db
            .save_queue(self.queue.items(), self.queue.current_index());
    }

    fn restore_queue(db: &Database) -> Queue {
        let mut queue = Queue::new();
        if let Ok((items, current_index)) = db.load_queue() {
            for qi in items {
                queue.add(qi);
            }
            if let Some(idx) = current_index {
                queue.play_at(idx);
            }
        }
        queue
    }

    #[allow(dead_code)] // used by integration tests
    pub async fn flush_actions(&mut self) {
        while let Ok(action) = self.action_rx.try_recv() {
            let _ = self.handle_action(action).await;
        }
    }
}
