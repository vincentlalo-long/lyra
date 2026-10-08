use ratatui::{
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};
use crate::{app::App, theme};

fn short_name(path: &std::path::Path) -> String {
    path.file_stem()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Unknown".to_string())
}

/// Left-pane scratchpad: pinned now-playing header + upcoming rows.
/// Deleting here (`d`) only unqueues; files on disk are never touched.
pub fn render(frame: &mut Frame, app: &mut App, area: Rect) {
    let mut list_items = Vec::new();

    if app.queue.items.is_empty() {
        list_items.push(ListItem::new(Line::from(vec![
            Span::styled("  (empty)", Style::default().fg(theme::OVERLAY0)),
        ])));
        list_items.push(ListItem::new(Line::from(vec![
            Span::styled("  a: queue song   A: play next", Style::default().fg(theme::OVERLAY0)),
        ])));
    }

    for (pos, song_path) in app.queue.items.iter().enumerate() {
        let is_selected = pos == app.queue.selected;
        let style = if is_selected {
            Style::default()
                .fg(theme::YELLOW)
                .bg(theme::SURFACE0)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme::TEXT)
        };
        let marker = if is_selected { " " } else { "  " };
        let marker_style = if is_selected {
            Style::default().fg(theme::YELLOW).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme::OVERLAY0)
        };
        list_items.push(ListItem::new(Line::from(vec![
            Span::styled(marker, marker_style),
            Span::styled(format!("{:02}. ", pos + 1), style),
            Span::styled(short_name(song_path), style),
        ])));
    }

    // Pinned header: what is playing now (queue or library).
    let now_label = match &app.current_playing_path {
        Some(p) if app.queue.now_from_queue => format!("▶ {} (queued)", short_name(p)),
        Some(p) => format!("▶ {} (library)", short_name(p)),
        None => "▶ —".to_string(),
    };

    let title = format!(" 󰐗 Queue ({}) ", app.queue.len());

    // Split area: 1 line header + list below.
    let header = Paragraph::new(Line::from(vec![
        Span::styled("Now: ", Style::default().fg(theme::OVERLAY0)),
        Span::styled(now_label, Style::default().fg(theme::GREEN).add_modifier(Modifier::BOLD)),
    ]));
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(area);
    frame.render_widget(header, chunks[0]);

    let list_widget = List::new(list_items).block(
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme::MAUVE)),
    );

    let selected_pos = if app.queue.items.is_empty() {
        None
    } else {
        Some(app.queue.selected)
    };
    app.queue.state.select(selected_pos);
    frame.render_stateful_widget(list_widget, chunks[1], &mut app.queue.state);

    if app.queue.len() > 1 {
        let mut scrollbar_state =
            ScrollbarState::new(app.queue.len()).position(selected_pos.unwrap_or(0));
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None)
            .thumb_style(Style::default().fg(theme::MAUVE))
            .track_style(Style::default().fg(theme::SURFACE0));
        frame.render_stateful_widget(
            scrollbar,
            chunks[1].inner(Margin { vertical: 1, horizontal: 0 }),
            &mut scrollbar_state,
        );
    }
}

