use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use crate::scanner::Scanner;

/// Renders an animated loading popup rectangle in the center of the screen
pub fn render(frame: &mut Frame, scanner: &Scanner) {
    let screen_area = frame.area();

    // Center a 60-character wide, 7-line high popup box
    let vertical_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((screen_area.height.saturating_sub(7)) / 2),
            Constraint::Length(7),
            Constraint::Min(0),
        ])
        .split(screen_area);

    let popup_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length((screen_area.width.saturating_sub(60)) / 2),
            Constraint::Length(60.min(screen_area.width)),
            Constraint::Min(0),
        ])
        .split(vertical_chunks[1])[1];

    // Clock rocking / ticking animation frames
    let clocks = ["🕐", "🕑", "🕒", "🕓", "🕔", "🕕", "🕖", "🕗", "🕘", "🕙", "🕚", "🕛"];
    let current_clock = clocks[scanner.anim_tick % clocks.len()];
    let pendulum = if (scanner.anim_tick / 2) % 2 == 0 { "▲" } else { "▼" };

    // Filling rectangle progress bar based on folders scanned
    let bar_width = 44;
    let filled_count = (scanner.folders_scanned / 2) % (bar_width + 1);
    let empty_count = bar_width.saturating_sub(filled_count);
    let progress_bar = format!("[{}{}]", "■".repeat(filled_count), " ".repeat(empty_count));

    let title_line = Line::from(vec![
        Span::styled(format!("{pendulum} {current_clock} "), Style::default().fg(Color::Yellow)),
        Span::styled("Scanning Subdirectories ... ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled(format!("{current_clock} {pendulum}"), Style::default().fg(Color::Yellow)),
    ]);

    let stats_line = Line::from(Span::styled(
        format!("Folders scanned: {}  |  MP3s found: {}", scanner.folders_scanned, scanner.songs_found),
        Style::default().fg(Color::White),
    ));

    let bar_line = Line::from(Span::styled(progress_bar, Style::default().fg(Color::Green)));

    let dialog_content = Paragraph::new(vec![
        Line::raw(""),
        title_line,
        stats_line,
        bar_line,
    ])
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Background Task ")
            .border_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
    );

    // Clear the background under the popup so lists don't bleed through
    frame.render_widget(Clear, popup_area);
    frame.render_widget(dialog_content, popup_area);
}
