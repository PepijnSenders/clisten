// Now-playing panel: track info, blob visualizer, and queue display.

use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
    Frame,
};
use tokio::sync::mpsc::UnboundedSender;

use crate::action::Action;
use crate::api::models::DiscoveryItem;
use crate::components::blob::BlobVisualizer;
use crate::components::queue_list;
use crate::components::Component;
use crate::player::StreamMetadata;

#[derive(Default)]
pub struct NowPlaying {
    action_tx: Option<UnboundedSender<Action>>,
    current_item: Option<DiscoveryItem>,
    position_secs: f64,
    paused: bool,
    buffering: bool,
    stream_metadata: Option<StreamMetadata>,
    queue_items: Vec<(String, String)>,
    queue_current: Option<usize>,
    blob: BlobVisualizer,
    audio_rms: f64,
    audio_peak: f64,
}

impl NowPlaying {
    pub fn new() -> Self {
        Self::default()
    }

    /// Prepare for a new track: set the item, reset playback state, clear old metadata.
    pub fn set_buffering(&mut self, item: DiscoveryItem) {
        self.current_item = Some(item);
        self.position_secs = 0.0;
        self.paused = false;
        self.buffering = true;
        self.stream_metadata = None;
    }

    /// Clear all playback state (called on stop / playback finished).
    fn reset(&mut self) {
        self.current_item = None;
        self.position_secs = 0.0;
        self.buffering = false;
        self.stream_metadata = None;
        self.audio_rms = 0.0;
        self.audio_peak = 0.0;
    }

    pub fn set_queue(&mut self, items: Vec<(String, String)>, current_index: Option<usize>) {
        self.queue_items = items;
        self.queue_current = current_index;
    }

    pub fn is_playing(&self) -> bool {
        self.current_item.is_some()
    }

    #[allow(dead_code)] // used by integration tests
    pub fn position_secs(&self) -> f64 {
        self.position_secs
    }
    #[allow(dead_code)] // used by integration tests
    pub fn is_paused(&self) -> bool {
        self.paused
    }
}

impl Component for NowPlaying {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) {
        self.action_tx = Some(tx);
    }

    fn update(&mut self, action: &Action) -> anyhow::Result<Vec<Action>> {
        match action {
            Action::Tick => {
                self.blob.tick(
                    self.current_item.is_some(),
                    self.paused,
                    self.buffering,
                    self.position_secs,
                    self.audio_rms,
                    self.audio_peak,
                );
            }
            Action::AudioLevels { rms, peak } => {
                self.audio_rms = *rms;
                self.audio_peak = *peak;
            }
            Action::PlayItem(item) => {
                self.set_buffering(item.clone());
            }
            Action::PlaybackPosition(pos) => {
                self.position_secs = *pos;
                self.buffering = false;
            }
            Action::StreamMetadataChanged(metadata) => {
                self.stream_metadata = Some(metadata.clone());
            }
            Action::TogglePlayPause => {
                self.paused = !self.paused;
            }
            Action::Stop | Action::PlaybackFinished => {
                self.reset();
            }
            _ => {}
        }
        Ok(vec![])
    }

    fn draw(&self, frame: &mut Frame, area: Rect) {
        let has_queue = !self.queue_items.is_empty();
        let chunks = if has_queue {
            Layout::vertical([Constraint::Min(7), Constraint::Percentage(50)]).split(area)
        } else {
            Layout::vertical([Constraint::Min(0)]).split(area)
        };

        // Section header
        let title_style = if self.current_item.is_some() && !self.paused {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let np_area = chunks[0];
        let title_area = Rect {
            x: np_area.x,
            y: np_area.y,
            width: np_area.width,
            height: 1,
        };
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(" Now Playing", title_style))),
            title_area,
        );

        let inner = Rect {
            x: np_area.x + 1,
            y: np_area.y + 1,
            width: np_area.width.saturating_sub(2),
            height: np_area.height.saturating_sub(1),
        };

        let Some(item) = &self.current_item else {
            frame.render_widget(
                Paragraph::new("Nothing playing").style(Style::default().fg(Color::DarkGray)),
                inner,
            );
            if has_queue {
                queue_list::draw(frame, chunks[1], &self.queue_items, self.queue_current);
            }
            return;
        };

        // Layout: track info | blob visualizer | tags
        let has_tags = matches!(
            item,
            DiscoveryItem::NtsEpisode { genres, .. }
            | DiscoveryItem::NtsLiveChannel { genres, .. }
            if !genres.is_empty()
        );
        let has_url = matches!(item, DiscoveryItem::DirectUrl { .. });
        let tag_height = if has_tags || has_url { 2 } else { 0 };

        let inner_chunks = Layout::vertical([
            Constraint::Length(4),
            Constraint::Min(4),
            Constraint::Length(tag_height),
        ])
        .split(inner);

        // Track info
        self.draw_track_info(frame, inner_chunks[0], item);

        // Blob visualizer
        self.blob.draw(frame, inner_chunks[1]);

        // Tags / URL
        self.draw_tags(frame, inner_chunks[2], item);

        if has_queue {
            queue_list::draw(frame, chunks[1], &self.queue_items, self.queue_current);
        }
    }
}

impl NowPlaying {
    fn draw_track_info(&self, frame: &mut Frame, area: Rect, item: &DiscoveryItem) {
        let mins = self.position_secs as u64 / 60;
        let secs = self.position_secs as u64 % 60;
        let status = if self.buffering {
            "⟳ Loading..."
        } else if self.paused {
            "⏸"
        } else {
            "▶"
        };

        let m = self.stream_metadata.as_ref();
        let (title_text, subtitle_text) = item.display_pair(
            m.and_then(|m| m.station_name.as_deref()),
            m.and_then(|m| m.display_title()).as_deref(),
            m.and_then(|m| m.display_subtitle()).as_deref(),
        );

        // NTS items: show stream metadata as a third line (DirectUrl items
        // fold metadata into title/subtitle via display_pair instead).
        let meta_line = if !matches!(item, DiscoveryItem::DirectUrl { .. }) {
            self.stream_metadata
                .as_ref()
                .and_then(|m| m.display_title())
        } else {
            None
        };

        let mut lines = vec![
            Line::from(Span::styled(
                title_text,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                subtitle_text,
                Style::default().fg(Color::Cyan),
            )),
        ];

        if let Some(meta) = meta_line {
            lines.push(Line::from(Span::styled(
                meta,
                Style::default().fg(Color::Magenta),
            )));
        } else {
            lines.push(Line::from(""));
        }

        if self.buffering {
            lines.push(Line::from(Span::styled(
                status,
                Style::default().fg(Color::Yellow),
            )));
        } else {
            lines.push(Line::from(format!("{} {}:{:02}", status, mins, secs)));
        }

        frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), area);
    }

    fn draw_tags(&self, frame: &mut Frame, area: Rect, item: &DiscoveryItem) {
        let text: Option<String> = match item {
            DiscoveryItem::NtsEpisode { genres, .. }
            | DiscoveryItem::NtsLiveChannel { genres, .. }
                if !genres.is_empty() =>
            {
                Some(format!("Tags: {}", genres.join(", ")))
            }
            DiscoveryItem::DirectUrl { url, .. } => Some(url.chars().take(200).collect()),
            _ => None,
        };
        if let Some(text) = text {
            let line = Line::from(Span::styled(text, Style::default().fg(Color::DarkGray)));
            frame.render_widget(Paragraph::new(line).wrap(Wrap { trim: true }), area);
        }
    }
}
