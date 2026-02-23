// Spectrum visualizer: classic EQ-style vertical bars with peak-hold dots.
//
// 12 bars across the width, heights derived from pseudo-frequency-band
// decomposition of the RMS signal. Filled bottom-up with braille dots.
// Gradient: green (bottom) -> yellow (mid) -> magenta (top).
// Bars decay smoothly; beat transients cause jumps.

use ratatui::{layout::Rect, style::Color, Frame};

use super::{blend_colors, Visualizer};

const NUM_BARS: usize = 12;

#[derive(Default)]
pub struct SpectrumVisualizer {
    phase: f64,
    bar_heights: [f64; NUM_BARS],
    peak_heights: [f64; NUM_BARS],
    peak_decay: [f64; NUM_BARS],
    intensity: f32,
    prev_rms: f64,
}

impl Visualizer for SpectrumVisualizer {
    fn tick(
        &mut self,
        playing: bool,
        paused: bool,
        buffering: bool,
        _position_secs: f64,
        audio_rms: f64,
        audio_peak: f64,
    ) {
        let target_intensity = if !playing {
            0.0
        } else if buffering {
            0.3
        } else if paused {
            0.6
        } else {
            1.0
        };
        self.intensity += (target_intensity - self.intensity) * 0.05;

        self.phase += 0.05;

        let smoothed = self.prev_rms * 0.3 + audio_rms * 0.7;
        self.prev_rms = smoothed;

        let transient = if smoothed > 0.01 {
            (audio_peak / smoothed.max(0.01) - 1.0).clamp(0.0, 0.8)
        } else {
            0.0
        };

        for i in 0..NUM_BARS {
            // Pseudo-frequency distribution: each bar gets a different sine
            // combination to simulate frequency bands
            let freq = 1.0 + i as f64 * 0.7;
            let band_energy =
                smoothed * (0.6 + 0.4 * ((self.phase * freq + i as f64 * 0.9).sin() * 0.5 + 0.5));

            let target = (band_energy + transient * 0.3) * self.intensity as f64;
            // Bars jump up fast but decay slowly
            if target > self.bar_heights[i] {
                self.bar_heights[i] = self.bar_heights[i] * 0.3 + target * 0.7;
            } else {
                self.bar_heights[i] = self.bar_heights[i] * 0.92 + target * 0.08;
            }

            // Peak hold: rises instantly, holds briefly, then decays
            if self.bar_heights[i] > self.peak_heights[i] {
                self.peak_heights[i] = self.bar_heights[i];
                self.peak_decay[i] = 0.0;
            } else {
                self.peak_decay[i] += 0.02;
                self.peak_heights[i] -= self.peak_decay[i] * 0.01;
                if self.peak_heights[i] < self.bar_heights[i] {
                    self.peak_heights[i] = self.bar_heights[i];
                }
            }
        }
    }

    fn draw(&self, frame: &mut Frame, area: Rect) {
        if area.width < 2 || area.height < 2 || self.intensity < 0.01 {
            return;
        }

        let cols = area.width as usize;
        let rows = area.height as usize;
        let dot_rows = rows * 4; // braille sub-rows

        let bar_width = (cols / NUM_BARS).max(1);
        let gap = if bar_width > 1 { 1 } else { 0 };
        let filled_width = bar_width.saturating_sub(gap);

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

        for bar_idx in 0..NUM_BARS {
            let bar_x_start = bar_idx * bar_width;
            if bar_x_start >= cols {
                break;
            }

            let bar_h = (self.bar_heights[bar_idx] * dot_rows as f64).min(dot_rows as f64);
            let peak_h = (self.peak_heights[bar_idx] * dot_rows as f64).min(dot_rows as f64);

            for row in 0..rows {
                for col_offset in 0..filled_width.min(2) {
                    let col = bar_x_start + col_offset;
                    if col >= cols {
                        break;
                    }

                    let mut dots: u8 = 0;
                    let mut has_dot = false;
                    let mut max_frac: f64 = 0.0;
                    let mut is_peak = false;

                    for (i, &(dx, dy)) in dot_offsets.iter().enumerate() {
                        if dx != col_offset % 2 && filled_width > 1 {
                            continue;
                        }
                        if dx != 0 && filled_width <= 1 {
                            continue;
                        }

                        let dot_y = row * 4 + dy;
                        let y_from_bottom = dot_rows.saturating_sub(1 + dot_y);

                        if (y_from_bottom as f64) < bar_h {
                            dots |= dot_bits[i];
                            has_dot = true;
                            let frac = y_from_bottom as f64 / dot_rows as f64;
                            if frac > max_frac {
                                max_frac = frac;
                            }
                        } else if peak_h > 0.0 && ((y_from_bottom as f64) - peak_h).abs() < 1.5 {
                            dots |= dot_bits[i];
                            has_dot = true;
                            is_peak = true;
                        }
                    }

                    if has_dot {
                        let ch = char::from_u32(0x2800 + dots as u32).unwrap_or(' ');
                        let color = if is_peak && max_frac < 0.01 {
                            Color::White
                        } else {
                            // Gradient: green -> yellow -> magenta
                            if max_frac < 0.5 {
                                blend_colors(Color::Green, Color::Yellow, (max_frac * 2.0) as f32)
                            } else {
                                blend_colors(
                                    Color::Yellow,
                                    Color::Magenta,
                                    ((max_frac - 0.5) * 2.0) as f32,
                                )
                            }
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
}
