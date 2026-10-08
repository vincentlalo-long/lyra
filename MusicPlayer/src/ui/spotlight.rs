use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};
use crate::{app::App, theme};

pub fn render(frame: &mut Frame, app: &mut App, area: Rect) {
    let block = Block::default()
        .title(" 󰐗 Spotlight & Specs ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::BLUE));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 2 || inner.width < 4 {
        return;
    }

    let mut lines = Vec::new();

    // 1. Up Next row
    let up_next_label = app
        .get_up_next_name()
        .unwrap_or_else(|| "End of queue / playlist".to_string());

    lines.push(Line::from(vec![
        Span::styled("Up Next: ", Style::default().fg(theme::YELLOW).add_modifier(Modifier::BOLD)),
        Span::styled(up_next_label, Style::default().fg(theme::TEXT)),
    ]));

    lines.push(Line::raw(""));

    // 2. Currently playing metadata & specs (from lazy cache)
    if let Some(path) = app.current_playing_path.clone() {
        let meta = app.song_meta(&path);
        
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("MP3")
            .to_uppercase();

        let title = if meta.title.is_empty() {
            path.file_stem().and_then(|s| s.to_str()).unwrap_or("Unknown").to_string()
        } else {
            meta.title
        };

        let dur_str = crate::meta::format_duration(meta.duration);

        if !title.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("Title:  ", Style::default().fg(theme::OVERLAY0)),
                Span::styled(title, Style::default().fg(theme::TEXT).add_modifier(Modifier::BOLD)),
            ]));
        }

        if !meta.artist.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("Artist: ", Style::default().fg(theme::OVERLAY0)),
                Span::styled(meta.artist, Style::default().fg(theme::ROSE)),
            ]));
        }

        if !meta.album.is_empty() && inner.height >= 7 {
            lines.push(Line::from(vec![
                Span::styled("Album:  ", Style::default().fg(theme::OVERLAY0)),
                Span::styled(meta.album, Style::default().fg(theme::SUBTEXT0)),
            ]));
        }

        if inner.height >= 8 {
            lines.push(Line::from(vec![
                Span::styled("Codec:  ", Style::default().fg(theme::OVERLAY0)),
                Span::styled(format!("{ext} • {dur_str} • 44.1kHz"), Style::default().fg(theme::SUBTEXT0)),
            ]));
        }
    } else {
        lines.push(Line::from(vec![
            Span::styled("Status: ", Style::default().fg(theme::OVERLAY0)),
            Span::styled("No track selected", Style::default().fg(theme::SUBTEXT0)),
        ]));
    }

    frame.render_widget(Paragraph::new(lines), inner);
}
