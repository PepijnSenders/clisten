// UI components. Each implements the Component trait: register for actions,
// handle key events, update state, and draw into a ratatui frame.

pub mod blob;
pub mod direct_play_modal;
pub mod discovery_list;
pub mod now_playing;
pub mod nts;
pub mod play_controls;
pub mod queue_list;
pub mod search_bar;

use crossterm::event::KeyEvent;
use ratatui::layout::Rect;
use ratatui::Frame;
use tokio::sync::mpsc::UnboundedSender;

use crate::action::Action;

/// Braille dot spinner frames, shared by loading indicators.
pub const BRAILLE_SPINNER: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

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
