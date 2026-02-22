// Scrollable, filterable list of DiscoveryItems (left panel). Handles
// keyboard navigation, text filtering, and progressive append for search results.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, ListState, Paragraph},
    Frame,
};
use tokio::sync::mpsc::UnboundedSender;

use crate::action::Action;
use crate::api::models::DiscoveryItem;
use crate::components::{Component, BRAILLE_SPINNER};

#[derive(Default)]
pub struct DiscoveryList {
    action_tx: Option<UnboundedSender<Action>>,
    /// Full unfiltered dataset
    all_items: Vec<DiscoveryItem>,
    /// Currently visible items (filtered or full)
    items: Vec<DiscoveryItem>,
    state: ListState,
    filter_query: Option<String>,
    loading: bool,
    frame_count: u64,
}

impl DiscoveryList {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_items(&mut self, items: Vec<DiscoveryItem>) {
        self.all_items = items;
        self.refilter();
        self.loading = false;
    }

    pub fn set_filter(&mut self, query: Option<String>) {
        self.filter_query = query;
        self.refilter();
    }

    pub fn append_items(&mut self, new_items: Vec<DiscoveryItem>) {
        let prev_selected = self.state.selected();
        self.all_items.extend(new_items);
        self.refilter();
        // Preserve scroll position when appending
        if let Some(idx) = prev_selected {
            if idx < self.items.len() {
                self.state.select(Some(idx));
            }
        }
    }

    /// Rebuild the visible items list from all_items + current filter.
    fn refilter(&mut self) {
        match self.filter_query {
            Some(ref q) => {
                let q = q.to_lowercase();
                self.items = self
                    .all_items
                    .iter()
                    .filter(|item| {
                        item.title().to_lowercase().contains(&q)
                            || item.subtitle().to_lowercase().contains(&q)
                    })
                    .cloned()
                    .collect();
            }
            None => {
                self.items = self.all_items.clone();
            }
        }
        self.state
            .select(if self.items.is_empty() { None } else { Some(0) });
    }

    #[allow(dead_code)] // used by integration tests
    pub fn visible_items(&self) -> &[DiscoveryItem] {
        &self.items
    }

    #[allow(dead_code)] // used by integration tests
    pub fn total_item_count(&self) -> usize {
        self.all_items.len()
    }

    #[allow(dead_code)] // used by integration tests
    pub fn selected_index(&self) -> Option<usize> {
        self.state.selected()
    }

    pub fn set_loading(&mut self, loading: bool) {
        self.loading = loading;
    }

    #[allow(dead_code)] // used by integration tests
    pub fn is_loading(&self) -> bool {
        self.loading
    }

    pub fn selected_item(&self) -> Option<&DiscoveryItem> {
        self.state.selected().and_then(|i| self.items.get(i))
    }

    pub fn next(&mut self) {
        if self.items.is_empty() {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => (i + 1).min(self.items.len() - 1),
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn prev(&mut self) {
        if self.items.is_empty() {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => i.saturating_sub(1),
            None => 0,
        };
        self.state.select(Some(i));
    }
}

impl Component for DiscoveryList {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) {
        self.action_tx = Some(tx);
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> anyhow::Result<bool> {
        let tx = self.action_tx.as_ref().expect("component not registered");
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                self.next();
                Ok(true)
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.prev();
                Ok(true)
            }
            KeyCode::Enter => {
                if let Some(item) = self.selected_item() {
                    match item {
                        DiscoveryItem::NtsGenre { genre_id, .. } => {
                            tx.send(Action::SearchByGenre {
                                genre_id: genre_id.clone(),
                            })?;
                        }
                        _ => {
                            tx.send(Action::PlayItem(item.clone()))?;
                        }
                    }
                }
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn update(&mut self, action: &Action) -> anyhow::Result<Vec<Action>> {
        if matches!(action, Action::Tick) {
            self.frame_count = self.frame_count.wrapping_add(1);
        }
        Ok(vec![])
    }

    fn draw(&self, frame: &mut Frame, area: Rect) {
        if self.loading {
            let idx = (self.frame_count / 3) as usize % BRAILLE_SPINNER.len();
            let spinner = BRAILLE_SPINNER[idx];
            let paragraph = Paragraph::new(Line::from(vec![
                Span::styled(format!("  {} ", spinner), Style::default().fg(Color::Cyan)),
                Span::styled("Searching...", Style::default().fg(Color::DarkGray)),
            ]));
            frame.render_widget(paragraph, area);
            return;
        }

        let selected = self.state.selected();
        let items: Vec<ListItem> = self
            .items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let is_selected = selected == Some(i);
                let num = format!("{:02} ", i + 1);

                let title_style = if is_selected {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                let subtitle_color = if is_selected {
                    Color::Cyan
                } else if i % 2 == 0 {
                    Color::DarkGray
                } else {
                    Color::Gray
                };

                let bg = if is_selected {
                    Some(Color::Rgb(30, 30, 40))
                } else {
                    None
                };

                let line_spans = vec![
                    Span::styled(num, Style::default().fg(Color::DarkGray)),
                    Span::styled(item.title(), title_style),
                ];

                let title_line = Line::from(line_spans);
                let sub_line = Line::from(vec![
                    Span::styled("   ", Style::default().fg(Color::DarkGray)),
                    Span::styled(item.subtitle(), Style::default().fg(subtitle_color)),
                ]);

                let mut list_item = ListItem::new(vec![title_line, sub_line]);
                if let Some(bg_color) = bg {
                    list_item = list_item.style(Style::default().bg(bg_color));
                }
                list_item
            })
            .collect();

        let list = List::new(items)
            .highlight_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▌");

        frame.render_stateful_widget(list, area, &mut self.state.clone());
    }
}
