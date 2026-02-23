// Onboarding wizard: dynamic screen overlay shown on first launch or retrigger.
// Each screen has an ID; config tracks which are completed. New screens added to
// ALL_SCREENS will automatically show for existing users on next launch.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use tokio::sync::mpsc::UnboundedSender;

use crate::action::Action;
use crate::components::{centered_overlay, Component};
use crate::theme::{Theme, THEME_DARK, THEME_LIGHT};

pub const SCREEN_WELCOME: &str = "welcome";
pub const SCREEN_BROWSE: &str = "browse_nts";
pub const SCREEN_DIRECT_PLAY: &str = "direct_play";
pub const SCREEN_THEME: &str = "theme_select";

/// All screens in display order. Adding a new entry here auto-triggers
/// onboarding for existing users who haven't seen it.
pub const ALL_SCREENS: &[&str] = &[
    SCREEN_WELCOME,
    SCREEN_BROWSE,
    SCREEN_DIRECT_PLAY,
    SCREEN_THEME,
];

#[derive(Default)]
pub struct Onboarding {
    action_tx: Option<UnboundedSender<Action>>,
    active: bool,
    screens: Vec<&'static str>,
    current_index: usize,
    selected_theme: usize, // 0 = dark, 1 = light
}

impl Onboarding {
    pub fn new() -> Self {
        Self::default()
    }

    /// Activate with a specific subset of screens (e.g. only pending ones).
    pub fn activate(&mut self, screens: Vec<&'static str>) {
        self.screens = screens;
        self.current_index = 0;
        self.selected_theme = 0;
        self.active = true;
    }

    /// Activate showing all screens (used by "Restart Onboarding").
    pub fn activate_all(&mut self) {
        self.activate(ALL_SCREENS.to_vec());
    }

    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    fn next_screen(&mut self) {
        if self.current_index < self.screens.len().saturating_sub(1) {
            self.current_index += 1;
        } else {
            self.complete();
        }
    }

    fn prev_screen(&mut self) {
        if self.current_index > 0 {
            self.current_index -= 1;
        }
    }

    fn complete(&mut self) {
        let theme = if self.selected_theme == 0 {
            THEME_DARK.to_string()
        } else {
            THEME_LIGHT.to_string()
        };
        let completed_screens = self.screens.iter().map(|s| s.to_string()).collect();
        if let Some(tx) = &self.action_tx {
            tx.send(Action::OnboardingComplete {
                theme,
                completed_screens,
            })
            .ok();
        }
    }

    fn progress_dots(&self, theme: &Theme) -> Line<'static> {
        let total = self.screens.len();
        let dots: Vec<Span> = (0..total)
            .map(|i| {
                if i == self.current_index {
                    Span::styled("● ", Style::default().fg(theme.primary))
                } else {
                    Span::styled("○ ", Style::default().fg(theme.text_dim))
                }
            })
            .collect();
        Line::from(dots)
    }
}

impl Component for Onboarding {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) {
        self.action_tx = Some(tx);
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> anyhow::Result<bool> {
        if !self.active {
            return Ok(false);
        }

        let current_screen = self.screens.get(self.current_index).copied().unwrap_or("");

        match key.code {
            KeyCode::Right | KeyCode::Enter => {
                self.next_screen();
            }
            KeyCode::Left => {
                self.prev_screen();
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if current_screen == SCREEN_THEME {
                    self.selected_theme = 1;
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if current_screen == SCREEN_THEME {
                    self.selected_theme = 0;
                }
            }
            KeyCode::Char('q') => {
                if let Some(tx) = &self.action_tx {
                    tx.send(Action::Quit).ok();
                }
            }
            _ => {}
        }

        Ok(true)
    }

    fn draw(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if !self.active {
            return;
        }

        frame.render_widget(Clear, area);

        let overlay_area = centered_overlay(area, 50, 18);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border));
        let inner = block.inner(overlay_area);
        frame.render_widget(block, overlay_area);

        let current_screen = self.screens.get(self.current_index).copied().unwrap_or("");
        let mut lines = match current_screen {
            SCREEN_WELCOME => self.screen_welcome(theme),
            SCREEN_BROWSE => self.screen_browse(theme),
            SCREEN_DIRECT_PLAY => self.screen_direct_play(theme),
            SCREEN_THEME => self.screen_theme(theme),
            _ => vec![],
        };
        lines.push(self.progress_dots(theme));

        let paragraph = Paragraph::new(lines).alignment(Alignment::Center);
        frame.render_widget(paragraph, inner);
    }
}

fn screen_title(text: &str, theme: &Theme) -> Line<'static> {
    Line::from(Span::styled(
        text.to_string(),
        Style::default()
            .fg(theme.primary)
            .add_modifier(Modifier::BOLD),
    ))
}

