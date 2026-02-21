// src/components/play_controls.rs

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use tokio::sync::mpsc::UnboundedSender;

use crate::action::Action;
use crate::components::Component;

const BRAILLE_SPINNER: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

pub struct PlayControls {
    action_tx: Option<UnboundedSender<Action>>,
    pub playing: bool,
    pub paused: bool,
    pub buffering: bool,
    pub queue_pos: Option<usize>,
    pub queue_len: usize,
    pub volume: Option<u8>,
    pub current_title: Option<String>,
    frame_count: u64,
}

impl PlayControls {
    pub fn new() -> Self {
        Self {
            action_tx: None,
            playing: false,
            paused: false,
            buffering: false,
            queue_pos: None,
            queue_len: 0,
            volume: None,
            current_title: None,
            frame_count: 0,
        }
    }
}

impl Component for PlayControls {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) {
        self.action_tx = Some(tx);
    }

    fn update(&mut self, action: &Action) -> anyhow::Result<Vec<Action>> {
        match action {
            Action::Tick => { self.frame_count = self.frame_count.wrapping_add(1); }
            Action::PlaybackLoading => { self.buffering = true; }
            Action::PlaybackStarted { ref title, .. } => { self.playing = true; self.paused = false; self.buffering = false; self.current_title = Some(title.clone()); }
            Action::PlaybackPosition(_) => { self.buffering = false; }
            Action::PlaybackFinished | Action::Stop => { self.playing = false; self.paused = false; self.buffering = false; self.current_title = None; }
            Action::TogglePlayPause => { self.paused = !self.paused; }
            Action::VolumeChanged(vol) => { self.volume = Some(*vol); }
            _ => {}
        }
        Ok(vec![])
    }

    fn draw(&self, frame: &mut Frame, area: Rect) {
        let status = if self.buffering {
            let idx = (self.frame_count / 3) as usize % BRAILLE_SPINNER.len();
            BRAILLE_SPINNER[idx]
        } else if self.paused {
            "⏸"
        } else if self.playing {
            if self.frame_count % 30 < 15 { "♪ ▶" } else { "♫ ▶" }
        } else {
            "■"
        };

        let status_color = if self.buffering {
            Color::Yellow
        } else if self.playing && !self.paused {
            Color::Green
        } else {
            Color::DarkGray
        };

        let border_color = if self.playing && !self.paused {
            Color::Cyan
        } else {
            Color::DarkGray
        };

        let queue_info = if self.queue_len > 0 {
            format!("Track {}/{}", self.queue_pos.unwrap_or(0) + 1, self.queue_len)
        } else {
            String::new()
        };

        let div = Span::styled(" │ ", Style::default().fg(Color::DarkGray));
        let key_style = Style::default().fg(Color::White);
        let desc_style = Style::default().fg(Color::DarkGray);

        // Build right-side track name for line 1
        let track_display = self.current_title.as_deref().unwrap_or("");

        let mut line1_spans = vec![
            Span::styled(format!(" {} ", status), Style::default().fg(status_color).add_modifier(Modifier::BOLD)),
            div.clone(),
            Span::styled("Space", key_style),
            Span::styled(" Play/Pause", desc_style),
            div.clone(),
            Span::styled("n", key_style),
            Span::styled(" Next", desc_style),
            div.clone(),
            Span::styled("p", key_style),
            Span::styled(" Prev", desc_style),
            div.clone(),
            Span::styled("f", key_style),
            Span::styled(" Fav", desc_style),
            div.clone(),
            Span::styled("o", key_style),
            Span::styled(" URL", desc_style),
        ];

        if !track_display.is_empty() {
            // Calculate used width so far
            let used: usize = line1_spans.iter().map(|s| s.content.len()).sum();
            let available = (area.width as usize).saturating_sub(used + 4);
            if available > 5 {
                let truncated: String = track_display.chars().take(available).collect();
                line1_spans.push(Span::raw("  "));
                line1_spans.push(Span::styled(truncated, Style::default().fg(Color::Cyan)));
            }
        }

        let line1 = Line::from(line1_spans);

        let vol_info = self.volume.map(|v| format!("Vol {}%", v)).unwrap_or_default();

        let line2 = Line::from(vec![
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
            Span::styled(vol_info, Style::default().fg(Color::Cyan)),
            Span::raw("  "),
            Span::styled(queue_info, Style::default().fg(Color::Cyan)),
        ]);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color));

        let paragraph = Paragraph::new(vec![line1, line2]).block(block);
        frame.render_widget(paragraph, area);
    }
}
