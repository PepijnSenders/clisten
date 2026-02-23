// Waveform visualizer: oscilloscope-style scrolling sine wave.
//
// Continuous sine wave plotted with braille dots across the full width.
// Amplitude scales with RMS, frequency modulated by peak.
// Faint "echo" trail wave in dimmer color.
// Horizontal color gradient cycling through palettes.

use ratatui::{layout::Rect, style::Color, Frame};

use super::{blend_colors, Visualizer};

const TRAIL_COLORS: &[Color] = &[Color::Cyan, Color::Blue, Color::Magenta];
const MAIN_COLORS: &[Color] = &[Color::LightCyan, Color::LightMagenta, Color::LightGreen];

#[derive(Default)]
pub struct WaveformVisualizer {
    phase: f64,
    color_phase: f64,
    intensity: f32,
    amplitude: f64,
    frequency: f64,
    prev_rms: f64,
}

impl Visualizer for WaveformVisualizer {
    fn tick(
        &mut self,
        playing: bool,
        paused: bool,
        buffering: bool,
        _position_secs: f64,
        audio_rms: f64,
        audio_peak: f64,
    ) {
        let target_intensity = if !playing || paused {
            0.0
        } else if buffering {
            0.3
        } else {
            1.0
        };
        self.intensity += (target_intensity - self.intensity) * 0.15;

        let smoothed = self.prev_rms * 0.3 + audio_rms * 0.7;
        self.prev_rms = smoothed;

        // Amplitude tracks RMS
        let target_amp = 0.2 + smoothed * 0.8;
        self.amplitude = self.amplitude * 0.85 + target_amp * 0.15;

        // Frequency modulated by peak
        let target_freq = 2.0 + audio_peak * 4.0;
        self.frequency = self.frequency * 0.9 + target_freq * 0.1;

        // Scroll phase
        if playing && !paused {
            self.phase += 0.08 + smoothed * 0.12;
        }
        self.color_phase += 0.005;
    }

    fn draw(&self, frame: &mut Frame, area: Rect) {
        if area.width < 2 || area.height < 2 || self.intensity < 0.01 {
            return;
        }

        let cols = area.width as usize;
        let rows = area.height as usize;
        let dot_rows = rows * 4;
        let center_y = dot_rows as f64 / 2.0;

        let buf = frame.buffer_mut();

        let dot_bits: [u8; 8] = [0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80];
        let dot_offsets: [(usize, usize); 8] = [
            (0, 0),
            (0, 1),
            (0, 2),
            (1, 0),
            (1, 1),
            (1, 2),
            (0, 3),
            (1, 3),
        ];

        // Precompute wave values for each dot column
        let dot_cols = cols * 2;
        let amp = self.amplitude * self.intensity as f64 * center_y * 0.8;

        // Two waves: main and trail (echo)
        let main_wave: Vec<f64> = (0..dot_cols)
            .map(|x| {
                let t = x as f64 / dot_cols as f64;
                let angle = t * std::f64::consts::TAU * self.frequency + self.phase;
                center_y + amp * angle.sin()
            })
            .collect();

        let trail_wave: Vec<f64> = (0..dot_cols)
            .map(|x| {
                let t = x as f64 / dot_cols as f64;
                let angle = t * std::f64::consts::TAU * self.frequency + self.phase - 0.4;
                center_y + amp * 0.7 * angle.sin()
            })
            .collect();

        // Color gradient along horizontal axis
        let main_palette_idx = (self.color_phase as usize) % MAIN_COLORS.len();
        let main_palette_next = (main_palette_idx + 1) % MAIN_COLORS.len();
        let trail_palette_idx = (self.color_phase as usize) % TRAIL_COLORS.len();
        let trail_palette_next = (trail_palette_idx + 1) % TRAIL_COLORS.len();
        let palette_blend = self.color_phase.fract() as f32;

        for row in 0..rows {
            for col in 0..cols {
                let mut dots: u8 = 0;
                let mut has_main = false;
                let mut has_trail = false;

                for (i, &(dx, dy)) in dot_offsets.iter().enumerate() {
                    let dot_x = col * 2 + dx;
                    let dot_y = row * 4 + dy;

                    if dot_x < dot_cols {
                        // Check main wave (thicker line: +/- 1 dot)
                        let main_y = main_wave[dot_x];
                        if (dot_y as f64 - main_y).abs() < 1.5 {
                            dots |= dot_bits[i];
                            has_main = true;
                        }
                        // Check trail wave
                        let trail_y = trail_wave[dot_x];
                        if (dot_y as f64 - trail_y).abs() < 1.0 {
                            dots |= dot_bits[i];
                            has_trail = true;
                        }
                    }
                }

                if dots != 0 {
                    let ch = char::from_u32(0x2800 + dots as u32).unwrap_or(' ');
                    // Horizontal gradient
                    let h_frac = col as f32 / cols.max(1) as f32;
                    let color = if has_main {
                        let base = blend_colors(
                            MAIN_COLORS[main_palette_idx],
                            MAIN_COLORS[main_palette_next],
                            palette_blend,
                        );
                        let end = blend_colors(
                            MAIN_COLORS[(main_palette_idx + 1) % MAIN_COLORS.len()],
                            MAIN_COLORS[(main_palette_next + 1) % MAIN_COLORS.len()],
                            palette_blend,
                        );
                        blend_colors(base, end, h_frac)
                    } else if has_trail {
                        let base = blend_colors(
                            TRAIL_COLORS[trail_palette_idx],
                            TRAIL_COLORS[trail_palette_next],
                            palette_blend,
                        );
                        let end = blend_colors(
                            TRAIL_COLORS[(trail_palette_idx + 1) % TRAIL_COLORS.len()],
                            TRAIL_COLORS[(trail_palette_next + 1) % TRAIL_COLORS.len()],
                            palette_blend,
                        );
                        blend_colors(base, end, h_frac)
                    } else {
                        Color::DarkGray
                    };

                    let x = area.x + col as u16;
                    let y = area.y + row as u16;
                    if let Some(cell) = buf.cell_mut((x, y)) {
                        cell.set_char(ch);
                        cell.set_fg(color);
                    }
                }
            }
        }
    }
}
