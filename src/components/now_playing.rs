// src/components/now_playing.rs

use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
    Frame,
};
use tokio::sync::mpsc::UnboundedSender;

use crate::action::Action;
use crate::api::models::DiscoveryItem;
use crate::components::Component;

/// Color palette for the blob — cycles through these moods
const PALETTES: &[&[Color]; 3] = &[
    // Cool: Cyan → Blue → Magenta
    &[Color::Cyan, Color::Blue, Color::Magenta, Color::LightMagenta],
    // Warm: Yellow → Red → Magenta
    &[Color::Yellow, Color::Red, Color::Magenta, Color::LightRed],
    // Electric: Green → Cyan → White
    &[Color::Green, Color::Cyan, Color::White, Color::LightGreen],
];

pub struct NowPlaying {
    action_tx: Option<UnboundedSender<Action>>,
    pub current_item: Option<DiscoveryItem>,
    pub position_secs: f64,
    pub paused: bool,
    pub buffering: bool,
    pub stream_metadata: Option<String>,
    queue_items: Vec<(String, String)>,
    queue_current: Option<usize>,
    // Animation state
    frame_count: u64,
    blob_phase: f64,
    color_phase: f64,
    intensity: f32,
    prev_position: f64,
    beat: f64,
}

impl NowPlaying {
    pub fn new() -> Self {
        Self {
            action_tx: None,
            current_item: None,
            position_secs: 0.0,
            paused: false,
            buffering: false,
            stream_metadata: None,
            queue_items: Vec::new(),
            queue_current: None,
            frame_count: 0,
            blob_phase: 0.0,
            color_phase: 0.0,
            intensity: 0.0,
            prev_position: 0.0,
            beat: 0.0,
        }
    }

    pub fn set_queue(&mut self, items: Vec<(String, String)>, current_index: Option<usize>) {
        self.queue_items = items;
        self.queue_current = current_index;
    }

    /// Compute the blob radius at a given angle for the current phase.
    fn blob_radius(&self, theta: f64, base: f64, amps: &[f64; 4]) -> f64 {
        let t = self.blob_phase;
        base + amps[0] * (2.0 * theta + t * 1.0).sin()
            + amps[1] * (3.0 * theta + t * 1.5).sin()
            + amps[2] * (5.0 * theta + t * 0.7).cos()
            + amps[3] * (7.0 * theta + t * 2.0).sin()
    }

    /// Pick a color for a point at distance ratio `dr` (0.0 = center, 1.0 = edge).
    fn blob_color(&self, dr: f64) -> Color {
        let palette_f = self.color_phase % (PALETTES.len() as f64);
        let pal_idx = palette_f as usize % PALETTES.len();
        let pal_next = (pal_idx + 1) % PALETTES.len();
        let blend = palette_f.fract() as f32;

        let zone = if dr < 0.4 { 0 } else if dr < 0.7 { 1 } else if dr < 0.9 { 2 } else { 3 };

        let c1 = PALETTES[pal_idx][zone];
        let c2 = PALETTES[pal_next][zone];

        blend_colors(c1, c2, blend)
    }

