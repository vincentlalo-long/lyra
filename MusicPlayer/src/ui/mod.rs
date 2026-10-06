mod browser;
mod controls;
mod loading;
mod playlist;

use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::{App, ViewMode};

pub fn render(frame: &mut Frame, app: &App) {
    let layout_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(5),    // Main view
            Constraint::Length(4), // Footer
        ])
        .split(frame.area());

    let playlist_tab_style = if app.view_mode == ViewMode::Playlist {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let browser_tab_style = if app.view_mode == ViewMode::Browser {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let header_content = Paragraph::new(vec![Line::from(vec![
        Span::styled("🎵 LYRA PLAYER  ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw(" |  Mode [Tab]: "),
        Span::styled(" [1] Playlist ", playlist_tab_style),
        Span::raw(" "),
        Span::styled(" [2] File Browser ", browser_tab_style),
    ])])
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(header_content, layout_chunks[0]);

    match app.view_mode {
        ViewMode::Playlist => playlist::render(frame, app, layout_chunks[1]),
        ViewMode::Browser => browser::render(frame, app, layout_chunks[1]),
    }

    controls::render(frame, app, layout_chunks[2]);

    // If scanning in background, render animated loading dialog on top
    if app.scanner.is_scanning {
        loading::render(frame, &app.scanner);
    }
}
