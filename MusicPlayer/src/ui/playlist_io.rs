use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use crate::app::{App, PlaylistIoMode};

pub fn render(frame: &mut Frame, app: &App) {
    let Some(modal) = &app.playlist_io_modal else {
        return;
    };
    let screen = frame.area();
    let popup_w = 60.min(screen.width.saturating_sub(4));
    let popup_h = 10.min(screen.height.saturating_sub(4));

    let v_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((screen.height.saturating_sub(popup_h)) / 2),
            Constraint::Length(popup_h),
            Constraint::Min(0),
        ])
        .split(screen);

    let h_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length((screen.width.saturating_sub(4)) / 2),
            Constraint::Length(popup_w),
            Constraint::Min(0),
        ])
        .split(v_chunks[1]);

    let popup_area = h_chunks[1];
    let title = match modal.mode {
        PlaylistIoMode::Import => "  Import Playlist (M3U)  ",
        PlaylistIoMode::Export => "  Export Playlist (M3U)  ",
    };
    let block = Block::default()
        .title(title)
        .title_bottom(Line::from(vec![
            Span::styled(" [Enter: Confirm | Esc: Cancel] ", Style::default().fg(app.theme.faint)),
        ]))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.border_focused));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let mode_text = match modal.mode {
        PlaylistIoMode::Import => "Read tracks from:",
        PlaylistIoMode::Export => "Write tracks to:",
    };
    let lines = vec![
        Line::from(vec![
            Span::styled(mode_text, Style::default().fg(app.theme.warning).add_modifier(Modifier::BOLD)),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(&modal.input, Style::default().fg(app.theme.text).bg(app.theme.selected_bg)),
            Span::styled("█", Style::default().fg(app.theme.hint)),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled(" Tracks in playlist: ", Style::default().fg(app.theme.faint)),
            Span::styled(app.playlist.songs.len().to_string(), Style::default().fg(app.theme.info).add_modifier(Modifier::BOLD)),
        ]),
    ];

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}
