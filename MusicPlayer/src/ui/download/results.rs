use ratatui::{
    layout::{Alignment, Margin, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};
use crate::{app::App, theme};

pub(super) fn render_input_bar(frame: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.download.is_input_active && !app.download.show_metadata_form && !app.download.show_dir_picker;
    let border_color = if is_focused { theme::YELLOW } else { theme::BLUE };

    let mut spans = vec![
        Span::styled("󰍉 ", Style::default().fg(if is_focused { theme::YELLOW } else { theme::BLUE }).add_modifier(Modifier::BOLD)),
    ];

    if app.download.query.is_empty() && !is_focused {
        spans.push(Span::styled(
            "Press '/' or 'i' to search YouTube, or paste URL... (1-5: Switch Tab)",
            Style::default().fg(theme::OVERLAY0),
        ));
    } else {
        spans.push(Span::styled(
            &app.download.query,
            Style::default().fg(theme::TEXT).add_modifier(Modifier::BOLD),
        ));
        if is_focused {
            spans.push(Span::styled("█", Style::default().fg(theme::YELLOW)));
        }
    }

    let hint = if app.download.is_searching {
        Line::from(Span::styled(" 󰑮 Searching YouTube... ", Style::default().fg(theme::YELLOW).add_modifier(Modifier::BOLD)))
    } else if is_focused {
        Line::from(vec![
            Span::styled("[", Style::default().fg(theme::OVERLAY0)),
            Span::styled("Enter", Style::default().fg(theme::YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled(": Search | ", Style::default().fg(theme::TEXT)),
            Span::styled("Esc", Style::default().fg(theme::YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled(": Normal Mode (1-5 to switch tab)]", Style::default().fg(theme::TEXT)),
        ])
    } else {
        Line::from(vec![
            Span::styled("[", Style::default().fg(theme::OVERLAY0)),
            Span::styled("/ or i", Style::default().fg(theme::YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled(": Search | ", Style::default().fg(theme::TEXT)),
            Span::styled("1-5", Style::default().fg(theme::YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled(": Switch Tab | ", Style::default().fg(theme::TEXT)),
            Span::styled("Tab", Style::default().fg(theme::YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled(": Next]", Style::default().fg(theme::TEXT)),
        ])
    };

    let p = Paragraph::new(Line::from(spans)).block(
        Block::default()
            .title(" 󰇚 YouTube Search & Download ")
            .title(hint)
            .title_alignment(Alignment::Right)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color)),
    );
    frame.render_widget(p, area);
}


pub(super) fn render_search_results(frame: &mut Frame, app: &mut App, area: Rect) {
    let items_len = app.download.search_results.len();
    let title = if items_len > 0 {
        format!(" 󰄠 Search Results ({items_len}) ")
    } else {
        " 󰄠 Search Results ".to_string()
    };

    let title_right = if items_len > 0 {
        Line::from(vec![
            Span::styled(" [", Style::default().fg(theme::OVERLAY0)),
            Span::styled("Enter", Style::default().fg(theme::YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled(": Review | ", Style::default().fg(theme::TEXT)),
            Span::styled("e", Style::default().fg(theme::YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled(": More Results | ", Style::default().fg(theme::TEXT)),
            Span::styled("/", Style::default().fg(theme::YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled(": New Search] ", Style::default().fg(theme::TEXT)),
        ])
    } else {
        Line::default()
    };

    let block = Block::default()
        .title(title)
        .title(title_right)
        .title_alignment(Alignment::Right)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::MAUVE));

    if app.download.search_results.is_empty() {
        let tips = vec![
            Line::raw(""),
            Line::from(Span::styled("  󰛩 Quick Guide:", Style::default().fg(theme::BLUE).add_modifier(Modifier::BOLD))),
            Line::from(Span::styled("  • Type song title or keywords and press Enter to search YouTube.", Style::default().fg(theme::TEXT))),
            Line::from(Span::styled("  • Or paste direct URL (e.g. https://www.youtube.com/watch?v=...).", Style::default().fg(theme::TEXT))),
            Line::from(Span::styled("  • Select any result to open the Metadata & Save To review form.", Style::default().fg(theme::TEXT))),
            Line::from(Span::styled("  • Automatically downloads audio, converts to MP3, embeds 1:1 cover art and synced lyrics!", Style::default().fg(theme::YELLOW))),
        ];
        frame.render_widget(Paragraph::new(tips).block(block), area);
        return;
    }

    let items_len = app.download.search_results.len();
    let mut list_items = Vec::new();

    for (i, item) in app.download.search_results.iter().enumerate() {
        let is_selected = i == app.download.selected_result;
        let prefix = if is_selected { "▶ " } else { "  " };

        let style = if is_selected {
            Style::default().fg(theme::YELLOW).bg(theme::SURFACE0).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme::TEXT)
        };

        let line = Line::from(vec![
            Span::styled(prefix, Style::default().fg(theme::YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{:02}. ", i + 1), Style::default().fg(theme::OVERLAY0)),
            Span::styled(format!("{} - {} ", item.artist, item.title), style),
            Span::styled(format!("[{}] ", item.duration), Style::default().fg(theme::BLUE)),
            Span::styled(format!("• {}", item.uploader), Style::default().fg(theme::OVERLAY0)),
        ]);
        list_items.push(ListItem::new(line));
    }

    app.download.result_list_state.select(Some(app.download.selected_result));
    frame.render_stateful_widget(List::new(list_items).block(block), area, &mut app.download.result_list_state);

    if items_len > 1 {
        let mut sc_state = ScrollbarState::new(items_len).position(app.download.selected_result);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .thumb_style(Style::default().fg(theme::MAUVE))
                .track_style(Style::default().fg(theme::SURFACE0)),
            area.inner(Margin { vertical: 1, horizontal: 0 }),
            &mut sc_state,
        );
    }
}

