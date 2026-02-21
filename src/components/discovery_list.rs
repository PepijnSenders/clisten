// src/components/discovery_list.rs

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

pub struct DiscoveryList {
    action_tx: Option<UnboundedSender<Action>>,
    /// Full unfiltered dataset
    pub all_items: Vec<DiscoveryItem>,
    /// Currently visible items (filtered or full)
    pub items: Vec<DiscoveryItem>,
    pub state: ListState,
    pub favorites: std::collections::HashSet<String>,
    pub filter_query: Option<String>,
    pub loading: bool,
    pub frame_count: u64,
}

impl DiscoveryList {
    pub fn new() -> Self {
        Self {
            action_tx: None,
            all_items: vec![],
            items: vec![],
            state: ListState::default(),
            favorites: std::collections::HashSet::new(),
            filter_query: None,
            loading: false,
            frame_count: 0,
        }
    }

    pub fn set_items(&mut self, items: Vec<DiscoveryItem>) {
        self.all_items = items.clone();
        // Apply current filter if any
        if let Some(ref query) = self.filter_query.clone() {
            self.apply_filter(query);
        } else {
            self.items = items;
            self.state.select(if self.items.is_empty() { None } else { Some(0) });
        }
        self.loading = false;
    }

    pub fn set_filter(&mut self, query: Option<String>) {
        self.filter_query = query.clone();
        match query {
            Some(ref q) => self.apply_filter(q),
            None => {
                self.items = self.all_items.clone();
                self.state.select(if self.items.is_empty() { None } else { Some(0) });
            }
        }
    }

    fn apply_filter(&mut self, query: &str) {
        let q = query.to_lowercase();
        self.items = self.all_items.iter().filter(|item| {
            item.title().to_lowercase().contains(&q)
                || item.subtitle().to_lowercase().contains(&q)
        }).cloned().collect();
        self.state.select(if self.items.is_empty() { None } else { Some(0) });
    }

    /// Returns the currently visible items (respects filter)
    #[allow(dead_code)]
    pub fn visible_items(&self) -> &[DiscoveryItem] {
        &self.items
    }

    pub fn append_items(&mut self, new_items: Vec<DiscoveryItem>) {
        self.all_items.extend(new_items);
        // Re-apply filter if any
        if let Some(ref query) = self.filter_query.clone() {
            let selected = self.state.selected();
            self.apply_filter(query);
            // Preserve scroll position
            if let Some(idx) = selected {
                if idx < self.items.len() {
                    self.state.select(Some(idx));
                }
            }
        } else {
            self.items = self.all_items.clone();
        }
    }

    #[allow(dead_code)]
    pub fn set_loading(&mut self, loading: bool) {
        self.loading = loading;
    }

    #[allow(dead_code)]
    pub fn is_loading(&self) -> bool {
        self.loading
    }

    pub fn selected_item(&self) -> Option<&DiscoveryItem> {
        self.state.selected().and_then(|i| self.items.get(i))
    }

    pub fn set_favorites(&mut self, favorites: std::collections::HashSet<String>) {
        self.favorites = favorites;
    }

    pub fn next(&mut self) {
        if self.items.is_empty() { return; }
        let i = match self.state.selected() {
            Some(i) => (i + 1).min(self.items.len() - 1),
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn prev(&mut self) {
        if self.items.is_empty() { return; }
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
        let tx = self.action_tx.as_ref().unwrap();
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => { self.next(); Ok(true) }
            KeyCode::Up | KeyCode::Char('k') => { self.prev(); Ok(true) }
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
            KeyCode::Char('a') => {
                if let Some(item) = self.selected_item() {
                    tx.send(Action::AddToQueue(item.clone()))?;
                }
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn update(&mut self, action: &Action) -> anyhow::Result<Vec<Action>> {
        match action {
            Action::Tick => {
                self.frame_count = self.frame_count.wrapping_add(1);
            }
            Action::FilterList(query) => {
                self.set_filter(Some(query.clone()));
            }
            Action::ClearFilter => {
                self.set_filter(None);
            }
            _ => {}
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
        let items: Vec<ListItem> = self.items.iter().enumerate().map(|(i, item)| {
            let is_fav = self.favorites.contains(&item.favorite_key());
            let heart = if is_fav { " ♥" } else { "" };
            let is_selected = selected == Some(i);
            let num = format!("{:02} ", i + 1);

            let title_style = if is_selected {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
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
                Span::styled(num.clone(), Style::default().fg(Color::DarkGray)),
                Span::styled(item.title(), title_style),
                Span::styled(heart, Style::default().fg(Color::Red)),
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
        }).collect();

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
