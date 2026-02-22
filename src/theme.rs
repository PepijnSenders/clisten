// Color theme definitions for the TUI chrome. Visualizer palettes are
// self-contained and unaffected by theme choice.

use ratatui::style::Color;

pub const THEME_DARK: &str = "dark";
pub const THEME_LIGHT: &str = "light";

/// Named color slots used by the UI chrome (borders, text, status indicators).
#[derive(Debug, Clone)]
pub struct Theme {
    /// Primary accent color (active tab, active items, highlights).
    pub primary: Color,
    /// Secondary accent / subtitle color.
    pub secondary: Color,
    /// Normal text color.
    pub text: Color,
    /// Dimmed / inactive text color.
    pub text_dim: Color,
    /// Accent color for key hints, secondary highlights.
    pub accent: Color,
    /// Background color for selected items.
    pub selection_bg: Color,
    /// Border and divider color.
    pub border: Color,
    /// Error indicators.
    pub error: Color,
    /// Warning / loading indicators.
    pub warning: Color,
    /// Success / playing indicators.
    pub success: Color,
    /// Buffering indicator color.
    pub buffering: Color,
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            primary: Color::Cyan,
            secondary: Color::Magenta,
            text: Color::White,
            text_dim: Color::DarkGray,
            accent: Color::Yellow,
            selection_bg: Color::Rgb(30, 30, 40),
            border: Color::DarkGray,
            error: Color::Red,
            warning: Color::Yellow,
            success: Color::Green,
            buffering: Color::Yellow,
        }
    }

    pub fn light() -> Self {
        Self {
            primary: Color::Blue,
            secondary: Color::Magenta,
            text: Color::Black,
            text_dim: Color::DarkGray,
            accent: Color::Rgb(180, 120, 0),
            selection_bg: Color::Rgb(220, 225, 235),
            border: Color::Rgb(180, 180, 180),
            error: Color::Red,
            warning: Color::Rgb(180, 120, 0),
            success: Color::Rgb(0, 140, 60),
            buffering: Color::Rgb(180, 120, 0),
        }
    }

    pub fn from_name(name: &str) -> Self {
        match name {
            THEME_LIGHT => Self::light(),
            _ => Self::dark(),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}
