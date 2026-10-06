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

    for (index, entry_path) in app.browser.entries.iter().enumerate() {
        let is_parent_folder = app.browser.current_dir.parent() == Some(entry_path.as_path());
        let entry_name = match entry_path.file_name() {
            Some(name) => name.to_string_lossy().to_string(),
            None => "..".to_string(),
        };

        let (icon, display_label) = if is_parent_folder {
            ("📁 ", ".. (Parent Directory)".to_string())
        } else if entry_path.is_dir() {
            ("📁 ", format!("{entry_name}/"))
        } else {
            ("🎵 ", entry_name)
        };

        let item_style = if index == app.browser.selected {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
                .bg(Color::Rgb(40, 40, 40))
        } else if entry_path.is_dir() {
            Style::default().fg(Color::Blue)
        } else {
            Style::default().fg(Color::White)
        };

        let line_text = format!("{icon}{display_label}");
        list_items.push(ListItem::new(Line::from(Span::styled(line_text, item_style))));
    }

    let current_path_display = app.browser.current_dir.to_string_lossy();
    let hidden_status = if app.browser.show_hidden { "Hidden: ON" } else { "Hidden: OFF" };
    let box_title = format!(" File Browser: {current_path_display} [{hidden_status}] ");
    let list_widget = List::new(list_items).block(
        Block::default()
            .title(box_title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow)),
    );

    frame.render_widget(list_widget, area);
}
