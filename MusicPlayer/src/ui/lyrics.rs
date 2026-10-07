use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use crate::app::App;

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(" Lyrics (.lrc) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta));

    let lyrics = match &app.lyrics {
        Some(l) if !l.lines.is_empty() => l,
        _ => {
            let empty_msg = Paragraph::new(vec![
                Line::raw(""),
                Line::from(Span::styled(
                    "No synchronized lyrics (.lrc) found for this track.",
                    Style::default().fg(Color::DarkGray),
                )),
                Line::raw(""),
                Line::from(Span::styled(
                    "Drop a matching .lrc file in the same folder to see lyrics here.",
                    Style::default().fg(Color::DarkGray),
                )),
            ])
            .alignment(Alignment::Center)
            .block(block);

            frame.render_widget(empty_msg, area);
            return;
        }
    };

    let current_pos = app.audio.position();
    let active_index = lyrics
        .lines
        .iter()
        .position(|l| l.timestamp > current_pos)
        .unwrap_or(lyrics.lines.len())
        .saturating_sub(1);

    let inner_height = area.height.saturating_sub(2) as usize;
    let half_height = inner_height / 2;
    let start_idx = active_index.saturating_sub(half_height);
    let end_idx = (start_idx + inner_height).min(lyrics.lines.len());

    let mut text_lines = Vec::new();

    // Pad top if needed so active line stays centered
    let pad_top = half_height.saturating_sub(active_index);
    for _ in 0..pad_top {
        text_lines.push(Line::raw(""));
    }

    for i in start_idx..end_idx {
        let line_data = &lyrics.lines[i];
        let is_active = i == active_index;

        let style = if is_active {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else if i < active_index {
            Style::default().fg(Color::White)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let prefix = if is_active { "▶ " } else { "  " };
        let formatted = format!("{prefix}{}", line_data.text);
        text_lines.push(Line::from(Span::styled(formatted, style)));
    }

    let paragraph = Paragraph::new(text_lines)
        .alignment(Alignment::Center)
        .block(block);

    frame.render_widget(paragraph, area);
}
