// UI components. Each implements the Component trait: register for actions,
// handle key events, update state, and draw into a ratatui frame.

pub mod direct_play_modal;
pub mod discovery_list;
pub mod now_playing;
pub mod nts;
pub mod onboarding;
pub mod play_controls;
pub mod queue_list;
pub mod search_bar;
pub mod seek_modal;
pub mod visualizers;

use crossterm::event::KeyEvent;
use ratatui::layout::Rect;
use ratatui::Frame;
use tokio::sync::mpsc::UnboundedSender;

use crate::action::Action;
use crate::theme::Theme;

/// Braille dot spinner frames, shared by loading indicators.
pub const BRAILLE_SPINNER: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Format seconds as "M:SS".
pub fn format_time(secs: f64) -> String {
    let total = secs as u64;
    format!("{}:{:02}", total / 60, total % 60)
}

/// Compute a centered overlay rectangle within `area`, clamped to fit.
pub fn centered_overlay(area: Rect, width: u16, height: u16) -> Rect {
    let w = width.min(area.width.saturating_sub(4));
    let h = height.min(area.height.saturating_sub(2));
    Rect::new(
        area.x + area.width.saturating_sub(w) / 2,
        area.y + area.height.saturating_sub(h) / 2,
        w,
        h,
    )
}

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
    fn draw(&self, frame: &mut Frame, area: Rect, theme: &Theme);
}
