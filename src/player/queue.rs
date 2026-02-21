// src/player/queue.rs

use crate::api::models::DiscoveryItem;

#[derive(Debug, Clone)]
pub struct QueueItem {
    pub item: DiscoveryItem,
    pub url: String,
    pub stream_title: Option<String>,
}

pub struct Queue {
    items: Vec<QueueItem>,
    current_index: Option<usize>,
}

impl Queue {
    pub fn new() -> Self {
        Self { items: vec![], current_index: None }
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

    /// Remove item at index.
    #[allow(dead_code)]
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

    #[allow(dead_code)]
    pub fn current(&self) -> Option<&QueueItem> {
        self.current_index.and_then(|i| self.items.get(i))
    }

    /// Advance to next track. Returns the next item if available.
    pub fn next(&mut self) -> Option<&QueueItem> {
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

    #[allow(dead_code)]
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

    /// Update the stream title of the current item (e.g. from ICY metadata).
    pub fn set_current_stream_title(&mut self, title: String) {
        if let Some(i) = self.current_index {
            if let Some(item) = self.items.get_mut(i) {
                item.stream_title = Some(title);
            }
        }
    }
}
