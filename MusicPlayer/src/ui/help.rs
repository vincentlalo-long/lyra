use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};
use crate::theme;

pub fn render(frame: &mut Frame) {
    let screen_area = frame.area();

    let popup_height = 20.min(screen_area.height);
    let popup_width = 58.min(screen_area.width);

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
        Line::from(Span::styled("  Navigation", Style::default().fg(theme::BLUE).add_modifier(Modifier::BOLD))),
        Line::from(vec![
            Span::styled("    Tab        ", Style::default().fg(theme::YELLOW)),
            Span::styled("Switch Playlist / Browser", Style::default().fg(theme::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("    ↑ / k      ", Style::default().fg(theme::YELLOW)),
            Span::styled("Move up", Style::default().fg(theme::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("    ↓ / j      ", Style::default().fg(theme::YELLOW)),
            Span::styled("Move down", Style::default().fg(theme::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("    Enter      ", Style::default().fg(theme::YELLOW)),
            Span::styled("Play track / Open folder", Style::default().fg(theme::TEXT)),
        ]),
        Line::raw(""),
        Line::from(Span::styled("  Playback", Style::default().fg(theme::GREEN).add_modifier(Modifier::BOLD))),
        Line::from(vec![
            Span::styled("    Space      ", Style::default().fg(theme::YELLOW)),
            Span::styled("Toggle Play / Pause", Style::default().fg(theme::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("    ← / h      ", Style::default().fg(theme::YELLOW)),
            Span::styled("Seek backward 5s", Style::default().fg(theme::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("    → / l      ", Style::default().fg(theme::YELLOW)),
            Span::styled("Seek forward 5s", Style::default().fg(theme::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("    n / p      ", Style::default().fg(theme::YELLOW)),
            Span::styled("Next / Previous track", Style::default().fg(theme::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("    + / -      ", Style::default().fg(theme::YELLOW)),
            Span::styled("Volume Up / Down", Style::default().fg(theme::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("    m          ", Style::default().fg(theme::YELLOW)),
            Span::styled("Toggle Mute", Style::default().fg(theme::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("    r          ", Style::default().fg(theme::YELLOW)),
            Span::styled("Toggle Repeat (All/One/Off)", Style::default().fg(theme::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("    Ctrl+r / R ", Style::default().fg(theme::YELLOW)),
            Span::styled("Replay current track from 00:00", Style::default().fg(theme::TEXT)),
        ]),
        Line::raw(""),
        Line::from(Span::styled("  Tools & System", Style::default().fg(theme::MAUVE).add_modifier(Modifier::BOLD))),
        Line::from(vec![
            Span::styled("    /          ", Style::default().fg(theme::YELLOW)),
            Span::styled("Search / Filter (Esc to cancel)", Style::default().fg(theme::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("    s          ", Style::default().fg(theme::YELLOW)),
            Span::styled("Scan subdirectories for MP3s", Style::default().fg(theme::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("    .          ", Style::default().fg(theme::YELLOW)),
            Span::styled("Toggle hidden dotfiles", Style::default().fg(theme::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("    ?          ", Style::default().fg(theme::YELLOW)),
            Span::styled("Toggle this Help popup", Style::default().fg(theme::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("    q          ", Style::default().fg(theme::YELLOW)),
            Span::styled("Quit Lyra", Style::default().fg(theme::TEXT)),
        ]),
    ];

    let block = Block::default()
        .title(" 󰋖 Help & Keybindings ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::MAUVE).add_modifier(Modifier::BOLD));

    frame.render_widget(Clear, popup_area);
    frame.render_widget(Paragraph::new(lines).block(block), popup_area);
}
