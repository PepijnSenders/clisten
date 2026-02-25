// Ordered playback queue with a cursor pointing at the current track.

use super::StreamMetadata;
use crate::api::models::DiscoveryItem;

/// A single entry in the playback queue.
#[derive(Debug, Clone)]
pub struct QueueItem {
    pub item: DiscoveryItem,
    pub url: String,
    pub stream_metadata: Option<StreamMetadata>,
}

/// Ordered playback queue with a cursor pointing at the current track.
#[derive(Default)]
pub struct Queue {
    items: Vec<QueueItem>,
    current_index: Option<usize>,
}

impl Queue {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add item to end of queue.
    pub fn add(&mut self, item: QueueItem) {
        self.items.push(item);
        if self.current_index.is_none() {
            self.current_index = Some(0);
        }
    }

    /// Insert item right after current position.
    pub fn add_next(&mut self, item: QueueItem) {
        let pos = self.current_index.map_or(0, |i| i + 1);
        self.items.insert(pos, item);
        if self.current_index.is_none() {
            self.current_index = Some(0);
        }
    }

    pub fn remove(&mut self, index: usize) {
        if index < self.items.len() {
            self.items.remove(index);
            if self.items.is_empty() {
                self.current_index = None;
            } else if let Some(curr) = self.current_index {
                if index <= curr && curr > 0 {
                    self.current_index = Some(curr - 1);
                }
            }
        }
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.current_index = None;
    }

    pub fn current(&self) -> Option<&QueueItem> {
        self.current_index.and_then(|i| self.items.get(i))
    }

    /// Advance to next track. Returns the new current item, or None if at end.
    pub fn advance(&mut self) -> Option<&QueueItem> {
        if let Some(i) = self.current_index {
            if i + 1 < self.items.len() {
                self.current_index = Some(i + 1);
                return self.items.get(i + 1);
            }
        }
        None
    }

    /// Go back to previous track.
    pub fn prev(&mut self) -> Option<&QueueItem> {
        if let Some(i) = self.current_index {
            if i > 0 {
                self.current_index = Some(i - 1);
                return self.items.get(i - 1);
            }
        }
        None
    }

    pub fn items(&self) -> &[QueueItem] {
        &self.items
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn current_index(&self) -> Option<usize> {
        self.current_index
    }

    /// Jump to a specific position in the queue.
    pub fn play_at(&mut self, index: usize) -> Option<&QueueItem> {
        if index < self.items.len() {
            self.current_index = Some(index);
            self.items.get(index)
        } else {
            None
        }
    }

    /// Update live channel items in the queue with fresh show names and genres.
    /// Matches by channel number (the stable identifier). Returns `true` if
    /// anything was actually changed.
    pub fn update_live_channels(&mut self, live: &[DiscoveryItem]) -> bool {
        let mut changed = false;
        for qi in &mut self.items {
            if let DiscoveryItem::NtsLiveChannel {
                channel,
                show_name,
                genres,
            } = &mut qi.item
            {
                for fresh in live {
                    if let DiscoveryItem::NtsLiveChannel {
                        channel: ch,
                        show_name: new_name,
                        genres: new_genres,
                    } = fresh
                    {
                        if ch == channel && (show_name != new_name || genres != new_genres) {
                            *show_name = new_name.clone();
                            *genres = new_genres.clone();
                            changed = true;
                        }
                    }
                }
            }
        }
        changed
    }

    /// Update the stream metadata of the current item (e.g. from ICY metadata).
    pub fn set_current_stream_metadata(&mut self, metadata: StreamMetadata) {
        if let Some(i) = self.current_index {
            if let Some(item) = self.items.get_mut(i) {
                item.stream_metadata = Some(metadata);
            }
        }
    }
}
