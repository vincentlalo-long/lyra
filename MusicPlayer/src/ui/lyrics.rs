use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};
use crate::{app::App, theme};

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(" 󰎈 Lyrics ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::MAUVE));

    let lyrics = match &app.lyrics {
        Some(l) if !l.lines.is_empty() => l,
        _ => {
            let empty_msg = Paragraph::new(vec![
                Line::raw(""),
                Line::raw(""),
                Line::from(Span::styled(
                    "󰎈  No synchronized lyrics (.lrc) found",
                    Style::default().fg(theme::OVERLAY0),
                )),
                Line::raw(""),
                Line::from(Span::styled(
                    "Drop a matching .lrc file in the same folder",
                    Style::default().fg(theme::SURFACE2),
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

    // Pad top to center the active line
    let pad_top = half_height.saturating_sub(active_index);
    for _ in 0..pad_top {
        text_lines.push(Line::raw(""));
    }

    for i in start_idx..end_idx {
        let line_data = &lyrics.lines[i];
        let distance = if i >= active_index {
            i - active_index
        } else {
            active_index - i
        };

        // Depth of Field styling
        let (style, prefix, suffix) = match distance {
            0 => (
                Style::default().fg(theme::YELLOW).add_modifier(Modifier::BOLD),
                "▶  ",
                "  ◀",
            ),
            1 => (Style::default().fg(theme::TEXT), "   ", ""),
            2 => (Style::default().fg(theme::SUBTEXT0), "   ", ""),
            3 => (Style::default().fg(theme::OVERLAY0), "   ", ""),
            _ => (Style::default().fg(theme::SURFACE2), "   ", ""),
        };

        let formatted = format!("{prefix}{}{suffix}", line_data.text);
        text_lines.push(Line::from(Span::styled(formatted, style)));
    }

    let paragraph = Paragraph::new(text_lines)
        .alignment(Alignment::Center)
        .block(block);

    frame.render_widget(paragraph, area);
}
