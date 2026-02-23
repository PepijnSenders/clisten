// Modal overlay for precise seeking within a track (press `t` to open).

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use tokio::sync::mpsc::UnboundedSender;

use crate::action::Action;
use crate::components::{centered_overlay, format_time, Component};
use crate::theme::Theme;

/// Modal overlay for precise seeking within an on-demand track.
pub struct SeekModal {
    action_tx: Option<UnboundedSender<Action>>,
    visible: bool,
    position: f64,
    duration: f64,
    cursor_position: f64,
}

impl Default for SeekModal {
    fn default() -> Self {
        Self {
            action_tx: None,
            visible: false,
            position: 0.0,
            duration: 0.0,
            cursor_position: 0.0,
        }
    }
}

impl SeekModal {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn show(&mut self, position: f64, duration: f64) {
        self.visible = true;
        self.position = position;
        self.duration = duration;
        self.cursor_position = position;
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn update_position(&mut self, position: f64) {
        self.position = position;
    }

    pub fn update_duration(&mut self, duration: f64) {
        self.duration = duration;
    }

    fn move_cursor(&mut self, delta: f64) {
        self.cursor_position = (self.cursor_position + delta).clamp(0.0, self.duration);
    }
}

impl Component for SeekModal {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) {
        self.action_tx = Some(tx);
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> anyhow::Result<bool> {
        if !self.visible {
            return Ok(false);
        }

        match key.code {
            KeyCode::Esc | KeyCode::Char('t') => {
                if let Some(tx) = &self.action_tx {
                    tx.send(Action::CloseSeekModal).ok();
                }
            }
            KeyCode::Enter => {
                let offset = self.cursor_position - self.position;
                if let Some(tx) = &self.action_tx {
                    tx.send(Action::SeekRelative(offset)).ok();
                    tx.send(Action::CloseSeekModal).ok();
                }
            }
            KeyCode::Left => self.move_cursor(-5.0),
            KeyCode::Right => self.move_cursor(5.0),
            KeyCode::Char('h') => self.move_cursor(-30.0),
            KeyCode::Char('l') => self.move_cursor(30.0),
            KeyCode::Char('0') => self.cursor_position = 0.0,
            KeyCode::Char('$') => self.cursor_position = self.duration,
            _ => {}
        }

        Ok(true)
    }

    fn draw(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if !self.visible {
            return;
        }

        let overlay_area = centered_overlay(area, 60, 7);

        frame.render_widget(Clear, overlay_area);

        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Seek Timeline ")
            .title_style(
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            );

        let inner = block.inner(overlay_area);
        frame.render_widget(block, overlay_area);

        // Time line: cursor_time / duration (now: current_time)
        let time_line = Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(
                format_time(self.cursor_position),
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" / {}", format_time(self.duration)),
                Style::default().fg(theme.text),
            ),
            Span::styled(
                format!("  (now: {})", format_time(self.position)),
                Style::default().fg(theme.text_dim),
            ),
        ]);

        // Visual seek bar
        let bar_width = (inner.width as usize).saturating_sub(4);
        let bar = if self.duration > 0.0 && bar_width > 2 {
            let pos_frac = (self.position / self.duration).clamp(0.0, 1.0);
            let cursor_frac = (self.cursor_position / self.duration).clamp(0.0, 1.0);
            let pos_idx = (pos_frac * (bar_width - 1) as f64).round() as usize;
            let cursor_idx = (cursor_frac * (bar_width - 1) as f64).round() as usize;

            let mut chars: Vec<(&str, ratatui::style::Color)> = Vec::with_capacity(bar_width);
            for i in 0..bar_width {
                if i == cursor_idx && i == pos_idx {
                    // Both markers at same position — show cursor
                    chars.push(("┃", theme.accent));
                } else if i == cursor_idx {
                    chars.push(("┃", theme.accent));
                } else if i == pos_idx {
                    chars.push(("▶", theme.primary));
                } else if i < pos_idx {
                    chars.push(("━", theme.primary));
                } else {
                    chars.push(("─", theme.text_dim));
                }
            }

            let mut spans = vec![Span::raw("  ")];
            for (ch, color) in &chars {
                spans.push(Span::styled(*ch, Style::default().fg(*color)));
            }
            Line::from(spans)
        } else {
            Line::from("")
        };

        // Hint line
        let hint = Line::from(Span::styled(
            "  ←→ ±5s · h/l ±30s · 0/$ start/end · Enter seek · Esc close",
            Style::default().fg(theme.text_dim),
        ));

        let paragraph = Paragraph::new(vec![time_line, Line::from(""), bar, hint]);
        frame.render_widget(paragraph, inner);
    }
}
