// Text input for filtering the discovery list. Activated with `/`.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{layout::Rect, style::Style, widgets::Paragraph, Frame};
use tokio::sync::mpsc::UnboundedSender;

use crate::action::Action;
use crate::components::Component;
use crate::theme::Theme;

/// Text input for live-filtering the discovery list (activated with `/`).
#[derive(Default)]
pub struct SearchBar {
    action_tx: Option<UnboundedSender<Action>>,
    input: String,
    focused: bool,
}

impl SearchBar {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_focused(&self) -> bool {
        self.focused
    }

    pub fn input(&self) -> &str {
        &self.input
    }
}

impl Component for SearchBar {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) {
        self.action_tx = Some(tx);
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> anyhow::Result<bool> {
        if !self.focused {
            return Ok(false);
        }
        let tx = self.action_tx.as_ref().expect("component not registered");
        match key.code {
            KeyCode::Char(c) => {
                self.input.push(c);
                Ok(true)
            }
            KeyCode::Backspace => {
                self.input.pop();
                Ok(true)
            }
            KeyCode::Enter => {
                if !self.input.is_empty() {
                    self.focused = false;
                    tx.send(Action::SearchSubmit)?;
                }
                Ok(true)
            }
            KeyCode::Esc => {
                self.focused = false;
                self.input.clear();
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn update(&mut self, action: &Action) -> anyhow::Result<Vec<Action>> {
        match action {
            Action::FocusSearch => {
                self.focused = true;
            }
            Action::Back => {
                self.focused = false;
                self.input.clear();
            }
            Action::SearchSubmit => {
                // Keep input visible so user can see what they searched for
                self.focused = false;
            }
            _ => {}
        }
        Ok(vec![])
    }

    fn draw(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let style = if self.focused {
            Style::default().fg(theme.accent)
        } else {
            Style::default().fg(theme.text_dim)
        };

        let display = if self.input.is_empty() && !self.focused {
            "/ Search...".to_string()
        } else if self.focused {
            format!("/ {}_", self.input)
        } else {
            format!("/ {}", self.input)
        };

        let paragraph = Paragraph::new(display).style(style);
        frame.render_widget(paragraph, area);
    }
}
