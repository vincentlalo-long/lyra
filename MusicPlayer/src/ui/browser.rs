use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};
use crate::{
    app::{App, ViewMode},
    browser::BrowserActionModal,
};

/// Splits an input around the caret so the `█` block renders at the
/// real editing position instead of always at end-of-text.
fn caret_spans(text: &str, caret: usize, text_style: Style, cursor_style: Style) -> Vec<Span<'_>> {
    let caret = caret.min(text.chars().count());
    let before: String = text.chars().take(caret).collect();
    let after: String = text.chars().skip(caret).collect();
    vec![
        Span::styled(before, text_style),
        Span::styled("█", cursor_style),
        Span::styled(after, text_style),
    ]
}

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
                .fg(app.theme.success)
                .bg(app.theme.selected_bg)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        } else if is_selected {
            Style::default()
                .fg(app.theme.warning)
                .bg(app.theme.selected_bg)
                .add_modifier(Modifier::BOLD)
        } else if is_playing {
            Style::default().fg(app.theme.success).add_modifier(Modifier::BOLD)
        } else if entry_path.is_dir() {
            Style::default().fg(app.theme.info)
        } else if is_playlist {
            Style::default().fg(app.theme.accent).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(app.theme.text)
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

    let border_color = if app.view_mode == ViewMode::Browser {
        app.theme.border_focused
    } else {
        app.theme.border
    };
        let block = Block::default()
            .title(title)
            .title_bottom(Line::from(vec![
                Span::styled(" [Enter: Open/Play | N/+: New Album | d: Delete | m: Move | e: Edit | B: Cover | i: Import | L: Set Library] ", Style::default().fg(app.theme.hint).add_modifier(Modifier::BOLD)),
            ]))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color));

        // Publish row rects for mouse click detection.
        let inner_area = block.inner(area);
        app.browser_row_rects.clear();
        for (pos, _) in filtered.iter().enumerate() {
            let row = inner_area.y.saturating_add(pos as u16);
            app.browser_row_rects.push((
                row,
                Rect {
                    x: inner_area.x,
                    y: row,
                    width: inner_area.width,
                    height: 1,
                },
                pos,
            ));
        }

        let list_widget = List::new(list_items).block(block);

    app.browser.state.select(selected_pos);
    frame.render_stateful_widget(list_widget, area, &mut app.browser.state);

    if filtered.len() > 1 {
        let mut scrollbar_state = ScrollbarState::new(filtered.len())
            .position(selected_pos.unwrap_or(0));
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None)
            .thumb_style(Style::default().fg(app.theme.accent))
            .track_style(Style::default().fg(app.theme.progress_track));
        frame.render_stateful_widget(
            scrollbar,
            area.inner(Margin { vertical: 1, horizontal: 0 }),
            &mut scrollbar_state,
        );
    }

    // Render interactive modal if active
    if let Some(modal) = app.browser_modal.clone() {
        render_browser_modal(frame, &modal, app.browser_caret, &app.theme);
    }
}

