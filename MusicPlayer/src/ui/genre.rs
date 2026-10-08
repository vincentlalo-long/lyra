use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};
use crate::{app::App, theme};

/// Centered genre picker: `j/k` move, `Enter` queues the whole genre,
/// `f` filters the Playlist view, `Esc` closes.
pub fn render(frame: &mut Frame, app: &mut App) {
    let Some(picker) = &app.genre_picker else {
        return;
    };

    let screen_area = frame.area();
    let total_rows = picker.genres.len() + 1;
    let rows = total_rows.min(16) as u16;
    let popup_height = (rows + 6).min(screen_area.height).max(9);
    let popup_width = 54.min(screen_area.width);

    let vertical_chunks = Layout::default()
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
        .split(vertical_chunks[1])[1];

    let mut items = Vec::new();
    for (pos, (genre, count)) in picker.genres.iter().enumerate() {
        let is_selected = pos == picker.selected;
        let is_checked = picker.is_selected(genre);
        let check_sym = if is_checked { "󰄲" } else { "󰄱" };
        let prefix = if is_selected { "▶" } else { " " };

        let style = if is_selected {
            Style::default()
                .fg(theme::YELLOW)
                .bg(theme::SURFACE0)
                .add_modifier(Modifier::BOLD)
        } else if is_checked {
            Style::default().fg(theme::GREEN).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme::TEXT)
        };

        let line = Line::from(vec![
            Span::styled(format!("{prefix} {check_sym} "), style),
            Span::styled(format!("{genre} "), style),
            Span::styled(format!("({count})"), Style::default().fg(theme::OVERLAY0)),
        ]);
        items.push(ListItem::new(line));
    }

    // Apply Filter action row at the bottom of the list
    let is_apply_selected = picker.is_apply_button_selected();
    let apply_prefix = if is_apply_selected { "▶ " } else { "  " };
    let apply_style = if is_apply_selected {
        Style::default()
            .fg(theme::GREEN)
            .bg(theme::SURFACE0)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme::BLUE).add_modifier(Modifier::BOLD)
    };
    items.push(ListItem::new(Line::from(vec![
        Span::styled(format!("{apply_prefix}󰄲 [ Apply Filter / Lọc Playlist (f) ]"), apply_style),
    ])));

    let n_checked = picker.selected_set.len();
    let title_extra = if n_checked > 0 {
        format!(" Genres ({n_checked} selected) ")
    } else {
        " Genres (Multiple Choice) ".to_string()
    };

    let block = Block::default()
        .title(title_extra)
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::MAUVE).add_modifier(Modifier::BOLD));

    let inner = block.inner(popup_area);
    frame.render_widget(Clear, popup_area);
    frame.render_widget(block, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(2)])
        .split(inner);

    frame.render_widget(List::new(items), chunks[0]);
    frame.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            " Enter/Space: Tick • f: Lọc Playlist • q: Enqueue • Esc: Đóng ",
            Style::default().fg(theme::OVERLAY0),
        )])),
        chunks[1],
    );
}
