// Modal dialog for pasting a URL to play directly (press `o` to open).

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
use crate::api::models::DiscoveryItem;
use crate::components::{centered_overlay, Component};
use crate::theme::Theme;

/// Modal dialog for pasting and playing an arbitrary URL.
#[derive(Default)]
pub struct DirectPlayModal {
    action_tx: Option<UnboundedSender<Action>>,
    visible: bool,
    input: String,
    error: Option<String>,
}

impl DirectPlayModal {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn show(&mut self) {
        self.visible = true;
        self.input.clear();
        self.error = None;
    }

    pub fn hide(&mut self) {
        self.visible = false;
        self.input.clear();
        self.error = None;
    }

    fn submit(&mut self) {
        let url = self.input.trim().to_string();
        if url.is_empty() {
            self.visible = false;
            self.input.clear();
            return;
        }
        if !url.starts_with("http://") && !url.starts_with("https://") {
            self.error = Some("URL must start with http:// or https://".to_string());
            return;
        }
        if let Some(tx) = &self.action_tx {
            let item = DiscoveryItem::DirectUrl { url, title: None };
            tx.send(Action::PlayItem(item)).ok();
        }
        self.visible = false;
        self.input.clear();
        self.error = None;
    }
}

impl Component for DirectPlayModal {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) {
        self.action_tx = Some(tx);
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> anyhow::Result<bool> {
        if !self.visible {
            return Ok(false);
        }

        match key.code {
            KeyCode::Esc => {
                if let Some(tx) = &self.action_tx {
                    tx.send(Action::CloseDirectPlay).ok();
                }
            }
            KeyCode::Enter => {
                self.submit();
            }
            KeyCode::Char(c) => {
                self.input.push(c);
                self.error = None;
            }
            KeyCode::Backspace => {
                self.input.pop();
                self.error = None;
            }
            _ => {}
        }

        Ok(true)
    }

    fn draw(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if !self.visible {
            return;
        }

        let overlay_area = centered_overlay(area, 60, 6);

        frame.render_widget(Clear, overlay_area);

        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Open URL ")
            .title_style(
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            );

        let inner = block.inner(overlay_area);
        frame.render_widget(block, overlay_area);

        let prompt = Line::from(vec![
            Span::styled("URL: ", Style::default().fg(theme.accent)),
            Span::raw(&self.input),
            Span::styled("█", Style::default().fg(theme.text)),
        ]);
        let hint = Line::from(Span::styled(
            "  Enter to play · Esc to cancel",
            Style::default().fg(theme.text_dim),
        ));
        let error_line = if let Some(ref err) = self.error {
            Line::from(Span::styled(
                format!("  {}", err),
                Style::default().fg(theme.error),
            ))
        } else {
            Line::from("")
        };

        let paragraph = Paragraph::new(vec![prompt, hint, error_line]);
        frame.render_widget(paragraph, inner);
    }
}
