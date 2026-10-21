use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};
use crate::app::App;

pub fn render(frame: &mut Frame, app: &App) {
    let Some(modal) = &app.theme_picker_modal else {
        return;
    };
    let screen = frame.area();
    let themes = crate::theme::Theme::available();
    let popup_w = 52u16.min(screen.width.saturating_sub(4));
    let popup_h = (themes.len() + 4).min(16).min(screen.height.saturating_sub(4) as usize) as u16;

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
        .title("  Select Theme  ")
        .title_bottom(Line::from(vec![
            Span::styled(" ↑/k: Up  ↓/j: Down  ", Style::default().fg(app.theme.faint)),
            Span::styled("Enter: Select  Esc: Cancel", Style::default().fg(app.theme.faint)),
        ]))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(app.theme.border_focused));

    let inner = block.inner(popup_area);

    if inner.height < 2 {
        return;
    }

    let mut lines = Vec::new();
    lines.push(Line::from(vec![
        Span::styled(" Available themes: ", Style::default().fg(app.theme.warning).add_modifier(Modifier::BOLD)),
    ]));
    lines.push(Line::raw(""));

    for (i, theme_name) in themes.iter().enumerate() {
        let is_selected = i == modal.selected_idx;
        let is_current = *theme_name == app.theme_name;

        let (prefix, name_style) = if is_selected {
            (
                "▸ ",
                Style::default()
                    .fg(app.theme.accent)
                    .add_modifier(Modifier::BOLD),
            )
        } else if is_current {
            (
                "● ",
                Style::default()
                    .fg(app.theme.success)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            (
                "  ",
                Style::default().fg(app.theme.text),
            )
        };

        lines.push(Line::from(vec![
            Span::styled(prefix, name_style),
            Span::styled(theme_name.to_string(), if is_selected {
                Style::default().fg(app.theme.warning).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(app.theme.text)
            }),
        ]));
    }

    lines.push(Line::raw(""));
    lines.push(Line::from(vec![
        Span::styled("Current: ", Style::default().fg(app.theme.faint)),
        Span::styled(app.theme_name.as_str(), Style::default().fg(app.theme.info).add_modifier(Modifier::BOLD)),
    ]));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .style(Style::default().bg(app.theme.selected_bg));
    frame.render_widget(paragraph, popup_area);
}
