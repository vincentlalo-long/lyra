use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};
use crate::{app::App, theme};

pub fn render(frame: &mut Frame, app: &App) {
    let Some(tagger) = &app.genre_tagger else {
        return;
    };

    let screen_area = frame.area();
    let popup_width = 72.min(screen_area.width.saturating_sub(4));
    let popup_height = 13.min(screen_area.height.saturating_sub(2));

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

    let border_color = if tagger.focus_suggestions { theme::BLUE } else { theme::PEACH };
    let block = Block::default()
        .title(" 󰎆 Tag / Edit Song Genres ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color).add_modifier(Modifier::BOLD));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Song Title
            Constraint::Length(3), // Input Box
            Constraint::Length(3), // Suggestion Chips
            Constraint::Min(1),    // Hotkey Guide
        ])
        .split(inner.inner(Margin { vertical: 0, horizontal: 1 }));

    // 1. Song Title
    let title_line = Line::from(vec![
        Span::styled("󰝚 Track: ", Style::default().fg(theme::OVERLAY0)),
        Span::styled(&tagger.song_title, Style::default().fg(theme::TEXT).add_modifier(Modifier::BOLD)),
    ]);
    frame.render_widget(Paragraph::new(title_line), rows[0]);

    // 2. Input Box
    let input_focused = !tagger.focus_suggestions;
    let mut input_spans = vec![
        Span::styled("Tags: ", Style::default().fg(theme::BLUE).add_modifier(Modifier::BOLD)),
    ];
    if tagger.input.is_empty() && !input_focused {
        input_spans.push(Span::styled("Type genre tags separated by comma (e.g. Pop, Ballad)...", Style::default().fg(theme::OVERLAY0)));
    } else {
        input_spans.push(Span::styled(&tagger.input, Style::default().fg(theme::TEXT).add_modifier(Modifier::BOLD)));
        if input_focused {
            input_spans.push(Span::styled("█", Style::default().fg(theme::PEACH)));
        }
    }

    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(if input_focused { theme::PEACH } else { theme::SURFACE2 }));
    frame.render_widget(Paragraph::new(Line::from(input_spans)).block(input_block), rows[1]);

    // 3. Suggestions Carousel / Chips
    let mut chip_spans = vec![
        Span::styled("󰛩 Suggestions: ", Style::default().fg(theme::OVERLAY0)),
    ];
    let start_idx = tagger.selected_suggestion.saturating_sub(3);
    for (i, sug) in tagger.suggestions.iter().enumerate().skip(start_idx).take(7) {
        let is_cur = i == tagger.selected_suggestion;
        let style = if is_cur && tagger.focus_suggestions {
            Style::default().fg(theme::BASE).bg(theme::YELLOW).add_modifier(Modifier::BOLD)
        } else if is_cur {
            Style::default().fg(theme::YELLOW).add_modifier(Modifier::UNDERLINED)
        } else {
            Style::default().fg(theme::SUBTEXT0)
        };
        chip_spans.push(Span::styled(format!("[{sug}]"), style));
        chip_spans.push(Span::raw(" "));
    }
    frame.render_widget(Paragraph::new(Line::from(chip_spans)), rows[2]);

    // 4. Instructions
    let hint_line = if tagger.focus_suggestions {
        Line::from(vec![
            Span::styled("Suggestions: ", Style::default().fg(theme::YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled("[←/→/h/l: Select | Space/Enter: Add tag | Tab/↑: Back to text]", Style::default().fg(theme::TEXT)),
        ])
    } else {
        Line::from(vec![
            Span::styled("[Enter", Style::default().fg(theme::YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled(": Save | ", Style::default().fg(theme::OVERLAY0)),
            Span::styled("Tab/↓", Style::default().fg(theme::YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled(": Browse suggestions | ", Style::default().fg(theme::OVERLAY0)),
            Span::styled("Esc", Style::default().fg(theme::YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled(": Cancel]", Style::default().fg(theme::OVERLAY0)),
        ])
    };
    frame.render_widget(Paragraph::new(hint_line).alignment(Alignment::Center), rows[3]);
}
