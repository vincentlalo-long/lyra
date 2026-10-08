use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};
use crate::{app::App, theme};

pub fn render(frame: &mut Frame, app: &mut App) {
    let screen_area = frame.area();
    let popup_width = 64.min(screen_area.width.saturating_sub(4));
    let popup_height = 20.min(screen_area.height.saturating_sub(4));

    let vert = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((screen_area.height.saturating_sub(popup_height)) / 2),
            Constraint::Length(popup_height),
            Constraint::Min(0),
        ])
        .split(screen_area);

    let popup_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length((screen_area.width.saturating_sub(popup_width)) / 2),
            Constraint::Length(popup_width),
            Constraint::Min(0),
        ])
        .split(vert[1])[1];

    frame.render_widget(Clear, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Header: current folder path
            Constraint::Min(5),    // Subfolder list
            Constraint::Length(2), // Bottom hints
        ])
        .split(popup_area.inner(Margin { vertical: 1, horizontal: 1 }));

    // Block container
    let block = Block::default()
        .title(" 󰉋 Browse Destination Directory ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::BLUE).add_modifier(Modifier::BOLD));
    frame.render_widget(block, popup_area);

    // Current dir label
    let curr_path_str = app.download.dir_picker.current_dir.to_string_lossy();
    let path_p = Paragraph::new(Line::from(vec![
        Span::styled("Current: ", Style::default().fg(theme::OVERLAY0)),
        Span::styled(curr_path_str, Style::default().fg(theme::YELLOW).add_modifier(Modifier::BOLD)),
    ]));
    frame.render_widget(path_p, chunks[0]);

    // Entries list (only dirs)
    let entries = &app.download.dir_picker.entries;
    let mut list_items = Vec::new();
    for (i, entry) in entries.iter().enumerate() {
        let is_selected = i == app.download.dir_picker.selected;
        let is_parent = app.download.dir_picker.current_dir.parent() == Some(entry.as_path());

        let name = if is_parent {
            ".. (Go to parent folder)".to_string()
        } else {
            entry.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "/".into())
        };

        let style = if is_selected {
            Style::default().fg(theme::GREEN).bg(theme::SURFACE0).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme::TEXT)
        };

        let icon = if is_parent { "󰉍 " } else { "󰉋 " };
        list_items.push(ListItem::new(Line::from(vec![
            Span::styled(icon, if is_selected { Style::default().fg(theme::YELLOW) } else { Style::default().fg(theme::BLUE) }),
            Span::styled(name, style),
        ])));
    }

    app.download.dir_picker.state.select(Some(app.download.dir_picker.selected));
    frame.render_stateful_widget(List::new(list_items), chunks[1], &mut app.download.dir_picker.state);

    if entries.len() > 1 {
        let mut sc_state = ScrollbarState::new(entries.len()).position(app.download.dir_picker.selected);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .thumb_style(Style::default().fg(theme::BLUE))
                .track_style(Style::default().fg(theme::SURFACE0)),
            chunks[1],
            &mut sc_state,
        );
    }

    let hint = Line::from(vec![
        Span::styled("[Enter: ", Style::default().fg(theme::OVERLAY0)),
        Span::styled("Open Folder", Style::default().fg(theme::YELLOW)),
        Span::styled(" | Space: ", Style::default().fg(theme::OVERLAY0)),
        Span::styled("Select This Folder", Style::default().fg(theme::GREEN).add_modifier(Modifier::BOLD)),
        Span::styled(" | Esc: Cancel]", Style::default().fg(theme::OVERLAY0)),
    ]);
    frame.render_widget(Paragraph::new(hint).alignment(Alignment::Center), chunks[2]);
}
