// src/components/nts/mod.rs

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use strum::{Display, EnumIter, IntoEnumIterator};
use tokio::sync::mpsc::UnboundedSender;

use crate::action::Action;
use crate::components::Component;

#[derive(Debug, Clone, Copy, PartialEq, EnumIter, Display)]
pub enum NtsSubTab {
    Live,
    Picks,
    Search,
}

pub struct NtsTab {
    action_tx: Option<UnboundedSender<Action>>,
    pub active_sub: NtsSubTab,
    pub loaded: std::collections::HashSet<String>,
}

impl NtsTab {
    pub fn new() -> Self {
        Self {
            action_tx: None,
            active_sub: NtsSubTab::Live,
            loaded: std::collections::HashSet::new(),
        }
    }

    /// Switch to a sub-tab by index (0-based).
    pub fn switch_sub_tab(&mut self, index: usize) -> Vec<Action> {
        let tabs: Vec<NtsSubTab> = NtsSubTab::iter().collect();
        if let Some(&tab) = tabs.get(index) {
            self.active_sub = tab;
            self.load_if_needed()
        } else {
            vec![]
        }
    }

    /// Return the load action if this sub-tab hasn't been loaded yet.
    fn load_if_needed(&mut self) -> Vec<Action> {
        let key = format!("{:?}", self.active_sub);
        if self.loaded.contains(&key) {
            return vec![];
        }
        self.loaded.insert(key);

        match self.active_sub {
            NtsSubTab::Live => vec![Action::LoadNtsLive],
            NtsSubTab::Picks => vec![Action::LoadNtsPicks],
            NtsSubTab::Search => vec![Action::LoadGenres],
        }
    }

    /// Get the current sub-tab index.
    pub fn active_index(&self) -> usize {
        NtsSubTab::iter().position(|t| t == self.active_sub).unwrap_or(0)
    }
}

impl Component for NtsTab {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) {
        self.action_tx = Some(tx);
    }

    fn update(&mut self, action: &Action) -> anyhow::Result<Vec<Action>> {
        match action {
            Action::SwitchSubTab(idx) => {
                return Ok(self.switch_sub_tab(*idx));
            }
            _ => {}
        }
        Ok(vec![])
    }

    fn draw(&self, frame: &mut Frame, area: Rect) {
        let active_idx = NtsSubTab::iter()
            .position(|t| t == self.active_sub)
            .unwrap_or(0);

        let mut spans: Vec<Span> = Vec::new();
        spans.push(Span::raw(" "));

        for (i, tab) in NtsSubTab::iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled(" │ ", Style::default().fg(Color::DarkGray)));
            }
            let label = format!("{}", tab);
            if i == active_idx {
                spans.push(Span::styled(
                    label,
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                ));
            } else {
                spans.push(Span::styled(
                    label,
                    Style::default().fg(Color::DarkGray),
                ));
            }
        }

        let line = Line::from(spans);
        let block = Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(Color::DarkGray));
        frame.render_widget(Paragraph::new(line).block(block), area);
    }
}
