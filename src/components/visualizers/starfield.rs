// Starfield visualizer: warp-speed particle field streaming outward from center.
//
// ~150 particles moving outward from the center point.
// Speed scales with RMS, beat transients spawn bursts.
// Particles brighten as they move outward (depth effect).
// Deterministic pseudo-random (no `rand` crate).

use std::cell::{Cell, RefCell};

use ratatui::{layout::Rect, style::Color, Frame};

use super::blend_colors;
use super::Visualizer;

const NUM_PARTICLES: usize = 150;

struct Particle {
    /// Angle from center (radians).
    angle: f64,
    /// Distance from center (0..1 normalized).
    radius: f64,
    /// Speed multiplier for this particle.
    speed: f64,
}

pub struct StarfieldVisualizer {
    particles: Vec<Particle>,
    phase: f64,
    intensity: f32,
    speed_mult: f64,
    prev_rms: f64,
    grid: RefCell<Vec<Vec<(u8, f64)>>>,
    grid_size: Cell<(usize, usize)>,
}

impl Default for StarfieldVisualizer {
    fn default() -> Self {
        let particles = (0..NUM_PARTICLES)
            .map(|i| {
                let seed = pseudo_rand_seed(i as f64, 0.0);
                Particle {
                    angle: seed * std::f64::consts::TAU,
                    radius: pseudo_rand_seed(i as f64, 1.0),
                    speed: 0.3 + pseudo_rand_seed(i as f64, 2.0) * 0.7,
                }
            })
            .collect();

        Self {
            particles,
            phase: 0.0,
            intensity: 0.0,
            speed_mult: 1.0,
            prev_rms: 0.0,
            grid: RefCell::new(Vec::new()),
            grid_size: Cell::new((0, 0)),
        }
    }
}

/// Deterministic pseudo-random in 0..1 (no rand crate).
fn pseudo_rand_seed(i: f64, offset: f64) -> f64 {
    ((i * 7.31 + offset * 1.17).sin() * 43758.5).fract().abs()
}

impl Visualizer for StarfieldVisualizer {
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

        let transient = if smoothed > 0.01 {
            (audio_peak / smoothed.max(0.01) - 1.0).clamp(0.0, 1.0)
        } else {
            0.0
        };

        // Speed scales with RMS
        let target_speed = if playing && !paused {
            0.5 + smoothed * 3.0 + transient * 2.0
        } else {
            0.1
        };
        self.speed_mult = self.speed_mult * 0.8 + target_speed * 0.2;

        self.phase += 0.01;

        // Move particles outward
        for (i, p) in self.particles.iter_mut().enumerate() {
            p.radius += p.speed * self.speed_mult * 0.008;

            // Respawn particles that leave the field
            if p.radius > 1.0 {
                p.radius = 0.01 + pseudo_rand_seed(i as f64 + self.phase, 3.0) * 0.1;
                p.angle = pseudo_rand_seed(i as f64 + self.phase, 4.0) * std::f64::consts::TAU;
                p.speed = 0.3 + pseudo_rand_seed(i as f64 + self.phase, 5.0) * 0.7;
            }
        }

        // Beat transient: spawn burst (reset some particles to center)
        if transient > 0.5 && !paused {
            let burst_count = (transient * 20.0) as usize;
            for i in 0..burst_count.min(self.particles.len()) {
                let idx = ((i as f64 * 7.31 + self.phase * 13.37).sin() * 43758.5)
                    .fract()
                    .abs();
                let pidx = (idx * self.particles.len() as f64) as usize;
                if pidx < self.particles.len() {
                    self.particles[pidx].radius = 0.0;
                    self.particles[pidx].speed =
                        0.6 + pseudo_rand_seed(pidx as f64 + self.phase, 6.0) * 0.4;
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
        let dot_cols = cols * 2;
        let dot_rows = rows * 4;
        let cx = dot_cols as f64 / 2.0;
        let cy = dot_rows as f64 / 2.0;
        let max_r = cx.min(cy);

        // Reuse cached grid, reallocate only on size change
        let mut grid = self.grid.borrow_mut();
        if self.grid_size.get() != (cols, rows) {
            grid.clear();
            grid.resize_with(rows, || vec![(0u8, 0.0); cols]);
            self.grid_size.set((cols, rows));
        } else {
            for row in grid.iter_mut() {
                for cell in row.iter_mut() {
                    *cell = (0u8, 0.0);
                }
            }
        }

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

        for p in &self.particles {
            let px = cx + p.angle.cos() * p.radius * max_r;
            let py = cy + p.angle.sin() * p.radius * max_r;

            let dot_x = px as usize;
            let dot_y = py as usize;

            if dot_x >= dot_cols || dot_y >= dot_rows {
                continue;
            }

            let cell_col = dot_x / 2;
            let cell_row = dot_y / 4;
            let local_x = dot_x % 2;
            let local_y = dot_y % 4;

            if cell_col >= cols || cell_row >= rows {
                continue;
            }

            // Find the bit index for this (local_x, local_y)
            for (i, &(ox, oy)) in dot_offsets.iter().enumerate() {
                if ox == local_x && oy == local_y {
                    grid[cell_row][cell_col].0 |= dot_bits[i];
                    // Track max radius for brightness
                    if p.radius > grid[cell_row][cell_col].1 {
                        grid[cell_row][cell_col].1 = p.radius;
                    }
                    break;
                }
            }
        }

        let buf = frame.buffer_mut();

        for (row, grid_row) in grid.iter().enumerate() {
            for (col, &(dots, max_radius)) in grid_row.iter().enumerate() {
                if dots == 0 {
                    continue;
                }

                let ch = char::from_u32(0x2800 + dots as u32).unwrap_or(' ');
                // Brightness increases with distance from center
                let brightness = max_radius.clamp(0.0, 1.0);
                let color = blend_colors(
                    Color::Blue,
                    Color::White,
                    (brightness * self.intensity as f64) as f32,
                );

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
