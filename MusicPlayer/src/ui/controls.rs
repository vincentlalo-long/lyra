use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use crate::app::App;

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let status_message = if let Some(playing_index) = app.playing_index {
        let song_name = match app.songs.get(playing_index) {
            Some(path) => path.file_name().and_then(|n| n.to_str()).unwrap_or("Unknown"),
            None => "Unknown",
        };
        let play_status = if app.audio.is_paused { "[PAUSED]" } else { "[PLAYING]" };
        format!("Status: {play_status}  Track: {song_name}")
    } else {
        "Status: [STOPPED]  No track selected".to_string()
    };

    let keybindings_help = "[Tab] Switch View   [↑ / k] Up   [↓ / j] Down   [Enter] Play/Open   [Space] Pause   [q] Quit";

    let controls_widget = Paragraph::new(vec![
        Line::from(Span::styled(status_message, Style::default().fg(Color::Green))),
        Line::from(Span::styled(keybindings_help, Style::default().fg(Color::DarkGray))),
    ])
    .block(Block::default().borders(Borders::ALL).title(" Controls "));

    frame.render_widget(controls_widget, area);
}
