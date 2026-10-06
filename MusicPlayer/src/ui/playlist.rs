use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};
use crate::app::App;

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let mut list_items = Vec::new();

    for (index, song_path) in app.playlist.songs.iter().enumerate() {
        let song_name = match song_path.file_name() {
            Some(name) => name.to_string_lossy(),
            None => "Unknown".into(),
        };

        let is_currently_playing = app.playlist.playing_index == Some(index);
        let prefix_icon = if is_currently_playing {
            if app.audio.is_paused { "⏸ " } else { "▶ " }
        } else {
            "   "
        };

        let item_style = if index == app.playlist.selected {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
                .bg(Color::Rgb(40, 40, 40))
        } else if is_currently_playing {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::White)
        };

        let line_text = format!("{prefix_icon}{song_name}");
        list_items.push(ListItem::new(Line::from(Span::styled(line_text, item_style))));
    }

    let box_title = format!(" Track List ({}) ", app.playlist.songs.len());
    let list_widget = List::new(list_items).block(
        Block::default()
            .title(box_title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );

    frame.render_widget(list_widget, area);
}
