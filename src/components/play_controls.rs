// Bottom status bar: playback state, keybinding hints, volume, and queue position.

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use tokio::sync::mpsc::UnboundedSender;

use crate::action::Action;
use crate::components::{Component, BRAILLE_SPINNER};
use crate::theme::Theme;

/// Bottom status bar showing playback state, keybinding hints, and queue info.
#[derive(Default)]
pub struct PlayControls {
    action_tx: Option<UnboundedSender<Action>>,
    playing: bool,
    paused: bool,
    buffering: bool,
    queue_pos: Option<usize>,
    queue_len: usize,
    volume: Option<u8>,
    current_title: Option<String>,
    frame_count: u64,
    is_seekable: bool,
    skip_nts_intro: bool,
}

impl PlayControls {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_queue_info(&mut self, pos: Option<usize>, len: usize) {
        self.queue_pos = pos;
        self.queue_len = len;
    }

    pub fn set_buffering(&mut self, buffering: bool) {
        self.buffering = buffering;
    }

    pub fn set_skip_nts_intro(&mut self, val: bool) {
        self.skip_nts_intro = val;
    }

    #[allow(dead_code)] // used by integration tests
    pub fn is_playing(&self) -> bool {
        self.playing
    }
    #[allow(dead_code)] // used by integration tests
    pub fn is_paused(&self) -> bool {
        self.paused
    }
    #[allow(dead_code)] // used by integration tests
    pub fn queue_len(&self) -> usize {
        self.queue_len
    }
    #[allow(dead_code)] // used by integration tests
    pub fn volume(&self) -> Option<u8> {
        self.volume
    }
}

impl Component for PlayControls {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) {
        self.action_tx = Some(tx);
    }

    fn update(&mut self, action: &Action) -> anyhow::Result<Vec<Action>> {
        match action {
            Action::Tick => {
                self.frame_count = self.frame_count.wrapping_add(1);
            }
            Action::PlaybackLoading => {
                self.buffering = true;
            }
            Action::PlaybackStarted { ref title, .. } => {
                self.playing = true;
                self.paused = false;
                self.buffering = false;
                self.current_title = Some(title.clone());
            }
            Action::PlaybackPosition(_) => {
                self.buffering = false;
            }
            Action::StreamMetadataChanged(ref metadata) => {
                if let Some(title) = metadata.display_title() {
                    self.current_title = Some(title);
                }
            }
            Action::PlaybackDuration(dur) => {
                self.is_seekable = dur.is_some();
            }
            Action::PlaybackFinished | Action::Stop => {
                self.playing = false;
                self.paused = false;
                self.buffering = false;
                self.current_title = None;
                self.is_seekable = false;
            }
            Action::TogglePlayPause => {
                self.paused = !self.paused;
            }
            Action::VolumeChanged(vol) => {
                self.volume = Some(*vol);
            }
            Action::ToggleSkipIntro => {
                self.skip_nts_intro = !self.skip_nts_intro;
            }
            _ => {}
        }
        Ok(vec![])
    }

    fn draw(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let status = if self.buffering {
            let idx = (self.frame_count / 3) as usize % BRAILLE_SPINNER.len();
            BRAILLE_SPINNER[idx]
        } else if self.paused {
            "⏸"
        } else if self.playing {
            if self.frame_count % 30 < 15 {
                "♪ ▶"
            } else {
                "♫ ▶"
            }
        } else {
            "■"
        };

        let status_color = if self.buffering {
            theme.buffering
        } else if self.playing && !self.paused {
            theme.success
        } else {
            theme.text_dim
        };

        let border_color = if self.playing && !self.paused {
            theme.primary
        } else {
            theme.border
        };

        let queue_info = if self.queue_len > 0 {
            format!(
                "Track {}/{}",
                self.queue_pos.unwrap_or(0) + 1,
                self.queue_len
            )
        } else {
            String::new()
        };

        let div = Span::styled(" │ ", Style::default().fg(theme.border));
        let key_style = Style::default().fg(theme.text);
        let desc_style = Style::default().fg(theme.text_dim);

        // Build right-side track name for line 1
        let track_display = self.current_title.as_deref().unwrap_or("");

        let mut line1_spans = vec![
            Span::styled(
                format!(" {} ", status),
                Style::default()
                    .fg(status_color)
                    .add_modifier(Modifier::BOLD),
            ),
            div.clone(),
            Span::styled("Space", key_style),
            Span::styled(" Play/Pause", desc_style),
            div.clone(),
            Span::styled("o", key_style),
            Span::styled(" URL", desc_style),
            div.clone(),
            Span::styled("v", key_style),
            Span::styled(" Viz", desc_style),
        ];

        if self.is_seekable {
            line1_spans.push(div.clone());
            line1_spans.push(Span::styled("←→", key_style));
            line1_spans.push(Span::styled(" Seek", desc_style));
            line1_spans.push(div.clone());
            line1_spans.push(Span::styled("t", key_style));
            line1_spans.push(Span::styled(" Timeline", desc_style));
        }

        if !track_display.is_empty() {
            // Calculate used width so far
            let used: usize = line1_spans.iter().map(|s| s.content.len()).sum();
            let available = (area.width as usize).saturating_sub(used + 4);
            if available > 5 {
                let truncated: String = track_display.chars().take(available).collect();
                line1_spans.push(Span::raw("  "));
                line1_spans.push(Span::styled(truncated, Style::default().fg(theme.primary)));
            }
        }

        let line1 = Line::from(line1_spans);

        let vol_info = self
            .volume
            .map(|v| format!("Vol {}%", v))
            .unwrap_or_default();

        let mut line2_spans = vec![
            Span::raw("   "),
            Span::styled("/", key_style),
            Span::styled(" Search", desc_style),
            div.clone(),
            Span::styled("Tab", key_style),
            Span::styled(" Switch", desc_style),
            div.clone(),
            Span::styled("?", key_style),
            Span::styled(" Help", desc_style),
            div.clone(),
            Span::styled("[ ]", key_style),
            Span::styled(" Vol", desc_style),
            div.clone(),
            Span::styled("q", key_style),
            Span::styled(" Quit", desc_style),
            Span::raw("   "),
            Span::styled(vol_info, Style::default().fg(theme.primary)),
            Span::raw("  "),
            Span::styled(queue_info, Style::default().fg(theme.primary)),
        ];

        if self.skip_nts_intro {
            line2_spans.push(Span::raw("  "));
            line2_spans.push(Span::styled(
                "⏭ Skip Intro",
                Style::default().fg(theme.accent),
            ));
        }

        let line2 = Line::from(line2_spans);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color));

        let paragraph = Paragraph::new(vec![line1, line2]).block(block);
        frame.render_widget(paragraph, area);
    }
}
