// src/components/mod.rs

pub mod discovery_list;
pub mod search_bar;
pub mod now_playing;
pub mod play_controls;
pub mod nts;
pub mod direct_play_modal;

use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;
use tokio::sync::mpsc::UnboundedSender;

use crate::action::Action;

pub trait Component {
    /// Register the action sender for this component.
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>);

    /// Handle a key event. Return Ok(true) if the event was consumed.
    fn handle_key_event(&mut self, key: KeyEvent) -> anyhow::Result<bool> {
        let _ = key;
        Ok(false)
    }

    /// Handle an action dispatched by App. Return optional follow-up actions.
    fn update(&mut self, action: &Action) -> anyhow::Result<Vec<Action>> {
        let _ = action;
        Ok(vec![])
    }

    /// Render this component into the given area.
    fn draw(&self, frame: &mut Frame, area: Rect);
}