impl Onboarding {
    fn screen_welcome(&self, theme: &Theme) -> Vec<Line<'static>> {
        let dim = Style::default().fg(theme.text_dim);
        vec![
            Line::from(""),
            Line::from(""),
            screen_title("c l i s t e n", theme),
            Line::from(""),
            Line::from(Span::styled(
                "Terminal UI for NTS Radio",
                Style::default().fg(theme.text),
            )),
            Line::from(Span::styled("Browse live streams, curated picks,", dim)),
            Line::from(Span::styled("and 120+ genres — powered by mpv.", dim)),
            Line::from(""),
            Line::from(""),
            Line::from(""),
            Line::from(Span::styled("Press Enter or → to get started", dim)),
            Line::from(""),
        ]
    }

    fn screen_browse(&self, theme: &Theme) -> Vec<Line<'static>> {
        let key = Style::default()
            .fg(theme.accent)
            .add_modifier(Modifier::BOLD);
        let text = Style::default().fg(theme.text);
        let dim = Style::default().fg(theme.text_dim);

        vec![
            Line::from(""),
            screen_title("Browse NTS Radio", theme),
            Line::from(""),
            Line::from(vec![
                Span::styled(" 1 ", key),
                Span::styled(" Live     ", text),
                Span::styled("Two 24/7 channels", dim),
            ]),
            Line::from(vec![
                Span::styled(" 2 ", key),
                Span::styled(" Picks    ", text),
                Span::styled("Curated episodes", dim),
            ]),
            Line::from(vec![
                Span::styled(" 3 ", key),
                Span::styled(" Search   ", text),
                Span::styled("Browse 120+ genres", dim),
            ]),
            Line::from(""),
            Line::from(Span::styled("Use Tab to switch, j/k to scroll,", dim)),
            Line::from(Span::styled("Enter to play", dim)),
            Line::from(""),
            Line::from(""),
        ]
    }

    fn screen_direct_play(&self, theme: &Theme) -> Vec<Line<'static>> {
        let key = Style::default()
            .fg(theme.accent)
            .add_modifier(Modifier::BOLD);
        let text = Style::default().fg(theme.text);
        let dim = Style::default().fg(theme.text_dim);

        vec![
            Line::from(""),
            screen_title("Play Any URL", theme),
            Line::from(""),
            Line::from(vec![
                Span::styled("Press ", dim),
                Span::styled(" o ", key),
                Span::styled(" to open a URL and play it.", dim),
            ]),
            Line::from(""),
            Line::from(Span::styled("Supports any stream mpv can handle:", dim)),
            Line::from(Span::styled(" · Internet radio stations", text)),
            Line::from(Span::styled(" · Podcast episodes", text)),
            Line::from(Span::styled(" · SoundCloud, Mixcloud, Bandcamp", text)),
            Line::from(Span::styled(" · Direct .mp3 / .m3u8 links", text)),
            Line::from(""),
        ]
    }

    fn screen_theme(&self, theme: &Theme) -> Vec<Line<'static>> {
        let dim = Style::default().fg(theme.text_dim);
        let dark_theme = Theme::dark();
        let light_theme = Theme::light();

        let (dark_marker, light_marker) = if self.selected_theme == 0 {
            ("> ", "  ")
        } else {
            ("  ", "> ")
        };
        let selected = Style::default().fg(theme.text).add_modifier(Modifier::BOLD);
        let dark_style = if self.selected_theme == 0 {
            selected
        } else {
            dim
        };
        let light_style = if self.selected_theme == 1 {
            selected
        } else {
            dim
        };

        vec![
            Line::from(""),
            screen_title("Choose Your Theme", theme),
            Line::from(""),
            Line::from(vec![
                Span::styled(dark_marker, dark_style),
                Span::styled("Dark    ", dark_style),
                Span::styled("████ ", Style::default().fg(dark_theme.primary)),
                Span::styled("████ ", Style::default().fg(dark_theme.secondary)),
                Span::styled("████ ", Style::default().fg(dark_theme.accent)),
                Span::styled("████", Style::default().fg(dark_theme.success)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(light_marker, light_style),
                Span::styled("Light   ", light_style),
                Span::styled("████ ", Style::default().fg(light_theme.primary)),
                Span::styled("████ ", Style::default().fg(light_theme.secondary)),
                Span::styled("████ ", Style::default().fg(light_theme.accent)),
                Span::styled("████", Style::default().fg(light_theme.success)),
            ]),
            Line::from(""),
            Line::from(""),
            Line::from(Span::styled("Use j/k to select, Enter to finish", dim)),
            Line::from(""),
        ]
    }
}
