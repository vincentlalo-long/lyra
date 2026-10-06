use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use crate::app::App;

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let sub_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Line 1: Track Status & Volume Bar
            Constraint::Length(1), // Line 2: Keybindings help
        ])
        .split(Block::default().borders(Borders::ALL).title(" Controls ").inner(area));

    // 1. Status and Volume
    let status_str = if let Some(playing_index) = app.playlist.playing_index {
        let song_name = match app.playlist.songs.get(playing_index) {
            Some(path) => path.file_name().and_then(|n| n.to_str()).unwrap_or("Unknown"),
            None => "Unknown",
        };
        let play_status = if app.audio.is_paused { "[PAUSED]" } else { "[PLAYING]" };
        format!("Status: {play_status}  Track: {song_name}")
    } else {
        "Status: [STOPPED]  No track selected".to_string()
    };

    let vol_percent = (app.audio.volume * 100.0).round() as u32;
    let filled_bars = (vol_percent / 10).min(10) as usize;
    let empty_bars = 10 - filled_bars;
    let volume_display = format!("Vol: {:>3}% [{}{}]", vol_percent, "=".repeat(filled_bars), " ".repeat(empty_bars));

    let line1 = Line::from(vec![
        Span::styled(status_str, Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        Span::raw("    "),
        Span::styled(volume_display, Style::default().fg(if vol_percent == 0 { Color::Red } else { Color::Yellow })),
    ]);

    // 2. Keybindings Help
    let keybindings_help = "[Tab] View   [.] Dotfiles   [s] Scan Subdirs   [↑/k] Up   [↓/j] Down   [Enter] Play   [Space] Pause   [n/p] Next/Prev   [+/-] Vol   [q] Quit";
    let line2 = Line::from(Span::styled(keybindings_help, Style::default().fg(Color::DarkGray)));

    // Outer block
    frame.render_widget(Block::default().borders(Borders::ALL).title(" Controls "), area);
    frame.render_widget(Paragraph::new(line1), sub_chunks[0]);
    frame.render_widget(Paragraph::new(line2), sub_chunks[1]);
}
