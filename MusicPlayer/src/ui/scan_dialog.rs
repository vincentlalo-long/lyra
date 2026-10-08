use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};
use crate::theme;

pub fn render(frame: &mut Frame, count: usize) {
    let screen_area = frame.area();

    let popup_height = 9.min(screen_area.height);
    let popup_width = 56.min(screen_area.width);

    let vertical_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((screen_area.height.saturating_sub(popup_height)) / 2),
            Constraint::Length(popup_height),
            Constraint::Min(0),
        ])
        .split(screen_area);

    let popup_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length((screen_area.width.saturating_sub(popup_width)) / 2),
            Constraint::Length(popup_width),
            Constraint::Min(0),
        ])
        .split(vertical_chunks[1])[1];

    let lines = vec![
        Line::from(vec![
            Span::styled(" 󰲸 Found ", Style::default().fg(theme::TEXT)),
            Span::styled(format!("{count}"), Style::default().fg(theme::GREEN).add_modifier(Modifier::BOLD)),
            Span::styled(" songs in scanned directory.", Style::default().fg(theme::TEXT)),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled("    [Enter] ", Style::default().fg(theme::YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled("Replace current playlist", Style::default().fg(theme::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("    [a]     ", Style::default().fg(theme::YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled("Append to playlist", Style::default().fg(theme::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("    [q]     ", Style::default().fg(theme::YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled("Add all to queue", Style::default().fg(theme::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("    [Esc]   ", Style::default().fg(theme::YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled("Dismiss / Cancel", Style::default().fg(theme::OVERLAY0)),
        ]),
    ];

    let block = Block::default()
        .title(" 󱑂 Scan Complete ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::GREEN).add_modifier(Modifier::BOLD));

    frame.render_widget(Clear, popup_area);
    frame.render_widget(Paragraph::new(lines).block(block), popup_area);
}