fn render_browser_modal(
    frame: &mut Frame,
    modal: &BrowserActionModal,
    caret: usize,
    theme: &crate::theme::Theme,
) {
    let screen = frame.area();

    match modal {
        BrowserActionModal::NewAlbum { input } => {
            let popup_w = 60.min(screen.width.saturating_sub(4));
            let popup_h = 7.min(screen.height.saturating_sub(2));
            let popup_area = center_popup(screen, popup_w, popup_h);

            frame.render_widget(Clear, popup_area);

            let block = Block::default()
                .title(" 󰉋 Create New Album / Folder ")
                .title_bottom(Line::from(vec![
                    Span::styled(" [Enter: Create | Esc: Cancel] ", Style::default().fg(theme.faint)),
                ]))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.border_focused).add_modifier(Modifier::BOLD));

            let inner = block.inner(popup_area);
            frame.render_widget(block, popup_area);

            let mut input_spans = vec![
                Span::raw("  [ "),
            ];
            input_spans.extend(caret_spans(
                input,
                caret,
                Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
                Style::default().fg(theme.success),
            ));
            input_spans.push(Span::raw(" ]"));
            let lines = vec![
                Line::from(vec![
                    Span::styled(" Enter album name: ", Style::default().fg(theme.warning).add_modifier(Modifier::BOLD)),
                ]),                Line::raw(""),
                Line::from(input_spans),
            ];
            frame.render_widget(Paragraph::new(lines), inner);
        }
        BrowserActionModal::RenameAlbum { target_dir, input } => {
            let popup_w = 60.min(screen.width.saturating_sub(4));
            let popup_h = 8.min(screen.height.saturating_sub(2));
            let popup_area = center_popup(screen, popup_w, popup_h);

            frame.render_widget(Clear, popup_area);

            let old_name = target_dir.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            let block = Block::default()
                .title(format!(" 󰏫 Rename Album: {old_name} "))
                .title_bottom(Line::from(vec![
                    Span::styled(" [Enter: Save | Esc: Cancel] ", Style::default().fg(theme.faint)),
                ]))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.border_focused).add_modifier(Modifier::BOLD));

            let inner = block.inner(popup_area);
            frame.render_widget(block, popup_area);

            let mut input_spans = vec![
                Span::raw("  [ "),
            ];
            input_spans.extend(caret_spans(
                input,
                caret,
                Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
                Style::default().fg(theme.hint),
            ));
            input_spans.push(Span::raw(" ]"));
            let lines = vec![
                Line::from(vec![
                    Span::styled(" New album name: ", Style::default().fg(theme.warning).add_modifier(Modifier::BOLD)),
                ]),
                Line::raw(""),
                Line::from(input_spans),
            ];
            frame.render_widget(Paragraph::new(lines), inner);
        }
        BrowserActionModal::EditTrack {
            target_file,
            field_idx,
            title,
            artist,
            album,
            file_name,
        } => {
            let popup_w = 70.min(screen.width.saturating_sub(4));
            let popup_h = 15.min(screen.height.saturating_sub(2));
            let popup_area = center_popup(screen, popup_w, popup_h);

            frame.render_widget(Clear, popup_area);

            let fname = target_file.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            let block = Block::default()
                .title(format!(" 󰎆 Edit Track Metadata: {fname} "))
                .title_bottom(Line::from(vec![
                    Span::styled(" [Tab/↑/↓: Nav | Enter: Next/Save+Rename | Esc: Cancel] ", Style::default().fg(theme.faint)),
                ]))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.border_focused).add_modifier(Modifier::BOLD));

            let inner = block.inner(popup_area);
            frame.render_widget(block, popup_area);

            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(2), // Field 0: Title
                    Constraint::Length(2), // Field 1: Artist
                    Constraint::Length(2), // Field 2: Album
                    Constraint::Length(2), // Field 3: File name
                    Constraint::Min(1),    // Submit hint
                ])
                .split(inner);

            let fields = [
                ("Title:  ", title),
                ("Artist: ", artist),
                ("Album:  ", album),
                ("File:   ", file_name),
            ];

            for (idx, (label, val)) in fields.iter().enumerate() {
                let is_active = *field_idx == idx;
                let mut spans = vec![
                    Span::styled(format!(" {label} "), if is_active { Style::default().fg(theme.warning).add_modifier(Modifier::BOLD) } else { Style::default().fg(theme.info) }),
                    Span::styled("[ ", if is_active { Style::default().fg(theme.warning) } else { Style::default().fg(theme.faint) }),
                ];
                if is_active {
                    spans.extend(caret_spans(
                        val.as_str(),
                        caret,
                        Style::default().fg(theme.warning).add_modifier(Modifier::BOLD),
                        Style::default().fg(theme.warning),
                    ));
                } else {
                    spans.push(Span::styled(*val, Style::default().fg(theme.text)));
                }
                spans.push(Span::styled(" ]", if is_active { Style::default().fg(theme.warning) } else { Style::default().fg(theme.faint) }));
                frame.render_widget(Paragraph::new(Line::from(spans)), rows[idx]);
            }

            let save_btn = Line::from(vec![
                Span::styled("  [ 󰐊 Press Enter on File to Save ID3 Tags + Rename ]", Style::default().fg(theme.success).add_modifier(Modifier::BOLD)),
            ]);
            frame.render_widget(Paragraph::new(save_btn).alignment(Alignment::Center), rows[4]);
        }
        BrowserActionModal::MoveTrack {
            target_file,
            candidate_albums,
            selected_idx,
            creating_new,
            new_album_input,
        } => {
            let popup_w = 74.min(screen.width.saturating_sub(4));
            let popup_h = 16.min(screen.height.saturating_sub(2));
            let popup_area = center_popup(screen, popup_w, popup_h);

            frame.render_widget(Clear, popup_area);

            let fname = target_file.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            let block = Block::default()
                .title(format!(" 󰒭 Move Track to Album: {fname} "))
                .title_bottom(Line::from(vec![
                    Span::styled(" [↑/↓: Select | Enter: Move | N: Create New Album | Esc: Cancel] ", Style::default().fg(theme.faint)),
                ]))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.border_focused).add_modifier(Modifier::BOLD));

            let inner = block.inner(popup_area);
            frame.render_widget(block, popup_area);

            if *creating_new {
                let mut input_spans = vec![
                    Span::raw("   [ "),
                ];
                input_spans.extend(caret_spans(
                    new_album_input,
                    caret,
                    Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
                    Style::default().fg(theme.success),
                ));
                input_spans.push(Span::raw(" ]"));
                let lines = vec![
                    Line::raw(""),
                    Line::from(vec![
                        Span::styled("  󰉋 Create New Album & Move Track: ", Style::default().fg(theme.warning).add_modifier(Modifier::BOLD)),
                    ]),
                    Line::raw(""),
                    Line::from(input_spans),
                    Line::raw(""),
                    Line::from(vec![
                        Span::styled("   (Press Enter to create & move track, Esc to go back)", Style::default().fg(theme.faint)),
                    ]),
                ];
                frame.render_widget(Paragraph::new(lines), inner);
            } else {
                let mut list_items = Vec::new();

                for (idx, alb_dir) in candidate_albums.iter().enumerate() {
                    let is_sel = *selected_idx == idx;
                    let name = alb_dir.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                    let prefix = if is_sel { "▶ " } else { "  " };
                    let style = if is_sel {
                        Style::default().fg(theme.warning).bg(theme.selected_bg).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(theme.text)
                    };
                    list_items.push(ListItem::new(Line::from(vec![
                        Span::styled(prefix, Style::default().fg(theme.warning)),
                        Span::styled("󰉋 ", Style::default().fg(theme.info)),
                        Span::styled(name, style),
                    ])));
                }

                // Last option: Create New Album
                let is_new_sel = *selected_idx == candidate_albums.len();
                let new_prefix = if is_new_sel { "▶ " } else { "  " };
                let new_style = if is_new_sel {
                    Style::default().fg(theme.success).bg(theme.selected_bg).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.success)
                };
                list_items.push(ListItem::new(Line::from(vec![
                    Span::styled(new_prefix, Style::default().fg(theme.warning)),
                    Span::styled("󰐊 ", Style::default().fg(theme.success)),
                    Span::styled("[+ Create New Album & Move Here...]", new_style),
                ])));

                frame.render_widget(List::new(list_items), inner);
            }
        }
        BrowserActionModal::SetCover { target_file, picker } => {
            let popup_w = 70.min(screen.width.saturating_sub(4));
            let popup_h = 18.min(screen.height.saturating_sub(2));
            let popup_area = center_popup(screen, popup_w, popup_h);

            frame.render_widget(Clear, popup_area);

            let fname = target_file.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            let block = Block::default()
                .title(format!(" 󰋩 Set Cover Art: {fname} "))
                .title_bottom(Line::from(vec![
                    Span::styled(" [↑/↓: Select | Enter: Open/Apply | ←: Parent | Esc: Cancel] ", Style::default().fg(theme.faint)),
                ]))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.border_focused).add_modifier(Modifier::BOLD));

            let inner = block.inner(popup_area);
            frame.render_widget(block, popup_area);

            let cur = picker.current_dir.to_string_lossy().to_string();
            let mut list_items = vec![ListItem::new(Line::from(vec![
                Span::styled(format!(" 󰉋 {cur}"), Style::default().fg(theme.info)),
            ]))];
            for (idx, p) in picker.entries.iter().enumerate() {
                let is_sel = picker.selected == idx;
                let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                let prefix = if is_sel { "▶ " } else { "  " };
                let (icon, style) = if p.is_dir() {
                    ("󰉋 ", Style::default().fg(theme.info))
                } else if is_sel {
                    ("󰋩 ", Style::default().fg(theme.warning).bg(theme.selected_bg).add_modifier(Modifier::BOLD))
                } else {
                    ("󰋩 ", Style::default().fg(theme.text))
                };
                list_items.push(ListItem::new(Line::from(vec![
                    Span::styled(prefix, Style::default().fg(theme.warning)),
                    Span::styled(icon, Style::default().fg(theme.info)),
                    Span::styled(name, style),
                ])));
            }
            frame.render_widget(List::new(list_items), inner);
        }
    }
}

fn center_popup(screen: Rect, width: u16, height: u16) -> Rect {
    let vert = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((screen.height.saturating_sub(height)) / 2),
            Constraint::Length(height),
            Constraint::Min(0),
        ])
        .split(screen);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length((screen.width.saturating_sub(width)) / 2),
            Constraint::Length(width),
            Constraint::Min(0),
        ])
        .split(vert[1])[1]
}