    /// Draw the braille blob visualizer into the given area.
    fn draw_blob(&self, frame: &mut Frame, area: Rect) {
        if area.width == 0 || area.height == 0 || self.intensity < 0.01 {
            return;
        }

        // Determine blob parameters based on playback state
        let beat_mod = 0.7 + 0.3 * self.beat;
        let (base, amps) = if self.current_item.is_some() {
            if self.buffering {
                // Gentle throb
                let pulse = 0.4 + 0.1 * (self.blob_phase * 1.5).sin();
                (pulse, [0.04, 0.03, 0.02, 0.01])
            } else if self.paused {
                // Frozen shape (phase doesn't advance)
                (0.4, [0.03, 0.04, 0.03, 0.05])
            } else {
                // Active with beat modulation + size breathing
                let breathing = 0.95 + 0.05 * (self.blob_phase * 0.5).sin();
                let base_r = 0.6 * breathing;
                (base_r, [
                    0.10 * beat_mod,
                    0.12 * beat_mod,
                    0.08 * beat_mod,
                    0.15 * beat_mod,
                ])
            }
        } else {
            // Stopped — small residual
            (0.2, [0.02, 0.01, 0.01, 0.01])
        };

        // Scale by intensity for smooth transitions
        let effective_base = base * self.intensity as f64;
        let effective_amps = [
            amps[0] * self.intensity as f64,
            amps[1] * self.intensity as f64,
            amps[2] * self.intensity as f64,
            amps[3] * self.intensity as f64,
        ];

        let buf = frame.buffer_mut();

        // Each terminal cell = 2 dots wide, 4 dots tall in braille
        let cols = area.width as usize;
        let rows = area.height as usize;
        let dot_cols = cols * 2;
        let dot_rows = rows * 4;

        // Center of the dot grid
        let cx = dot_cols as f64 / 2.0;
        let cy = dot_rows as f64 / 2.0;

        // Scale: normalize so the blob fits in the available space
        let scale = (cx.min(cy)).max(1.0);

        for row in 0..rows {
            for col in 0..cols {
                let mut dots: u8 = 0;
                let mut best_dr: f64 = 1.0; // track closest distance ratio for coloring
                let mut any_inside = false;

                // Braille dot layout within a cell:
                // dot 0 (0,0)  dot 3 (1,0)
                // dot 1 (0,1)  dot 4 (1,1)
                // dot 2 (0,2)  dot 5 (1,2)
                // dot 6 (0,3)  dot 7 (1,3)
                let dot_bits: [u8; 8] = [0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80];
                let dot_offsets: [(usize, usize); 8] = [
                    (0, 0), (0, 1), (0, 2),
                    (1, 0), (1, 1), (1, 2),
                    (0, 3), (1, 3),
                ];

                for (bit_idx, &(dx, dy)) in dot_offsets.iter().enumerate() {
                    let px = (col * 2 + dx) as f64;
                    let py = (row * 4 + dy) as f64;

                    let rel_x = (px - cx) / scale;
                    let rel_y = (py - cy) / scale;

                    let dist = (rel_x * rel_x + rel_y * rel_y).sqrt();
                    let angle = rel_y.atan2(rel_x);

                    let r = self.blob_radius(angle, effective_base, &effective_amps);

                    if dist < r {
                        dots |= dot_bits[bit_idx];
                        let dr = dist / r.max(0.001);
                        if dr < best_dr {
                            best_dr = dr;
                        }
                        any_inside = true;
                    }
                }

                if any_inside {
                    let ch = char::from_u32(0x2800 + dots as u32).unwrap_or(' ');
                    let color = self.blob_color(best_dr);
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

impl Component for NowPlaying {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) {
        self.action_tx = Some(tx);
    }

    fn update(&mut self, action: &Action) -> anyhow::Result<Vec<Action>> {
        match action {
            Action::Tick => {
                self.frame_count = self.frame_count.wrapping_add(1);

                if self.current_item.is_some() {
                    if self.buffering {
                        // Gentle throb
                        self.blob_phase += 0.04;
                        self.color_phase += 0.003;
                        self.beat = 0.0;
                    } else if self.paused {
                        // Freeze: don't advance blob_phase or color_phase
                        self.beat = 0.0;
                    } else {
                        // Playing: pseudo-beat reactivity
                        let pos_delta = self.position_secs - self.prev_position;
                        if pos_delta > 0.0 {
                            // Audio is advancing — compute beat pulse
                            let raw_beat = (self.blob_phase * 2.5).sin() * 0.5 + 0.5;
                            self.beat = raw_beat * raw_beat * raw_beat * raw_beat; // ^4 for sharp peaks
                        } else {
                            self.beat *= 0.9; // decay if stalled
                        }
                        self.blob_phase += 0.08 + 0.04 * self.beat;
                        self.color_phase += 0.003;
                    }
                } else {
                    self.blob_phase += 0.01;
                    self.color_phase += 0.003;
                    self.beat = 0.0;
                }
                self.prev_position = self.position_secs;

                // Smoothly interpolate intensity
                let target = if self.current_item.is_some() {
                    if self.buffering {
                        0.3
                    } else if self.paused {
                        0.6
                    } else {
                        1.0
                    }
                } else {
                    0.0
                };
                self.intensity += (target - self.intensity) * 0.05;
            }
            Action::PlayItem(item) => {
                self.current_item = Some(item.clone());
                self.position_secs = 0.0;
                self.paused = false;
                self.buffering = true;
                self.stream_metadata = None;
            }
            Action::PlaybackLoading => {
                self.buffering = true;
                self.stream_metadata = None;
            }
            Action::PlaybackPosition(pos) => {
                self.position_secs = *pos;
                self.buffering = false;
            }
            Action::StreamMetadataChanged(title) => {
                self.stream_metadata = Some(title.clone());
            }
            Action::TogglePlayPause => {
                self.paused = !self.paused;
            }
            Action::Stop | Action::PlaybackFinished => {
                self.current_item = None;
                self.position_secs = 0.0;
                self.buffering = false;
                self.stream_metadata = None;
            }
            _ => {}
        }
        Ok(vec![])
    }

    fn draw(&self, frame: &mut Frame, area: Rect) {
        // Split right panel: top for now playing, bottom for queue
        let has_queue = !self.queue_items.is_empty();
        let chunks = if has_queue {
            Layout::vertical([
                Constraint::Min(7),
                Constraint::Percentage(50),
            ]).split(area)
        } else {
            Layout::vertical([
                Constraint::Min(0),
            ]).split(area)
        };

        // ── Now Playing section ──
        let title_style = if self.current_item.is_some() && !self.paused {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let np_area = chunks[0];
        // Render title line at top
        let title_line = Line::from(Span::styled(" Now Playing", title_style));
        let title_area = Rect { x: np_area.x, y: np_area.y, width: np_area.width, height: 1 };
        frame.render_widget(Paragraph::new(title_line), title_area);

        let np_inner = Rect {
            x: np_area.x + 1,
            y: np_area.y + 1,
            width: np_area.width.saturating_sub(2),
            height: np_area.height.saturating_sub(1),
        };

        let Some(item) = &self.current_item else {
            let empty = Paragraph::new("Nothing playing")
                .style(Style::default().fg(Color::DarkGray));
            frame.render_widget(empty, np_inner);
            if has_queue {
                self.draw_queue(frame, chunks[1]);
            }
            return;
        };

        // Split inner area: track info (4 lines), visualizer (fill), tags (2 lines)
        let has_tags = matches!(
            item,
            DiscoveryItem::NtsEpisode { genres, .. }
            | DiscoveryItem::NtsLiveChannel { genres, .. }
            if !genres.is_empty()
        );
        let has_url = matches!(item, DiscoveryItem::DirectUrl { .. });
        let tag_height = if has_tags || has_url { 2 } else { 0 };

        let inner_chunks = Layout::vertical([
            Constraint::Length(4),  // track info
            Constraint::Min(4),    // visualizer
            Constraint::Length(tag_height),  // tags
        ]).split(np_inner);

        // ── Track info ──
        let mins = self.position_secs as u64 / 60;
        let secs = self.position_secs as u64 % 60;
        let status = if self.buffering {
            "⟳ Loading..."
        } else if self.paused {
            "⏸"
        } else {
            "▶"
        };

        let mut info_lines = vec![
            Line::from(Span::styled(
                item.display_title(),
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                item.subtitle(),
                Style::default().fg(Color::Cyan),
            )),
        ];

        if let Some(ref meta) = self.stream_metadata {
            info_lines.push(Line::from(Span::styled(
                meta.as_str(),
                Style::default().fg(Color::Magenta),
            )));
        } else {
            info_lines.push(Line::from(""));
        }

        if self.buffering {
            info_lines.push(Line::from(Span::styled(
                status,
                Style::default().fg(Color::Yellow),
            )));
        } else {
            info_lines.push(Line::from(format!("{} {}:{:02}", status, mins, secs)));
        }

        let info_para = Paragraph::new(info_lines).wrap(Wrap { trim: true });
        frame.render_widget(info_para, inner_chunks[0]);

        // ── Blob visualizer ──
        self.draw_blob(frame, inner_chunks[1]);

        // ── Tags / URL (bottom) ──
        if tag_height > 0 {
            let mut tag_lines = Vec::new();
            match item {
                DiscoveryItem::NtsEpisode { genres, .. }
                | DiscoveryItem::NtsLiveChannel { genres, .. } => {
                    if !genres.is_empty() {
                        tag_lines.push(Line::from(Span::styled(
                            format!("Tags: {}", genres.join(", ")),
                            Style::default().fg(Color::DarkGray),
                        )));
                    }
                }
                DiscoveryItem::DirectUrl { url, .. } => {
                    tag_lines.push(Line::from(Span::styled(
                        url.chars().take(200).collect::<String>(),
                        Style::default().fg(Color::DarkGray),
                    )));
                }
                _ => {}
            }
            if !tag_lines.is_empty() {
                let tag_para = Paragraph::new(tag_lines).wrap(Wrap { trim: true });
                frame.render_widget(tag_para, inner_chunks[2]);
            }
        }

        // ── Queue section ──
        if has_queue {
            self.draw_queue(frame, chunks[1]);
        }
    }
}

impl NowPlaying {
    fn draw_queue(&self, frame: &mut Frame, area: Rect) {
        // Draw horizontal separator at top of queue area
        let buf = frame.buffer_mut();
        for x in area.x..area.x + area.width {
            if let Some(cell) = buf.cell_mut((x, area.y)) {
                cell.set_char('─');
                cell.set_fg(Color::DarkGray);
            }
        }

        // Render title line below separator
        let title_line = Line::from(Span::styled(
            format!(" Queue ({})", self.queue_items.len()),
            Style::default().fg(Color::DarkGray),
        ));
        let title_area = Rect { x: area.x, y: area.y + 1, width: area.width, height: 1 };
        frame.render_widget(Paragraph::new(title_line), title_area);

        let inner = Rect {
            x: area.x + 1,
            y: area.y + 2,
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(2),
        };

        let lines: Vec<Line> = self
            .queue_items
            .iter()
            .enumerate()
            .map(|(i, (title, subtitle))| {
                let is_current = self.queue_current == Some(i);
                let marker = if is_current { "▶ " } else { "  " };
                let style = if is_current {
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                let sub_style = if is_current {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                Line::from(vec![
                    Span::styled(marker, style),
                    Span::styled(title.as_str(), style),
                    Span::styled(
                        if subtitle.is_empty() {
                            String::new()
                        } else {
                            format!(" - {}", subtitle)
                        },
                        sub_style,
                    ),
                ])
            })
            .collect();

        let paragraph = Paragraph::new(lines).wrap(Wrap { trim: true });
        frame.render_widget(paragraph, inner);
    }
}

/// Blend two ratatui colors. Works for named colors by mapping to approximate RGB.
fn blend_colors(c1: Color, c2: Color, t: f32) -> Color {
    let (r1, g1, b1) = color_to_rgb(c1);
    let (r2, g2, b2) = color_to_rgb(c2);
    let r = (r1 as f32 * (1.0 - t) + r2 as f32 * t) as u8;
    let g = (g1 as f32 * (1.0 - t) + g2 as f32 * t) as u8;
    let b = (b1 as f32 * (1.0 - t) + b2 as f32 * t) as u8;
    Color::Rgb(r, g, b)
}

fn color_to_rgb(c: Color) -> (u8, u8, u8) {
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
