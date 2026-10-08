use ratatui::{
    layout::{Margin, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};
use crate::{app::App, theme};

pub fn render(frame: &mut Frame, app: &mut App, area: Rect) {
    let filtered = app.browser.filtered_indices(&app.search_query);
    let mut list_items = Vec::new();
    let mut selected_pos = None;

    let playing_path = app.current_playing_path.as_ref();

    for (pos, &entry_idx) in filtered.iter().enumerate() {
        let entry_path = &app.browser.entries[entry_idx];
        let is_parent = app.browser.current_dir.parent() == Some(entry_path.as_path());
        let is_playing = playing_path == Some(entry_path);
        let is_selected = entry_idx == app.browser.selected;
        if is_selected {
            selected_pos = Some(pos);
        }

        let entry_name = match entry_path.file_name() {
            Some(name) => name.to_string_lossy().to_string(),
            None => "..".to_string(),
        };

        let is_playlist = entry_path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("m3u") || e.eq_ignore_ascii_case("m3u8"));

        let (icon, display_label) = if is_parent {
            ("󰉋 ", ".. (Parent Directory)".to_string())
        } else if entry_path.is_dir() {
            ("󰉋 ", format!("{entry_name}/"))
        } else if is_playing {
            (if app.audio.is_paused { "󰏤 " } else { "▶ " }, entry_name)
        } else if is_playlist {
            ("󰲸 ", format!("{entry_name} [Playlist]"))
        } else {
            ("󰎆 ", entry_name)
        };

        let style = if is_selected && is_playing {
            Style::default()
                .fg(theme::GREEN)
                .bg(theme::SURFACE0)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        } else if is_selected {
            Style::default()
                .fg(theme::YELLOW)
                .bg(theme::SURFACE0)
                .add_modifier(Modifier::BOLD)
        } else if is_playing {
            Style::default().fg(theme::GREEN).add_modifier(Modifier::BOLD)
        } else if entry_path.is_dir() {
            Style::default().fg(theme::BLUE)
        } else if is_playlist {
            Style::default().fg(theme::MAUVE).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme::TEXT)
        };

        let prefix = if is_selected { "❯ " } else { "  " };
        let line_text = format!("{prefix}{icon}{display_label}");
        list_items.push(ListItem::new(Line::from(Span::styled(line_text, style))));
    }

    let home = std::env::var("HOME").unwrap_or_default();
    let cur_str = app.browser.current_dir.to_string_lossy();
    let display_path = if !home.is_empty() && cur_str.starts_with(&home) {
        format!("~{}", &cur_str[home.len()..])
    } else {
        cur_str.to_string()
    };

    let hidden_icon = if app.browser.show_hidden { "󰈈" } else { "󰈉" };
    let count_info = format!("({} items)", filtered.len());
    let title = format!(" 󰉋 {display_path} {count_info} [{hidden_icon}] ");

    let block = Block::default()
        .title(title)
        .title_bottom(Line::from(vec![
            Span::styled(" [Enter/l: Open Folder / Add to Playlist | Backspace/h: Up | i: Import | a: Queue | P: Load Folder] ", Style::default().fg(theme::PEACH).add_modifier(Modifier::BOLD)),
        ]))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::MAUVE));

    let list_widget = List::new(list_items).block(block);

    app.browser.state.select(selected_pos);
    frame.render_stateful_widget(list_widget, area, &mut app.browser.state);

    if filtered.len() > 1 {
        let mut scrollbar_state = ScrollbarState::new(filtered.len())
            .position(selected_pos.unwrap_or(0));
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None)
            .thumb_style(Style::default().fg(theme::MAUVE))
            .track_style(Style::default().fg(theme::SURFACE0));
        frame.render_stateful_widget(
            scrollbar,
            area.inner(Margin { vertical: 1, horizontal: 0 }),
            &mut scrollbar_state,
        );
    }
}
