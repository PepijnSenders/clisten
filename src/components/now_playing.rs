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
use crate::components::Component;

pub struct NowPlaying {
    action_tx: Option<UnboundedSender<Action>>,
    pub current_item: Option<DiscoveryItem>,
    pub position_secs: f64,
    pub paused: bool,
    pub buffering: bool,
    pub stream_metadata: Option<String>,
    queue_items: Vec<(String, String)>,
    queue_current: Option<usize>,
    blob: BlobVisualizer,
    frame_count: u64,
}

impl NowPlaying {
    pub fn new() -> Self {
        Self {
            action_tx: None,
            current_item: None,
            position_secs: 0.0,
            paused: false,
            buffering: false,
            stream_metadata: None,
            queue_items: Vec::new(),
            queue_current: None,
            blob: BlobVisualizer::new(),
            frame_count: 0,
        }
    }

    pub fn set_queue(&mut self, items: Vec<(String, String)>, current_index: Option<usize>) {
        self.queue_items = items;
        self.queue_current = current_index;
    }

    fn draw_queue(&self, frame: &mut Frame, area: Rect) {
        let buf = frame.buffer_mut();
        for x in area.x..area.x + area.width {
            if let Some(cell) = buf.cell_mut((x, area.y)) {
                cell.set_char('─');
                cell.set_fg(Color::DarkGray);
            }
        }

        let title = Line::from(Span::styled(
            format!(" Queue ({})", self.queue_items.len()),
            Style::default().fg(Color::DarkGray),
        ));
        let title_area = Rect { x: area.x, y: area.y + 1, width: area.width, height: 1 };
        frame.render_widget(Paragraph::new(title), title_area);

        let inner = Rect {
            x: area.x + 1,
            y: area.y + 2,
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(2),
        };

        let lines: Vec<Line> = self
            .queue_items
            .iter()
            .enumerate()
            .map(|(i, (title, subtitle))| {
                let is_current = self.queue_current == Some(i);
                let marker = if is_current { "▶ " } else { "  " };
                let style = if is_current {
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                let sub_style = if is_current {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                Line::from(vec![
                    Span::styled(marker, style),
                    Span::styled(title.as_str(), style),
                    Span::styled(
                        if subtitle.is_empty() {
                            String::new()
                        } else {
                            format!(" - {}", subtitle)
                        },
                        sub_style,
                    ),
                ])
            })
            .collect();

        frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), inner);
    }
}

impl Component for NowPlaying {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) {
        self.action_tx = Some(tx);
    }

    fn update(&mut self, action: &Action) -> anyhow::Result<Vec<Action>> {
        match action {
            Action::Tick => {
                self.frame_count = self.frame_count.wrapping_add(1);
                self.blob.tick(
                    self.current_item.is_some(),
                    self.paused,
                    self.buffering,
                    self.position_secs,
                );
            }
            Action::PlayItem(item) => {
                self.current_item = Some(item.clone());
                self.position_secs = 0.0;
                self.paused = false;
                self.buffering = true;
                self.stream_metadata = None;
            }
            Action::PlaybackLoading => {
                self.buffering = true;
                self.stream_metadata = None;
            }
            Action::PlaybackPosition(pos) => {
                self.position_secs = *pos;
                self.buffering = false;
            }
            Action::StreamMetadataChanged(title) => {
                self.stream_metadata = Some(title.clone());
            }
            Action::TogglePlayPause => {
                self.paused = !self.paused;
            }
            Action::Stop | Action::PlaybackFinished => {
                self.current_item = None;
                self.position_secs = 0.0;
                self.buffering = false;
                self.stream_metadata = None;
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
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let np_area = chunks[0];
        let title_area = Rect { x: np_area.x, y: np_area.y, width: np_area.width, height: 1 };
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
                self.draw_queue(frame, chunks[1]);
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
            self.draw_queue(frame, chunks[1]);
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

        let mut lines = vec![
            Line::from(Span::styled(
                item.display_title(),
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                item.subtitle(),
                Style::default().fg(Color::Cyan),
            )),
        ];

        if let Some(ref meta) = self.stream_metadata {
            lines.push(Line::from(Span::styled(meta.as_str(), Style::default().fg(Color::Magenta))));
        } else {
            lines.push(Line::from(""));
        }

        if self.buffering {
            lines.push(Line::from(Span::styled(status, Style::default().fg(Color::Yellow))));
        } else {
            lines.push(Line::from(format!("{} {}:{:02}", status, mins, secs)));
        }

        frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), area);
    }

    fn draw_tags(&self, frame: &mut Frame, area: Rect, item: &DiscoveryItem) {
        if area.height == 0 {
            return;
        }
        let mut lines = Vec::new();
        match item {
            DiscoveryItem::NtsEpisode { genres, .. }
            | DiscoveryItem::NtsLiveChannel { genres, .. } => {
                if !genres.is_empty() {
                    lines.push(Line::from(Span::styled(
                        format!("Tags: {}", genres.join(", ")),
                        Style::default().fg(Color::DarkGray),
                    )));
                }
            }
            DiscoveryItem::DirectUrl { url, .. } => {
                lines.push(Line::from(Span::styled(
                    url.chars().take(200).collect::<String>(),
                    Style::default().fg(Color::DarkGray),
                )));
            }
            _ => {}
        }
        if !lines.is_empty() {
            frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), area);
        }
    }
}
