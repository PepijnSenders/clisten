// Layout and rendering: splits the terminal into panels, draws dividers,
// and composites overlays (help, direct-play modal, error bar).

use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::components::direct_play_modal::DirectPlayModal;
use crate::components::discovery_list::DiscoveryList;
use crate::components::now_playing::NowPlaying;
use crate::components::nts::NtsTab;
use crate::components::play_controls::PlayControls;
use crate::components::search_bar::SearchBar;
use crate::components::Component;

pub struct DrawState<'a> {
    pub nts_tab: &'a NtsTab,
    pub discovery_list: &'a DiscoveryList,
    pub search_bar: &'a SearchBar,
    pub now_playing: &'a NowPlaying,
    pub play_controls: &'a PlayControls,
    pub direct_play_modal: &'a DirectPlayModal,
    pub error_message: &'a Option<String>,
    pub show_help: bool,
}

pub fn draw(frame: &mut Frame, state: &DrawState) {
    let error_height = if state.error_message.is_some() { 1 } else { 0 };
    let outer = Layout::vertical([
        Constraint::Min(0),
        Constraint::Length(error_height),
        Constraint::Length(4),
    ])
    .split(frame.area());

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let content_area = outer_block.inner(outer[0]);
    frame.render_widget(outer_block, outer[0]);

    let main = Layout::horizontal([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(content_area);

    let left = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(0),
        Constraint::Length(2),
    ])
    .split(main[0]);

    state.nts_tab.draw(frame, left[0]);
    state.discovery_list.draw(frame, left[1]);

    let search_input_area = Rect {
        x: left[2].x,
        y: left[2].y + 1,
        width: left[2].width,
        height: 1,
    };
    state.search_bar.draw(frame, search_input_area);
    state.now_playing.draw(frame, main[1]);

    draw_dividers(frame, content_area, main[0], left[2].y);

    if let Some(ref msg) = state.error_message {
        let error_line = Line::from(vec![
            Span::styled(" ⚠ ", Style::default().fg(Color::Red)),
            Span::styled(msg.as_str(), Style::default().fg(Color::Yellow)),
            Span::styled("  Press r to retry.", Style::default().fg(Color::DarkGray)),
        ]);
        frame.render_widget(Paragraph::new(error_line), outer[1]);
    }

    state.play_controls.draw(frame, outer[2]);

    if state.direct_play_modal.is_visible() {
        state.direct_play_modal.draw(frame, frame.area());
    }

    if state.show_help {
        draw_help_overlay(frame);
    }
}

fn draw_dividers(frame: &mut Frame, content_area: Rect, left_panel: Rect, search_sep_y: u16) {
    let buf = frame.buffer_mut();
    let divider_x = left_panel.x + left_panel.width;

    if divider_x < content_area.x + content_area.width {
        let top_y = content_area.y;
        let bottom_y = content_area.y + content_area.height;

        if let Some(cell) = buf.cell_mut((divider_x, top_y.saturating_sub(1))) {
            cell.set_char('┬');
            cell.set_fg(Color::DarkGray);
        }
        for y in top_y..bottom_y {
            if let Some(cell) = buf.cell_mut((divider_x, y)) {
                cell.set_char('│');
                cell.set_fg(Color::DarkGray);
            }
        }
        if let Some(cell) = buf.cell_mut((divider_x, bottom_y)) {
            cell.set_char('┴');
            cell.set_fg(Color::DarkGray);
        }
    }

    // Horizontal divider above search bar
    let left_x = content_area.x.saturating_sub(1);
    if let Some(cell) = buf.cell_mut((left_x, search_sep_y)) {
        cell.set_char('├');
        cell.set_fg(Color::DarkGray);
    }
    for x in content_area.x..left_panel.x + left_panel.width {
        if let Some(cell) = buf.cell_mut((x, search_sep_y)) {
            cell.set_char('─');
            cell.set_fg(Color::DarkGray);
        }
    }
    if divider_x < content_area.x + content_area.width {
        if let Some(cell) = buf.cell_mut((divider_x, search_sep_y)) {
            cell.set_char('┤');
            cell.set_fg(Color::DarkGray);
        }
    }
}

fn draw_help_overlay(frame: &mut Frame) {
    let area = frame.area();
    let overlay_width = 58u16;
    let overlay_height = 24u16;
    let x = area.width.saturating_sub(overlay_width) / 2;
    let y = area.height.saturating_sub(overlay_height) / 2;
    let overlay_area = Rect::new(
        x,
        y,
        overlay_width.min(area.width),
        overlay_height.min(area.height),
    );

    frame.render_widget(Clear, overlay_area);

    let keybindings = [
        ("q", "Quit"),
        ("1–3", "Switch sub-tab"),
        ("Tab", "Next sub-tab"),
        ("Shift+Tab", "Previous sub-tab"),
        ("j / Down", "Scroll down"),
        ("k / Up", "Scroll up"),
        ("Enter", "Play / select genre"),
        ("a", "Add to queue"),
        ("A", "Add to queue next (after current)"),
        ("Space", "Toggle play/pause"),
        ("n", "Next track in queue"),
        ("p", "Previous track in queue"),
        ("s", "Stop playback"),
        ("o", "Open URL (direct play)"),
        ("/", "Focus search bar"),
        ("Escape", "Unfocus search / go back"),
        ("f", "Toggle favorite"),
        ("c", "Clear queue"),
        ("[ ]", "Volume down/up"),
        ("?", "Toggle this help overlay"),
        ("r", "Retry failed request"),
    ];

    let mut lines: Vec<Line> = vec![
        Line::from(Span::styled(
            " Keybindings ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];
    for (key, desc) in &keybindings {
        lines.push(Line::from(vec![
            Span::styled(format!("  {:12}", key), Style::default().fg(Color::Yellow)),
            Span::raw(*desc),
        ]));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Press any key to close",
        Style::default().fg(Color::DarkGray),
    )));

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Help ")
        .title_alignment(Alignment::Center);
    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, overlay_area);
}
