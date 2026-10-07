mod browser;
mod controls;
mod help;
mod loading;
mod lyrics;
mod playlist;

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::{
    app::{App, ViewMode},
    theme,
};

pub fn render(frame: &mut Frame, app: &App) {
    let layout_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(5),    // Dual Panel Body
            Constraint::Length(4), // Footer Controls
        ])
        .split(frame.area());

    // Header tabs
    let playlist_tab_style = if app.view_mode == ViewMode::Playlist {
        Style::default().fg(theme::MAUVE).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme::OVERLAY0)
    };

    let browser_tab_style = if app.view_mode == ViewMode::Browser {
        Style::default().fg(theme::MAUVE).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme::OVERLAY0)
    };

    let mut header_spans = vec![
        Span::styled(" 󰎆 LYRA MUSIC PLAYER", Style::default().fg(theme::BLUE).add_modifier(Modifier::BOLD)),
        Span::styled("   [Tab]: ", Style::default().fg(theme::OVERLAY0)),
        Span::styled("󰲸 Playlist", playlist_tab_style),
        Span::raw("  "),
        Span::styled("󰉋 Browser", browser_tab_style),
    ];

    if app.is_searching {
        header_spans.push(Span::raw("  "));
        header_spans.push(Span::styled(
            format!("🔍 /{}_ ", app.search_query),
            Style::default().fg(theme::YELLOW).add_modifier(Modifier::BOLD),
        ));
        header_spans.push(Span::styled(
            "[Enter: Play | Esc: Cancel]",
            Style::default().fg(theme::OVERLAY0),
        ));
    } else if !app.search_query.is_empty() {
        header_spans.push(Span::raw("  "));
        header_spans.push(Span::styled(
            format!("🔍 \"{}\" [Esc: Clear]", app.search_query),
            Style::default().fg(theme::YELLOW),
        ));
    }

    let help_info = Line::from(vec![
        Span::styled("[", Style::default().fg(theme::OVERLAY0)),
        Span::styled("q", Style::default().fg(theme::YELLOW).add_modifier(Modifier::BOLD)),
        Span::styled(": Quit | ", Style::default().fg(theme::TEXT)),
        Span::styled("/", Style::default().fg(theme::YELLOW).add_modifier(Modifier::BOLD)),
        Span::styled(": Search | ", Style::default().fg(theme::TEXT)),
        Span::styled("?", Style::default().fg(theme::YELLOW).add_modifier(Modifier::BOLD)),
        Span::styled(": Help", Style::default().fg(theme::TEXT)),
        Span::styled("] ", Style::default().fg(theme::OVERLAY0)),
    ]);

    let header_widget = Paragraph::new(Line::from(header_spans))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme::BLUE))
                .title(help_info)
                .title_alignment(Alignment::Right),
        );
    frame.render_widget(header_widget, layout_chunks[0]);

    // Dual Panel Body: 40% Left, 60% Right
    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(40), // Left: Playlist or Browser
            Constraint::Percentage(60), // Right: Lyrics Karaoke
        ])
        .split(layout_chunks[1]);

    match app.view_mode {
        ViewMode::Playlist => playlist::render(frame, app, body_chunks[0]),
        ViewMode::Browser => browser::render(frame, app, body_chunks[0]),
    }

    lyrics::render(frame, app, body_chunks[1]);

    controls::render(frame, app, layout_chunks[2]);

    // Popup overlays
    if app.scanner.is_scanning {
        loading::render(frame, &app.scanner);
    }

    if app.show_help {
        help::render(frame);
    }
}
