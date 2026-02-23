// Action dispatch: routes each Action variant to the right handler.

use crate::action::Action;
use crate::app::App;
use crate::components::nts::NtsSubTab;
use crate::components::Component;
use crate::player::queue::Queue;
use crate::theme::Theme;

impl App {
    pub async fn handle_action(&mut self, action: Action) -> anyhow::Result<()> {
        match action {
            // Lifecycle
            Action::Quit => {
                let _ = self.player.stop().await;
                self.running = false;
            }

            // Playback
            Action::PlayItem(item) => self.play_item(item).await?,
            Action::TogglePlayPause => {
                if !self.now_playing.is_playing() {
                    self.start_current_track().await?;
                } else {
                    let _ = self.player.toggle_pause().await;
                    self.now_playing.update(&Action::TogglePlayPause)?;
                    self.play_controls.update(&Action::TogglePlayPause)?;
                }
            }
            Action::Stop => {
                let _ = self.player.stop().await;
                self.seek_modal.hide();
                self.seek.reset();
            }
            Action::NextTrack => {
                self.play_queue_track(Queue::advance).await?;
            }
            Action::PrevTrack => {
                self.play_queue_track(Queue::prev).await?;
            }

            // Queue
            Action::AddToQueue(item) => self.enqueue(item, false),
            Action::AddToQueueNext(item) => self.enqueue(item, true),
            Action::RemoveFromQueue => self.remove_current_from_queue().await?,
            Action::ClearQueue => {
                self.queue.clear();
                self.play_controls.set_queue_info(None, 0);
                self.sync_queue_to_now_playing();
                self.persist_queue();
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
                self.viewing_query_results = false;
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
                if !query.is_empty() {
                    if self.nts_tab.active_sub() != NtsSubTab::Search {
                        self.nts_tab.switch_sub_tab(2);
                    }
                    self.action_tx.send(Action::SearchByQuery { query })?;
                }
            }
            Action::SearchByQuery { query } => self.search_by_query(query)?,

            // Direct play modal
            Action::OpenDirectPlay => self.direct_play_modal.show(),
            Action::CloseDirectPlay => self.direct_play_modal.hide(),

            // Seek
            Action::PlaybackDuration(dur) => {
                self.seek.duration_secs = dur;
                self.seek.is_seekable = dur.is_some();
                self.now_playing.update(&action)?;
                self.play_controls.update(&action)?;
                if self.seek_modal.is_visible() {
                    if let Some(d) = dur {
                        self.seek_modal.update_duration(d);
                    }
                }
                if self.seek.pending_intro_skip && dur.is_some() {
                    self.seek.pending_intro_skip = false;
                    self.action_tx.send(Action::SeekRelative(3.0))?;
                }
            }
            Action::SeekRelative(secs) => {
                let _ = self.player.seek_relative(secs).await;
            }
            Action::OpenSeekModal => {
                if self.seek.is_seekable {
                    if let Some(dur) = self.seek.duration_secs {
                        self.seek_modal.show(self.now_playing.position_secs(), dur);
                    }
                }
            }
            Action::CloseSeekModal => {
                self.seek_modal.hide();
            }

            // Visualizer
            Action::CycleVisualizer => {
                let new_kind = self.now_playing.cycle_visualizer();
                self.config.general.visualizer = new_kind;
                self.save_config_async();
            }

            Action::ToggleSkipIntro => {
                self.config.general.skip_nts_intro = !self.config.general.skip_nts_intro;
                self.play_controls.update(&action)?;
                self.save_config_async();
            }

            // Onboarding
            Action::OnboardingComplete {
                theme,
                completed_screens,
            } => {
                self.onboarding.set_active(false);
                self.config.general.theme = theme.clone();
                for id in completed_screens {
                    if !self.config.general.completed_onboarding.contains(&id) {
                        self.config.general.completed_onboarding.push(id);
                    }
                }
                self.theme = Theme::from_name(&theme);
                self.save_config_async();
                self.action_tx.send(Action::LoadNtsLive)?;
            }
            Action::ShowOnboarding => {
                self.onboarding.activate_all();
            }

            // Playback state updates (forwarded to display components)
            Action::AudioLevels { .. } => {
                self.now_playing.update(&action)?;
            }
            Action::PlaybackStarted { .. } => {
                self.now_playing.update(&action)?;
                self.play_controls.update(&action)?;
                if self.config.general.skip_nts_intro {
                    if let Some(track) = self.queue.current() {
                        if matches!(
                            track.item,
                            crate::api::models::DiscoveryItem::NtsEpisode { .. }
                        ) {
                            self.seek.pending_intro_skip = true;
                        }
                    }
                }
            }
            Action::PlaybackPosition(pos) => {
                self.now_playing.update(&action)?;
                self.play_controls.update(&action)?;
                if self.seek_modal.is_visible() {
                    self.seek_modal.update_position(pos);
                }
            }
            Action::PlaybackLoading => {
                self.play_controls.update(&action)?;
            }
            Action::StreamMetadataChanged(metadata) => {
                self.queue.set_current_stream_metadata(metadata.clone());
                let action = Action::StreamMetadataChanged(metadata);
                self.now_playing.update(&action)?;
                self.play_controls.update(&action)?;
                self.sync_queue_to_now_playing();
            }
            Action::PlaybackFinished => {
                self.now_playing.update(&action)?;
                self.play_controls.update(&action)?;
                self.seek_modal.hide();
                self.seek.reset();
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
                if self.nts_tab.active_sub() == NtsSubTab::Search
                    && (self.viewing_genre_results || self.viewing_query_results)
                {
                    self.viewing_query_results = false;
                    self.nts_tab.mark_unloaded(NtsSubTab::Search);
                    self.action_tx.send(Action::LoadGenres)?;
                } else {
                    self.discovery_list.set_filter(None);
                }
                self.search_bar.update(&Action::Back)?;
            }

            // Forward anything unhandled to components
            action => {
                let results = self.nts_tab.update(&action)?;
                for a in results {
                    self.action_tx.send(a)?;
                }
                let results = self.discovery_list.update(&action)?;
                for a in results {
                    self.action_tx.send(a)?;
                }
                self.search_bar.update(&action)?;
                self.now_playing.update(&action)?;
                self.play_controls.update(&action)?;
            }
        }
        Ok(())
    }

    fn switch_sub_tab(&mut self, idx: usize) -> anyhow::Result<()> {
        self.discovery_list.set_items(vec![]);
        self.discovery_list.set_loading(true);
        self.viewing_genre_results = false;
        self.viewing_query_results = false;
        self.discovery_list.set_filter(None);
        self.search_bar.update(&Action::Back)?;

        let actions = self.nts_tab.switch_sub_tab(idx);
        if actions.is_empty() {
            match self.nts_tab.active_sub() {
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

    /// Write the current config to disk without blocking the event loop.
    pub(super) fn save_config_async(&self) {
        let config = self.config.clone();
        tokio::spawn(async move { config.save().ok() });
    }
}
