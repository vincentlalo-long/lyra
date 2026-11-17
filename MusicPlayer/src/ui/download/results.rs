use ratatui::{
    layout::{Alignment, Margin, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};
use crate::app::App;

pub(super) fn render_input_bar(frame: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.download.is_input_active && !app.download.show_metadata_form && !app.download.show_dir_picker;

    let source_label = app.download.search_source_label();
    let mut spans = vec![
        Span::styled("󰍉 ", Style::default().fg(if is_focused { app.theme.warning } else { app.theme.info }).add_modifier(Modifier::BOLD)),
        Span::styled(
            format!("[{source_label}] "),
            Style::default().fg(app.theme.success).add_modifier(Modifier::BOLD),
        ),
    ];

    if app.download.query.is_empty() && !is_focused {
        spans.push(Span::styled(
            "Search official sources or paste URL... (s: Source, 1-5: Tab)",
            Style::default().fg(app.theme.faint),
        ));
    } else {
        spans.push(Span::styled(
            &app.download.query,
            Style::default().fg(app.theme.text).add_modifier(Modifier::BOLD),
        ));
        if is_focused {
            spans.push(Span::styled("█", Style::default().fg(app.theme.warning)));
        }
    }

    let hint = if app.download.is_searching {
        Line::from(Span::styled(
            format!(" 󰑮 Searching {source_label}... "),
            Style::default().fg(app.theme.warning).add_modifier(Modifier::BOLD),
        ))
    } else if is_focused {
        Line::from(vec![
            Span::styled("[", Style::default().fg(app.theme.faint)),
            Span::styled("Enter", Style::default().fg(app.theme.warning).add_modifier(Modifier::BOLD)),
            Span::styled(": Search | ", Style::default().fg(app.theme.text)),
            Span::styled("Ctrl+S", Style::default().fg(app.theme.warning).add_modifier(Modifier::BOLD)),
            Span::styled(": Source | ", Style::default().fg(app.theme.text)),
            Span::styled("Esc", Style::default().fg(app.theme.warning).add_modifier(Modifier::BOLD)),
            Span::styled(": Normal]", Style::default().fg(app.theme.text)),
        ])
    } else {
        Line::from(vec![
            Span::styled("[", Style::default().fg(app.theme.faint)),
            Span::styled("/ or i", Style::default().fg(app.theme.warning).add_modifier(Modifier::BOLD)),
            Span::styled(": Search | ", Style::default().fg(app.theme.text)),
            Span::styled("s", Style::default().fg(app.theme.warning).add_modifier(Modifier::BOLD)),
            Span::styled(": Source | ", Style::default().fg(app.theme.text)),
            Span::styled("1-5", Style::default().fg(app.theme.warning).add_modifier(Modifier::BOLD)),
            Span::styled(": Switch Tab | ", Style::default().fg(app.theme.text)),
            Span::styled("Tab", Style::default().fg(app.theme.warning).add_modifier(Modifier::BOLD)),
            Span::styled(": Next]", Style::default().fg(app.theme.text)),
        ])
    };

    let p = Paragraph::new(Line::from(spans)).block(
        Block::default()
            .title(" 󰇚 Music Search & Download ")
            .title(hint)
            .title_alignment(Alignment::Right)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(app.theme.border_focused)),
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
            Span::styled(" [", Style::default().fg(app.theme.faint)),
            Span::styled("Enter", Style::default().fg(app.theme.warning).add_modifier(Modifier::BOLD)),
            Span::styled(": Review | ", Style::default().fg(app.theme.text)),
            Span::styled("e", Style::default().fg(app.theme.warning).add_modifier(Modifier::BOLD)),
            Span::styled(": More | ", Style::default().fg(app.theme.text)),
            Span::styled("E", Style::default().fg(app.theme.warning).add_modifier(Modifier::BOLD)),
            Span::styled(format!(": Limit ({}) | ", app.download.search_limit), Style::default().fg(app.theme.text)),
            Span::styled("s", Style::default().fg(app.theme.warning).add_modifier(Modifier::BOLD)),
            Span::styled(": Source | ", Style::default().fg(app.theme.text)),
            Span::styled("/", Style::default().fg(app.theme.warning).add_modifier(Modifier::BOLD)),
            Span::styled(": New Search] ", Style::default().fg(app.theme.text)),
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
        .border_style(Style::default().fg(app.theme.border_focused));

    if app.download.search_results.is_empty() {
        let tips = vec![
            Line::raw(""),
            Line::from(Span::styled("  󰛩 Quick Guide:", Style::default().fg(app.theme.info).add_modifier(Modifier::BOLD))),
            Line::from(Span::styled("  • Default search uses official APIs (Spotify → Deezer → iTunes); YouTube is the fallback.", Style::default().fg(app.theme.text))),
            Line::from(Span::styled("  • Press 's' (or Ctrl+S while typing) to cycle the search source / pure YouTube.", Style::default().fg(app.theme.text))),
            Line::from(Span::styled("  • Or paste a direct URL (e.g. https://www.youtube.com/watch?v=...).", Style::default().fg(app.theme.text))),
            Line::from(Span::styled("  • Lyrics + canonical metadata are fetched from LRCLIB; select a result to review & download.", Style::default().fg(app.theme.warning))),
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
            Style::default().fg(app.theme.warning).bg(app.theme.selected_bg).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(app.theme.text)
        };

        let source_style = match item.source.as_str() {
            "spotify" => Style::default().fg(app.theme.success).add_modifier(Modifier::BOLD),
            "musicbrainz" => Style::default().fg(app.theme.hint).add_modifier(Modifier::BOLD),
            "deezer" => Style::default().fg(app.theme.info).add_modifier(Modifier::BOLD),
            "itunes" => Style::default().fg(app.theme.accent).add_modifier(Modifier::BOLD),
            _ => Style::default().fg(app.theme.faint),
        };
        let meta = if !item.album.is_empty() {
            item.album.clone()
        } else {
            item.uploader.clone()
        };
        let lyric_badge = if item.has_lyrics == Some(true) { " [LRC]" } else { "" };

        let line = Line::from(vec![
            Span::styled(prefix, Style::default().fg(app.theme.warning).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{:02}. ", i + 1), Style::default().fg(app.theme.faint)),
            Span::styled(format!("[{}] ", item.source), source_style),
            Span::styled(format!("{} - {} ", item.artist, item.title), style),
            Span::styled(format!("[{}] ", item.duration), Style::default().fg(app.theme.info)),
            Span::styled(format!("• {meta}{lyric_badge}"), Style::default().fg(app.theme.faint)),
        ]);
        list_items.push(ListItem::new(line));
    }

    app.download.result_list_state.select(Some(app.download.selected_result));
    frame.render_stateful_widget(List::new(list_items).block(block), area, &mut app.download.result_list_state);

    if items_len > 1 {
        let mut sc_state = ScrollbarState::new(items_len).position(app.download.selected_result);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .thumb_style(Style::default().fg(app.theme.accent))
                .track_style(Style::default().fg(app.theme.progress_track)),
            area.inner(Margin { vertical: 1, horizontal: 0 }),
            &mut sc_state,
        );
    }
}

