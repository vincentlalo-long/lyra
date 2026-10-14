use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};
use crate::app::App;

pub fn render(frame: &mut Frame, app: &mut App, area: Rect) {
    // Same left-column family tint as the cover panel above it, so the
    // Studio layout reads as one visual unit (see ui/cover.rs).
    let block = Block::default()
        .title(Span::styled(
            " 󰐗 Spotlight & Specs ",
            Style::default().fg(app.theme.accent_dim).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(app.theme.accent_dim));

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
        Span::styled("Up Next: ", Style::default().fg(app.theme.warning).add_modifier(Modifier::BOLD)),
        Span::styled(up_next_label, Style::default().fg(app.theme.text)),
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
                Span::styled("Title:  ", Style::default().fg(app.theme.faint)),
                Span::styled(title, Style::default().fg(app.theme.text).add_modifier(Modifier::BOLD)),
            ]));
        }

        if !meta.artist.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("Artist: ", Style::default().fg(app.theme.faint)),
                Span::styled(meta.artist, Style::default().fg(app.theme.accent_dim)),
            ]));
        }

        if !meta.album.is_empty() && inner.height >= 7 {
            lines.push(Line::from(vec![
                Span::styled("Album:  ", Style::default().fg(app.theme.faint)),
                Span::styled(meta.album, Style::default().fg(app.theme.muted)),
            ]));
        }

        if inner.height >= 8 {
            lines.push(Line::from(vec![
                Span::styled("Codec:  ", Style::default().fg(app.theme.faint)),
                Span::styled(format!("{ext} • {dur_str} • 44.1kHz"), Style::default().fg(app.theme.muted)),
            ]));
        }
    } else {
        lines.push(Line::from(vec![
            Span::styled("Status: ", Style::default().fg(app.theme.faint)),
            Span::styled("No track selected", Style::default().fg(app.theme.muted)),
        ]));
    }

    frame.render_widget(Paragraph::new(lines), inner);
}
