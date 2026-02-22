// Queue list: renders the playback queue below the now-playing panel.

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
    Frame,
};

use crate::theme::Theme;

/// Render the playback queue as a styled list with key hints at the bottom.
pub fn draw(
    frame: &mut Frame,
    area: Rect,
    items: &[(String, String)],
    current: Option<usize>,
    theme: &Theme,
) {
    // Horizontal separator
    let buf = frame.buffer_mut();
    for x in area.x..area.x + area.width {
        if let Some(cell) = buf.cell_mut((x, area.y)) {
            cell.set_char('─');
            cell.set_fg(theme.border);
        }
    }

    let title = Line::from(Span::styled(
        format!(" Queue ({})", items.len()),
        Style::default().fg(theme.text_dim),
    ));
    let title_area = Rect {
        x: area.x,
        y: area.y + 1,
        width: area.width,
        height: 1,
    };
    frame.render_widget(Paragraph::new(title), title_area);

    let inner = Rect {
        x: area.x + 1,
        y: area.y + 2,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };

    let lines: Vec<Line> = items
        .iter()
        .enumerate()
        .map(|(i, (title, subtitle))| {
            let is_current = current == Some(i);
            let marker = if is_current { "▶ " } else { "  " };
            let style = if is_current {
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.text)
            };
            let sub_style = if is_current {
                Style::default().fg(theme.primary)
            } else {
                Style::default().fg(theme.text_dim)
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

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), inner);

    // Key hints at bottom of queue area
    if area.height >= 4 {
        let hint_y = area.y + area.height - 1;
        let hint_area = Rect {
            x: area.x + 1,
            y: hint_y,
            width: area.width.saturating_sub(2),
            height: 1,
        };
        let hints = Line::from(vec![
            Span::styled("d", Style::default().fg(theme.text)),
            Span::styled(" Remove ", Style::default().fg(theme.text_dim)),
            Span::styled("│", Style::default().fg(theme.border)),
            Span::styled(" c", Style::default().fg(theme.text)),
            Span::styled(" Clear ", Style::default().fg(theme.text_dim)),
            Span::styled("│", Style::default().fg(theme.border)),
            Span::styled(" n", Style::default().fg(theme.text)),
            Span::styled(" Next ", Style::default().fg(theme.text_dim)),
            Span::styled("│", Style::default().fg(theme.border)),
            Span::styled(" p", Style::default().fg(theme.text)),
            Span::styled(" Prev", Style::default().fg(theme.text_dim)),
        ]);
        frame.render_widget(Paragraph::new(hints), hint_area);
    }
}
