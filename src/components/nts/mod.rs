// Sub-tab bar (Live / Picks / Search) and lazy-load coordinator.

use std::collections::HashSet;
use std::fmt;

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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum NtsSubTab {
    #[default]
    Live,
    Picks,
    Search,
}

impl NtsSubTab {
    pub const ALL: [NtsSubTab; 3] = [Self::Live, Self::Picks, Self::Search];
}

impl fmt::Display for NtsSubTab {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Live => write!(f, "Live"),
            Self::Picks => write!(f, "Picks"),
            Self::Search => write!(f, "Search"),
        }
    }
}

#[derive(Default)]
pub struct NtsTab {
    action_tx: Option<UnboundedSender<Action>>,
    pub active_sub: NtsSubTab,
    loaded: HashSet<NtsSubTab>,
}

impl NtsTab {
    pub fn new() -> Self {
        Self::default()
    }

    /// Switch to a sub-tab by index (0-based).
    pub fn switch_sub_tab(&mut self, index: usize) -> Vec<Action> {
        if let Some(&tab) = NtsSubTab::ALL.get(index) {
            self.active_sub = tab;
            self.load_if_needed()
        } else {
            vec![]
        }
    }

    /// Return the load action if this sub-tab hasn't been loaded yet.
    fn load_if_needed(&mut self) -> Vec<Action> {
        if !self.loaded.insert(self.active_sub) {
            return vec![];
        }
        match self.active_sub {
            NtsSubTab::Live => vec![Action::LoadNtsLive],
            NtsSubTab::Picks => vec![Action::LoadNtsPicks],
            NtsSubTab::Search => vec![Action::LoadGenres],
        }
    }

    /// Get the current sub-tab index (0-based).
    pub fn active_index(&self) -> usize {
        match self.active_sub {
            NtsSubTab::Live => 0,
            NtsSubTab::Picks => 1,
            NtsSubTab::Search => 2,
        }
    }

    /// Force a sub-tab to be re-fetched on next visit.
    pub fn mark_unloaded(&mut self, tab: NtsSubTab) {
        self.loaded.remove(&tab);
    }
}

impl Component for NtsTab {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) {
        self.action_tx = Some(tx);
    }

    fn draw(&self, frame: &mut Frame, area: Rect) {
        let active_idx = self.active_index();

        let mut spans: Vec<Span> = Vec::new();
        spans.push(Span::raw(" "));

        for (i, tab) in NtsSubTab::ALL.iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled(" │ ", Style::default().fg(Color::DarkGray)));
            }
            let label = tab.to_string();
            if i == active_idx {
                spans.push(Span::styled(
                    label,
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                ));
            } else {
                spans.push(Span::styled(label, Style::default().fg(Color::DarkGray)));
            }
        }

        let line = Line::from(spans);
        let block = Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(Color::DarkGray));
        frame.render_widget(Paragraph::new(line).block(block), area);
    }
}
