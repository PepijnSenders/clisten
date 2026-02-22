// Rings visualizer: concentric expanding ripples from center.
//
// 4-6 active rings expanding outward from center.
// Drawn as braille dot circles at various radii.
// Ring thickness thins as radius grows.
// Beat transients spawn new rings; expansion speed scales with RMS.

use ratatui::{layout::Rect, style::Color, Frame};

use super::{blend_colors, Visualizer};

const MAX_RINGS: usize = 8;

const RING_COLORS: &[Color] = &[
    Color::Cyan,
    Color::Magenta,
    Color::LightBlue,
    Color::LightMagenta,
    Color::LightCyan,
    Color::Blue,
];

struct Ring {
    radius: f64,
    /// 1.0 when spawned, fades toward 0.
    alpha: f64,
    color_idx: usize,
}

pub struct RingsVisualizer {
    rings: Vec<Ring>,
    phase: f64,
    intensity: f32,
    speed: f64,
    prev_rms: f64,
    spawn_timer: f64,
    ring_counter: usize,
}

impl Default for RingsVisualizer {
    fn default() -> Self {
        Self {
            rings: Vec::new(),
            phase: 0.0,
            intensity: 0.0,
            speed: 1.0,
            prev_rms: 0.0,
            spawn_timer: 0.0,
            ring_counter: 0,
        }
    }
}

impl RingsVisualizer {
    fn spawn_ring(&mut self) {
        if self.rings.len() >= MAX_RINGS {
            // Replace the oldest (largest radius) ring
            if let Some(oldest_idx) = self
                .rings
                .iter()
                .enumerate()
                .max_by(|a, b| {
                    a.1.radius
                        .partial_cmp(&b.1.radius)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(i, _)| i)
            {
                self.rings.remove(oldest_idx);
            }
        }
        self.rings.push(Ring {
            radius: 0.02,
            alpha: 1.0,
            color_idx: self.ring_counter % RING_COLORS.len(),
        });
        self.ring_counter += 1;
    }
}

impl Visualizer for RingsVisualizer {
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
            0.5
        } else {
            1.0
        };
        self.intensity += (target_intensity - self.intensity) * 0.05;

        let smoothed = self.prev_rms * 0.3 + audio_rms * 0.7;
        self.prev_rms = smoothed;

        let transient = if smoothed > 0.01 {
            (audio_peak / smoothed.max(0.01) - 1.0).clamp(0.0, 1.0)
        } else {
            0.0
        };

        // Expansion speed scales with RMS
        self.speed = 0.3 + smoothed * 2.0;

        self.phase += 0.02;

        // Expand rings
        for ring in &mut self.rings {
            ring.radius += self.speed * 0.012;
            ring.alpha = (1.0 - ring.radius).max(0.0);
        }

        // Remove dead rings
        self.rings.retain(|r| r.alpha > 0.01);

        // Auto-spawn rings periodically
        self.spawn_timer += 0.03 + smoothed * 0.05;
        if self.spawn_timer > 1.0 && playing {
            self.spawn_timer = 0.0;
            self.spawn_ring();
        }

        // Beat transient spawns extra ring
        if transient > 0.4 && playing {
            self.spawn_ring();
        }
    }

    fn draw(&self, frame: &mut Frame, area: Rect) {
        if area.width < 2 || area.height < 2 || self.intensity < 0.01 {
            return;
        }

        let cols = area.width as usize;
        let rows = area.height as usize;
        let dot_cols = cols * 2;
        let dot_rows = rows * 4;
        let cx = dot_cols as f64 / 2.0;
        let cy = dot_rows as f64 / 2.0;
        let max_r = cx.min(cy);

        let mut grid: Vec<Vec<(u8, Option<Color>)>> = vec![vec![(0u8, None); cols]; rows];

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

        for ring in &self.rings {
            let r_pixels = ring.radius * max_r;
            // Ring thickness thins as it grows
            let thickness = (2.0 - ring.radius * 1.5).max(0.5);

            let color = blend_colors(
                RING_COLORS[ring.color_idx],
                Color::Black,
                1.0 - (ring.alpha * self.intensity as f64) as f32,
            );

            // Scan all cells and check if any braille dot is near the ring circle
            for (row, grid_row) in grid.iter_mut().enumerate() {
                for (col, cell) in grid_row.iter_mut().enumerate() {
                    for (i, &(dx, dy)) in dot_offsets.iter().enumerate() {
                        let dot_x = (col * 2 + dx) as f64;
                        let dot_y = (row * 4 + dy) as f64;
                        let dist_x = dot_x - cx;
                        let dist_y = dot_y - cy;
                        let dist = (dist_x * dist_x + dist_y * dist_y).sqrt();

                        if (dist - r_pixels).abs() < thickness {
                            cell.0 |= dot_bits[i];
                            cell.1 = Some(color);
                        }
                    }
                }
            }
        }

        let buf = frame.buffer_mut();

        for (row, grid_row) in grid.iter().enumerate() {
            for (col, (dots, color_opt)) in grid_row.iter().enumerate() {
                if *dots == 0 {
                    continue;
                }

                let ch = char::from_u32(0x2800 + *dots as u32).unwrap_or(' ');
                let color = color_opt.unwrap_or(Color::DarkGray);

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
