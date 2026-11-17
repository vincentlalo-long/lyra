use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};
use crate::app::{App, LyricVerifyStatus};

pub fn render(frame: &mut Frame, app: &App) {
    let Some(modal) = &app.lyric_search_modal else {
        return;
    };
    let screen = frame.area();
    let popup_w: u16 = 68u16.min(screen.width.saturating_sub(4));
    let popup_h: u16 = ((modal.results.len() + 8).min(20).min(screen.height.saturating_sub(4) as usize)) as u16;

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
            Constraint::Length((screen.width.saturating_sub(popup_w)) / 2),
            Constraint::Length(popup_w),
            Constraint::Min(0),
        ])
        .split(v_chunks[1]);

    let popup_area = h_chunks[1];

    // Colored background for the popup
    let bg_lines: Vec<Line> = (0..popup_area.height as usize)
        .map(|_| Line::from(" ".repeat(popup_area.width as usize)))
        .collect();
    frame.render_widget(
        Paragraph::new(bg_lines)
            .style(Style::default().bg(app.theme.selected_bg)),
        popup_area,
    );

    let bottom_hints = if modal.editing_query {
        Line::from(vec![
            Span::styled(" Type query (Artist - Title)  ", Style::default().fg(app.theme.faint)),
            Span::styled("Enter: Search  Esc: Cancel Edit", Style::default().fg(app.theme.faint)),
        ])
    } else {
        Line::from(vec![
            Span::styled(" ↑/k ↓/j  /: Query  a: AI Whisper  v: Verify  ", Style::default().fg(app.theme.faint)),
            Span::styled("Enter: Save  Esc: Close", Style::default().fg(app.theme.faint)),
        ])
    };

    let block = Block::default()
        .title(Span::styled(
            "  Search Lyrics (LRCLIB) ",
            Style::default().fg(app.theme.accent).add_modifier(Modifier::BOLD),
        ))
        .title_bottom(bottom_hints)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(app.theme.border_focused));

    let inner = block.inner(popup_area);

    if inner.height < 2 {
        return;
    }

    let mut lines = Vec::new();

    // Query line
    let mut query_spans = vec![
        Span::styled("Query: ", Style::default().fg(app.theme.warning).add_modifier(Modifier::BOLD)),
        Span::styled(&modal.query, Style::default().fg(app.theme.text).add_modifier(Modifier::BOLD)),
    ];
    if modal.editing_query {
        query_spans.push(Span::styled("█", Style::default().fg(app.theme.warning)));
    }
    lines.push(Line::from(query_spans));

    // Verification status of any existing `.lrc` on disk
    if let Some(sp) = &modal.song_path {
        if let Some(st) = app.lyric_verify_cache.get(sp) {
            let (msg, color) = match st {
                LyricVerifyStatus::Checking => ("Checking against audio…".to_string(), app.theme.info),
                LyricVerifyStatus::Verified { confidence } => (
                    format!("✓ Verified match ({confidence}% with vocals)"),
                    app.theme.success,
                ),
                LyricVerifyStatus::Plain { confidence } => (
                    format!("Plain untimed text ({confidence}% overlap) — pick synced below"),
                    app.theme.warning,
                ),
                LyricVerifyStatus::Mismatch { confidence, heard } => {
                    let clip: String = heard.chars().take(34).collect();
                    let extra = if clip.is_empty() {
                        String::new()
                    } else {
                        format!(" [heard: \"{clip}\"]")
                    };
                    (format!("⚠️ MISMATCH ({confidence}%){extra}"), app.theme.danger)
                }
                LyricVerifyStatus::Missing => ("No .lrc file on disk yet".to_string(), app.theme.faint),
            };
            lines.push(Line::from(vec![
                Span::styled("Current .lrc: ", Style::default().fg(app.theme.faint)),
                Span::styled(msg, Style::default().fg(color).add_modifier(Modifier::BOLD)),
            ]));
        }
    }
    lines.push(Line::raw(""));

    if modal.is_loading {
        lines.push(Line::from(vec![
            Span::styled("⏳ Searching LRCLIB / Groq Whisper AI for synced lyrics...", Style::default().fg(app.theme.info)),
        ]));
    } else if modal.results.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("No results found. Press '/' to edit query or 'a' for AI Whisper.", Style::default().fg(app.theme.muted)),
        ]));
    } else {
        lines.push(Line::from(vec![
            Span::styled(
                format!("Results ({}):", modal.results.len()),
                Style::default().fg(app.theme.warning).add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::raw(""));

        for (i, result) in modal.results.iter().enumerate() {
            let is_selected = i == modal.selected;
            let prefix = if is_selected {
                "▸ "
            } else {
                "  "
            };
            let prefix_style = if is_selected {
                Style::default().fg(app.theme.accent).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(app.theme.faint)
            };

            let synced_tag = if result.synced {
                " [synced]"
            } else {
                " [plain]"
            };
            let album_tag = if result.album.is_empty() {
                String::new()
            } else {
                format!(" [{}]", result.album)
            };
            let artist_title = if result.artist.is_empty() {
                result.title.clone()
            } else {
                format!("{} - {}", result.artist, result.title)
            };

            lines.push(Line::from(vec![
                Span::styled(prefix, prefix_style),
                Span::styled(
                    format!("{artist_title}{album_tag} ({}s){synced_tag}", result.duration),
                    if is_selected {
                        Style::default().fg(app.theme.warning).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(app.theme.text)
                    },
                ),
            ]));
        }
    }

    lines.push(Line::raw(""));
    lines.push(Line::from(vec![
        Span::styled("Selected song: ", Style::default().fg(app.theme.faint)),
        Span::styled(
            modal.song_path.as_ref()
                .and_then(|p| p.file_name().and_then(|n| n.to_str()))
                .unwrap_or("unknown"),
            Style::default().fg(app.theme.info).add_modifier(Modifier::BOLD),
        ),
    ]));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .style(Style::default().bg(app.theme.selected_bg));
    frame.render_widget(paragraph, popup_area);
}
