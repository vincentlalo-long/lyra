use ratatui::{
    layout::{Margin, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};
use crate::{app::App, theme};

pub fn render(frame: &mut Frame, app: &mut App, area: Rect) {
    let filtered = app.playlist.filtered_indices(&app.search_query);
    let mut list_items = Vec::new();
    let mut selected_pos = None;

    for (pos, &song_idx) in filtered.iter().enumerate() {
        let song_path = &app.playlist.songs[song_idx];
        let song_name = match song_path.file_stem() {
            Some(name) => name.to_string_lossy(),
            None => "Unknown".into(),
        };

        let is_playing = app.playlist.playing_index == Some(song_idx);
        let is_selected = song_idx == app.playlist.selected;
        if is_selected {
            selected_pos = Some(pos);
        }

        let icon = if is_playing {
            if app.audio.is_paused { "⏸ " } else { "▶ " }
        } else {
            "  "
        };

        let num = format!("{:02}. ", pos + 1);

        // 4 distinct color states
        let (style, icon_style) = if is_selected && is_playing {
            // Cursor is hovering on the currently playing track
            (
                Style::default()
                    .fg(theme::GREEN)
                    .bg(theme::SURFACE0)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                Style::default()
                    .fg(theme::YELLOW)
                    .bg(theme::SURFACE0)
                    .add_modifier(Modifier::BOLD),
            )
        } else if is_selected {
            // Cursor is hovering on another track
            (
                Style::default()
                    .fg(theme::YELLOW)
                    .bg(theme::SURFACE0)
                    .add_modifier(Modifier::BOLD),
                Style::default().fg(theme::YELLOW),
            )
        } else if is_playing {
            // Track is playing elsewhere
            (
                Style::default().fg(theme::GREEN).add_modifier(Modifier::BOLD),
                Style::default().fg(theme::GREEN).add_modifier(Modifier::BOLD),
            )
        } else {
            // Normal idle track
            (
                Style::default().fg(theme::TEXT),
                Style::default().fg(theme::TEXT),
            )
        };

        let line_spans = vec![
            Span::styled(icon, icon_style),
            Span::styled(format!("{num}{song_name}"), style),
        ];
        list_items.push(ListItem::new(Line::from(line_spans)));
    }

    let count_label = if app.search_query.is_empty() {
        format!("{}", app.playlist.songs.len())
    } else {
        format!("{}/{}", filtered.len(), app.playlist.songs.len())
    };

    let title = format!(" 󰲸 Track List ({count_label}) ");
    let list_widget = List::new(list_items).block(
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme::BLUE)),
    );

    app.playlist.state.select(selected_pos);
    frame.render_stateful_widget(list_widget, area, &mut app.playlist.state);

    if filtered.len() > 1 {
        let mut scrollbar_state = ScrollbarState::new(filtered.len())
            .position(selected_pos.unwrap_or(0));
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None)
            .thumb_style(Style::default().fg(theme::BLUE))
            .track_style(Style::default().fg(theme::SURFACE0));
        frame.render_stateful_widget(
            scrollbar,
            area.inner(Margin { vertical: 1, horizontal: 0 }),
            &mut scrollbar_state,
        );
    }
}
