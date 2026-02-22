// Playback and queue management: play/pause, track navigation, enqueue, volume.

use crate::action::Action;
use crate::api::models::DiscoveryItem;
use crate::app::App;
use crate::components::Component;
use crate::player::queue::{Queue, QueueItem};

impl App {
    /// Start playing an item: enqueue it, and if nothing is playing, start playback.
    pub(super) async fn play_item(&mut self, item: DiscoveryItem) -> anyhow::Result<()> {
        let Some(url) = item.playback_url() else {
            return Ok(());
        };
        let nothing_playing = !self.now_playing.is_playing();
        let new_index = self.queue.len();

        self.queue.add(QueueItem {
            item: item.clone(),
            url: url.clone(),
            stream_metadata: None,
        });
        self.sync_play_controls();
        self.sync_queue_to_now_playing();

        if nothing_playing {
            self.queue.play_at(new_index);
            self.sync_play_controls();
            self.now_playing.update(&Action::PlayItem(item.clone()))?;
            self.player.play(&url).await?;
            self.action_tx.send(Action::PlaybackStarted {
                title: item.display_title(),
            })?;
            self.sync_queue_to_now_playing();
        }
        self.persist_queue();
        Ok(())
    }

    /// Advance to the next or previous track in the queue and play it.
    pub(super) async fn play_queue_track(
        &mut self,
        advance: fn(&mut Queue) -> Option<&QueueItem>,
    ) -> anyhow::Result<()> {
        if advance(&mut self.queue).is_some() {
            self.start_current_track().await?;
            self.persist_queue();
        }
        Ok(())
    }

    /// Remove the current track from the queue. If there's a next track, play it;
    /// otherwise stop playback.
    pub(super) async fn remove_current_from_queue(&mut self) -> anyhow::Result<()> {
        let Some(idx) = self.queue.current_index() else {
            return Ok(());
        };
        self.queue.remove(idx);
        if self.queue.is_empty() {
            let _ = self.player.stop().await;
            self.now_playing.update(&Action::PlaybackFinished)?;
            self.play_controls.update(&Action::PlaybackFinished)?;
        } else {
            self.start_current_track().await?;
        }
        self.sync_play_controls();
        self.sync_queue_to_now_playing();
        self.persist_queue();
        Ok(())
    }

    /// Set up UI state for the current track and start mpv playback.
    pub(super) async fn start_current_track(&mut self) -> anyhow::Result<()> {
        let Some(track) = self.queue.current() else {
            return Ok(());
        };
        let url = track.url.clone();
        let title = track.item.display_title();
        let item = track.item.clone();

        self.sync_play_controls();
        self.now_playing.set_buffering(item);
        self.play_controls.set_buffering(true);
        self.sync_queue_to_now_playing();

        if let Err(e) = self.player.play(&url).await {
            self.action_tx.send(Action::ShowError(e.to_string()))?;
        } else {
            self.action_tx.send(Action::PlaybackStarted { title })?;
        }
        Ok(())
    }

    pub(super) async fn adjust_volume(&mut self, delta: f64) -> anyhow::Result<()> {
        let _ = self.player.set_volume(delta).await;
        if let Ok(vol) = self.player.get_volume().await {
            self.action_tx
                .send(Action::VolumeChanged(vol.round().clamp(0.0, 100.0) as u8))?;
        }
        Ok(())
    }

    pub(super) fn enqueue(&mut self, item: DiscoveryItem, insert_next: bool) {
        let url = item.playback_url().unwrap_or_default();
        let qi = QueueItem {
            item,
            url,
            stream_metadata: None,
        };
        if insert_next {
            self.queue.add_next(qi);
        } else {
            self.queue.add(qi);
        }
        self.sync_play_controls();
        self.sync_queue_to_now_playing();
        self.persist_queue();
    }

    pub(super) fn sync_play_controls(&mut self) {
        self.play_controls
            .set_queue_info(self.queue.current_index(), self.queue.len());
    }

    pub(super) fn sync_queue_to_now_playing(&mut self) {
        let items: Vec<(String, String)> = self
            .queue
            .items()
            .iter()
            .map(|qi| {
                let m = qi.stream_metadata.as_ref();
                qi.item.display_pair(
                    m.and_then(|m| m.station_name.as_deref()),
                    m.and_then(|m| m.display_title()).as_deref(),
                    m.and_then(|m| m.display_subtitle()).as_deref(),
                )
            })
            .collect();
        self.now_playing
            .set_queue(items, self.queue.current_index());
    }
}
