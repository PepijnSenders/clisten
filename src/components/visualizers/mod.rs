// Visualizer system: trait, kind enum, shared color utilities, and factory.

pub mod blob;
pub mod rings;
pub mod spectrum;
pub mod starfield;
pub mod waveform;

use ratatui::{layout::Rect, style::Color, Frame};
use serde::{Deserialize, Serialize};

/// Common interface for all visualizers.
pub trait Visualizer {
    /// Advance animation state by one frame, given the current playback status.
    fn tick(
        &mut self,
        playing: bool,
        paused: bool,
        buffering: bool,
        position_secs: f64,
        audio_rms: f64,
        audio_peak: f64,
    );
    /// Render the visualizer into the given area.
    fn draw(&self, frame: &mut Frame, area: Rect);
}

/// Identifies which visualizer is active. Persisted in config.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum VisualizerKind {
    #[default]
    Blob,
    Spectrum,
    Waveform,
    Starfield,
    Rings,
}

impl VisualizerKind {
    /// Cycle to the next visualizer variant.
    pub fn next(self) -> Self {
        match self {
            Self::Blob => Self::Spectrum,
            Self::Spectrum => Self::Waveform,
            Self::Waveform => Self::Starfield,
            Self::Starfield => Self::Rings,
            Self::Rings => Self::Blob,
        }
    }

    /// Human-readable label for display.
    pub fn label(self) -> &'static str {
        match self {
            Self::Blob => "Blob",
            Self::Spectrum => "Spectrum",
            Self::Waveform => "Waveform",
            Self::Starfield => "Starfield",
            Self::Rings => "Rings",
        }
    }
}

/// Create a boxed visualizer instance for the given kind.
pub fn create_visualizer(kind: VisualizerKind) -> Box<dyn Visualizer> {
    match kind {
        VisualizerKind::Blob => Box::new(blob::BlobVisualizer::default()),
        VisualizerKind::Spectrum => Box::new(spectrum::SpectrumVisualizer::default()),
        VisualizerKind::Waveform => Box::new(waveform::WaveformVisualizer::default()),
        VisualizerKind::Starfield => Box::new(starfield::StarfieldVisualizer::default()),
        VisualizerKind::Rings => Box::new(rings::RingsVisualizer::default()),
    }
}

/// Linear interpolation between two ratatui colors in RGB space.
pub fn blend_colors(c1: Color, c2: Color, t: f32) -> Color {
    let (r1, g1, b1) = color_to_rgb(c1);
    let (r2, g2, b2) = color_to_rgb(c2);
    Color::Rgb(
        (r1 as f32 * (1.0 - t) + r2 as f32 * t) as u8,
        (g1 as f32 * (1.0 - t) + g2 as f32 * t) as u8,
        (b1 as f32 * (1.0 - t) + b2 as f32 * t) as u8,
    )
}

/// Extract RGB components from a ratatui Color, with fallback for indexed colors.
pub fn color_to_rgb(c: Color) -> (u8, u8, u8) {
    match c {
        Color::Rgb(r, g, b) => (r, g, b),
        Color::Black => (0, 0, 0),
        Color::Red => (205, 49, 49),
        Color::Green => (13, 188, 121),
        Color::Yellow => (229, 229, 16),
        Color::Blue => (36, 114, 200),
        Color::Magenta => (188, 63, 188),
        Color::Cyan => (17, 168, 205),
        Color::White => (229, 229, 229),
        Color::DarkGray => (118, 118, 118),
        Color::LightRed => (241, 76, 76),
        Color::LightGreen => (35, 209, 139),
        Color::LightYellow => (245, 245, 67),
        Color::LightBlue => (59, 142, 234),
        Color::LightMagenta => (214, 112, 214),
        Color::LightCyan => (41, 184, 219),
        _ => (180, 180, 180),
    }
}
