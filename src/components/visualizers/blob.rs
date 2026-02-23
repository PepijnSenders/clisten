// Braille-dot blob visualizer for the now-playing panel.
//
// Renders an animated, color-cycling blob using Unicode braille characters.
// Each terminal cell maps to a 2x4 braille dot grid, giving sub-character
// resolution. The blob shape is a sum of sinusoids at different frequencies,
// producing an organic, lava-lamp-like motion.

use ratatui::{layout::Rect, style::Color, Frame};

use super::{blend_colors, Visualizer};

/// Three color palettes the visualizer cycles through over time.
const PALETTES: &[&[Color]; 3] = &[
    &[
        Color::Cyan,
        Color::Blue,
        Color::Magenta,
        Color::LightMagenta,
    ],
    &[Color::Yellow, Color::Red, Color::Magenta, Color::LightRed],
    &[Color::Green, Color::Cyan, Color::White, Color::LightGreen],
];

/// Animated blob state. Call `tick()` each frame, then `draw()` to render.
#[derive(Default)]
pub struct BlobVisualizer {
    phase: f64,
    color_phase: f64,
    intensity: f32,
    beat: f64,
    prev_position: f64,
    prev_rms: f64,
}

impl Visualizer for BlobVisualizer {
    fn tick(
        &mut self,
        playing: bool,
        paused: bool,
        buffering: bool,
        position_secs: f64,
        audio_rms: f64,
        audio_peak: f64,
    ) {
        let has_audio_levels = audio_rms > 0.0 || self.prev_rms > 0.0;

        if playing && !paused {
            if buffering {
                self.phase += 0.04;
                self.color_phase += 0.003;
                self.beat = 0.0;
            } else if has_audio_levels {
                let smoothed = self.prev_rms * 0.3 + audio_rms * 0.7;
                self.prev_rms = smoothed;
                self.beat = smoothed * smoothed;
                let transient = if smoothed > 0.01 {
                    (audio_peak / smoothed.max(0.01) - 1.0).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                self.beat = (self.beat + transient * 0.5).clamp(0.0, 1.0);
                self.phase += 0.06 + 0.1 * smoothed + 0.05 * transient;
                self.color_phase += 0.003 + 0.002 * smoothed;
            } else {
                let pos_delta = position_secs - self.prev_position;
                if pos_delta > 0.0 {
                    let raw = (self.phase * 2.5).sin() * 0.5 + 0.5;
                    self.beat = raw * raw * raw * raw;
                } else {
                    self.beat *= 0.9;
                }
                self.phase += 0.08 + 0.04 * self.beat;
                self.color_phase += 0.003;
            }
        } else {
            self.beat *= 0.85;
            self.prev_rms *= 0.85;
        }
        self.prev_position = position_secs;

        let target = if !playing || paused {
            0.0
        } else if buffering {
            0.3
        } else {
            1.0
        };
        self.intensity += (target - self.intensity) * 0.15;
    }

    fn draw(&self, frame: &mut Frame, area: Rect) {
        if area.width == 0 || area.height == 0 || self.intensity < 0.01 {
            return;
        }

        let beat_mod = 0.7 + 0.3 * self.beat;
        let (base, amps) = self.blob_params(beat_mod);

        let eff_base = base * self.intensity as f64;
        let eff_amps = [
            amps[0] * self.intensity as f64,
            amps[1] * self.intensity as f64,
            amps[2] * self.intensity as f64,
            amps[3] * self.intensity as f64,
        ];

        let cols = area.width as usize;
        let rows = area.height as usize;
        let dot_cols = cols * 2;
        let dot_rows = rows * 4;
        let cx = dot_cols as f64 / 2.0;
        let cy = dot_rows as f64 / 2.0;
        let scale = cx.min(cy).max(1.0);

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

        for row in 0..rows {
            for col in 0..cols {
                let mut dots: u8 = 0;
                let mut best_dr: f64 = 1.0;
                let mut any_inside = false;

                for (i, &(dx, dy)) in dot_offsets.iter().enumerate() {
                    let px = (col * 2 + dx) as f64;
                    let py = (row * 4 + dy) as f64;
                    let rel_x = (px - cx) / scale;
                    let rel_y = (py - cy) / scale;
                    let dist = (rel_x * rel_x + rel_y * rel_y).sqrt();
                    let angle = rel_y.atan2(rel_x);
                    let r = self.radius(angle, eff_base, &eff_amps);

                    if dist < r {
                        dots |= dot_bits[i];
                        let dr = dist / r.max(0.001);
                        if dr < best_dr {
                            best_dr = dr;
                        }
                        any_inside = true;
                    }
                }

                if any_inside {
                    let ch = char::from_u32(0x2800 + dots as u32).unwrap_or(' ');
                    let color = self.color_at(best_dr);
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

impl BlobVisualizer {
    fn blob_params(&self, beat_mod: f64) -> (f64, [f64; 4]) {
        if self.intensity < 0.01 {
            return (0.2, [0.02, 0.01, 0.01, 0.01]);
        }

        if self.intensity < 0.35 {
            let pulse = 0.4 + 0.1 * (self.phase * 1.5).sin();
            return (pulse, [0.04, 0.03, 0.02, 0.01]);
        }

        if self.intensity < 0.65 {
            return (0.4, [0.03, 0.04, 0.03, 0.05]);
        }

        let breathing = 0.95 + 0.05 * (self.phase * 0.5).sin();
        let base = 0.6 * breathing;
        (
            base,
            [
                0.10 * beat_mod,
                0.12 * beat_mod,
                0.08 * beat_mod,
                0.15 * beat_mod,
            ],
        )
    }

    fn radius(&self, theta: f64, base: f64, amps: &[f64; 4]) -> f64 {
        let t = self.phase;
        base + amps[0] * (2.0 * theta + t).sin()
            + amps[1] * (3.0 * theta + t * 1.5).sin()
            + amps[2] * (5.0 * theta + t * 0.7).cos()
            + amps[3] * (7.0 * theta + t * 2.0).sin()
    }

    fn color_at(&self, dr: f64) -> Color {
        let palette_f = self.color_phase % (PALETTES.len() as f64);
        let idx = palette_f as usize % PALETTES.len();
        let next = (idx + 1) % PALETTES.len();
        let blend = palette_f.fract() as f32;

        let zone = if dr < 0.4 {
            0
        } else if dr < 0.7 {
            1
        } else if dr < 0.9 {
            2
        } else {
            3
        };

        blend_colors(PALETTES[idx][zone], PALETTES[next][zone], blend)
    }
}
