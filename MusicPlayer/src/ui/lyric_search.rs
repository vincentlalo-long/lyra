use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};
use crate::app::App;

pub fn render(frame: &mut Frame, app: &App) {
    let Some(modal) = &app.lyric_search_modal else {
        return;
    };
    let screen = frame.area();
    let popup_w: u16 = 60u16.min(screen.width.saturating_sub(4));
    let popup_h: u16 = ((modal.results.len() + 6).min(18).min(screen.height.saturating_sub(4) as usize)) as u16;

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

    let block = Block::default()
        .title(Span::styled(
            "  Search Lyrics (LRCLIB) ",
            Style::default().fg(app.theme.accent).add_modifier(Modifier::BOLD),
        ))
        .title_bottom(Line::from(vec![
            Span::styled(" ↑/k: Up  ↓/j: Down  ", Style::default().fg(app.theme.faint)),
            Span::styled("Enter: Save  Esc: Cancel", Style::default().fg(app.theme.faint)),
        ]))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(app.theme.border_focused));

    let inner = block.inner(popup_area);

    if inner.height < 2 {
        return;
    }

    let mut lines = Vec::new();

    // Query line
    lines.push(Line::from(vec![
        Span::styled("Query: ", Style::default().fg(app.theme.warning).add_modifier(Modifier::BOLD)),
        Span::styled(&modal.query, Style::default().fg(app.theme.text)),
    ]));
    lines.push(Line::raw(""));

    if modal.results.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("No results found.", Style::default().fg(app.theme.muted)),
        ]));
    } else {
        lines.push(Line::from(vec![
            Span::styled("Results: ", Style::default().fg(app.theme.warning).add_modifier(Modifier::BOLD)),
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

            lines.push(Line::from(vec![
                Span::styled(prefix, prefix_style),
                Span::styled(
                    format!("{}{} - {} ({}s){}", result.artist, result.title, result.album, result.duration, synced_tag),
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
