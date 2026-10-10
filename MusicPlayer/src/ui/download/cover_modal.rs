use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};
use crate::{app::App, theme};

pub fn render_cover_picker_modal(frame: &mut Frame, app: &mut App) {
    let screen_area = frame.area();
    let popup_width = 88.min(screen_area.width.saturating_sub(2));
    let popup_height = 24.min(screen_area.height.saturating_sub(2));

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

    let main_block = Block::default()
        .title(" 󰍉 Cover Art Picker & Live Preview ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::PURPLE).add_modifier(Modifier::BOLD));
    frame.render_widget(main_block, popup_area);

    let inner = popup_area.inner(Margin { vertical: 1, horizontal: 1 });
    let vert_modal = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(16),   // Top: Left & Right columns
            Constraint::Length(1), // Bottom: Hints across full width
        ])
        .split(inner);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(54), // Left: search & candidate list
            Constraint::Percentage(46), // Right: live image preview
        ])
        .split(vert_modal[0]);

    // Left Column
    let left_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Search bar
            Constraint::Min(6),    // Candidates list
        ])
        .split(cols[0]);

    // Search bar
    let is_search_focused = app.download.cover_picker_input_active;
    let search_border_style = if is_search_focused {
        Style::default().fg(theme::YELLOW).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme::SURFACE1)
    };
    let search_block = Block::default()
        .title(if is_search_focused { " 󰍉 Search (Active - Enter to search) " } else { " 󰍉 Search (/ to edit) " })
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(search_border_style);

    let mut search_spans = vec![
        Span::styled(&app.download.cover_picker_search, if is_search_focused { Style::default().fg(theme::TEXT) } else { Style::default().fg(theme::SUBTEXT0) }),
    ];
    if is_search_focused {
        search_spans.push(Span::styled("█", Style::default().fg(theme::YELLOW)));
    }
    let search_p = Paragraph::new(Line::from(search_spans)).block(search_block);
    frame.render_widget(search_p, left_rows[0]);

    // Candidates list (Item 0 is Local File option, Items 1..N are candidates)
    let total_items = 1 + app.download.cover_candidates.len();
    let selected_idx = app.download.cover_picker_selected.min(total_items.saturating_sub(1));

    let mut list_items = Vec::new();

    // 0: Local Image option
    let is_local_selected = selected_idx == 0;
    let local_style = if is_local_selected {
        Style::default().fg(theme::GREEN).bg(theme::SURFACE0).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme::TEXT)
    };

    let local_label = if let Some(path) = &app.download.form_custom_cover {
        let fname = std::path::Path::new(path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.clone());
        format!("󰉋 Local: {fname} [Chosen]")
    } else {
        "󰉋 [ Browse local image from disk... ]".to_string()
    };
    list_items.push(ListItem::new(Line::from(vec![
        Span::styled(if is_local_selected { "▶ " } else { "  " }, Style::default().fg(theme::YELLOW)),
        Span::styled(local_label, local_style),
    ])));

    // 1..N: Online candidates
    for (i, cand) in app.download.cover_candidates.iter().enumerate() {
        let is_cand_selected = selected_idx == i + 1;
        let cand_style = if is_cand_selected {
            Style::default().fg(theme::YELLOW).bg(theme::SURFACE0).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme::TEXT)
        };

        let (src_color, src_tag) = match cand.source.as_str() {
            "itunes" => (theme::BLUE, "[iTunes]"),
            "deezer" => (theme::MAUVE, "[Deezer]"),
            "caa" => (theme::PEACH, "[CAA]   "),
            "web" => (theme::GREEN, "[Web]   "),
            _ => (theme::OVERLAY0, "[Auto]  "),
        };

        let album_disp = if cand.album.is_empty() {
            &cand.title
        } else {
            &cand.album
        };
        let album_trunc: String = album_disp.chars().take(22).collect();

        list_items.push(ListItem::new(Line::from(vec![
            Span::styled(if is_cand_selected { "▶ " } else { "  " }, Style::default().fg(theme::YELLOW)),
            Span::styled(src_tag, Style::default().fg(src_color).add_modifier(Modifier::BOLD)),
            Span::styled(format!(" {album_trunc:<22} "), cand_style),
            Span::styled(format!("({:.0})", cand.score), Style::default().fg(theme::OVERLAY0)),
        ])));
    }

    let list_block = Block::default()
        .title(format!(" Candidates ({}/{}) ", if total_items > 0 { selected_idx + 1 } else { 0 }, total_items))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::BLUE));

    let mut state = ratatui::widgets::ListState::default();
    state.select(Some(selected_idx));
    frame.render_stateful_widget(List::new(list_items).block(list_block), left_rows[1], &mut state);

    // Left hints
    let left_hint = if is_search_focused {
        Line::from(vec![
            Span::styled("[Enter: ", Style::default().fg(theme::OVERLAY0)),
            Span::styled("Search", Style::default().fg(theme::YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled(" | ↓/Tab: ", Style::default().fg(theme::OVERLAY0)),
            Span::styled("Candidates", Style::default().fg(theme::BLUE)),
            Span::styled(" | Ctrl+U: ", Style::default().fg(theme::OVERLAY0)),
            Span::styled("Clear", Style::default().fg(theme::RED)),
            Span::styled(" | Esc: ", Style::default().fg(theme::OVERLAY0)),
            Span::styled("Exit Search]", Style::default().fg(theme::OVERLAY0)),
        ])
    } else {
        Line::from(vec![
            Span::styled("[↑/↓: ", Style::default().fg(theme::OVERLAY0)),
            Span::styled("Select", Style::default().fg(theme::YELLOW)),
            Span::styled(" | Enter: ", Style::default().fg(theme::OVERLAY0)),
            Span::styled("Apply", Style::default().fg(theme::GREEN).add_modifier(Modifier::BOLD)),
            Span::styled(" | /: ", Style::default().fg(theme::OVERLAY0)),
            Span::styled("Search", Style::default().fg(theme::BLUE)),
            Span::styled(" | b: ", Style::default().fg(theme::OVERLAY0)),
            Span::styled("Browse File]", Style::default().fg(theme::MAUVE)),
        ])
    };
    frame.render_widget(Paragraph::new(left_hint).alignment(Alignment::Center), vert_modal[1]);

    // Right Column: Preview Panel
    let right_block = Block::default()
        .title(" 󰋩 Live Preview ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::GREEN).add_modifier(Modifier::BOLD));
    frame.render_widget(right_block, cols[1]);

    let right_inner = cols[1].inner(Margin { vertical: 1, horizontal: 1 });
    let right_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(10),   // Image canvas
            Constraint::Length(4), // Metadata footer
        ])
        .split(right_inner);

    if let Some(art) = &app.download.cover_preview_art {
        let max_rows = right_rows[0].height;
        let max_cols = right_rows[0].width;
        let mut rows = max_rows;
        let mut cols = rows * 2;
        if cols > max_cols {
            cols = max_cols;
            rows = (cols / 2).max(1);
        }

        if rows > 0 && cols > 0 {
            let pad_top = (max_rows.saturating_sub(rows)) / 2;
            let pad_left = (max_cols.saturating_sub(cols)) / 2;
            let target_rect = ratatui::layout::Rect::new(
                right_rows[0].x + pad_left,
                right_rows[0].y + pad_top,
                cols,
                rows,
            );

            if crate::cover::is_kitty_supported() {
                let buf = frame.buffer_mut();
                for y in target_rect.y..target_rect.bottom() {
                    for x in target_rect.x..target_rect.right() {
                        if let Some(cell) = buf.cell_mut((x, y)) {
                            cell.set_skip(true);
                        }
                    }
                }
                app.kitty_cover_rect = Some(target_rect);
            } else {
                let preview_lines = art.render_halfblocks(cols, rows);
                frame.render_widget(Paragraph::new(preview_lines).alignment(Alignment::Center), right_rows[0]);
                app.kitty_cover_rect = None;
            }
        } else {
            app.kitty_cover_rect = None;
        }
    } else {
        app.kitty_cover_rect = None;
        if app.download.is_cover_loading {
            let msg = Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled("󰑮 Searching / downloading...", Style::default().fg(theme::YELLOW).add_modifier(Modifier::BOLD))),
                Line::from(Span::styled("Querying iTunes, Deezer, CAA & Web", Style::default().fg(theme::OVERLAY0))),
            ]).alignment(Alignment::Center);
            frame.render_widget(msg, right_rows[0]);
        } else {
            let msg = Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled("[ No Image Preview ]", Style::default().fg(theme::OVERLAY0))),
                Line::from(Span::styled("Select a candidate or pick a local image", Style::default().fg(theme::SUBTEXT0))),
            ]).alignment(Alignment::Center);
            frame.render_widget(msg, right_rows[0]);
        }
    }

    // Metadata footer in preview column
    let meta_lines = if selected_idx == 0 {
        if let Some(path) = &app.download.form_custom_cover {
            vec![
                Line::from(vec![
                    Span::styled("Type: ", Style::default().fg(theme::OVERLAY0)),
                    Span::styled("Local Image File", Style::default().fg(theme::GREEN).add_modifier(Modifier::BOLD)),
                ]),
                Line::from(vec![
                    Span::styled("Path: ", Style::default().fg(theme::OVERLAY0)),
                    Span::styled(path.chars().take(34).collect::<String>(), Style::default().fg(theme::TEXT)),
                ]),
            ]
        } else {
            vec![
                Line::from(vec![
                    Span::styled("Type: ", Style::default().fg(theme::OVERLAY0)),
                    Span::styled("Local File (Press Enter/b to choose)", Style::default().fg(theme::YELLOW)),
                ]),
            ]
        }
    } else {
        let cand_idx = selected_idx - 1;
        if let Some(cand) = app.download.cover_candidates.get(cand_idx) {
            vec![
                Line::from(vec![
                    Span::styled("Source: ", Style::default().fg(theme::OVERLAY0)),
                    Span::styled(&cand.source, Style::default().fg(theme::BLUE).add_modifier(Modifier::BOLD)),
                    Span::styled(format!(" (Score: {:.1})", cand.score), Style::default().fg(theme::YELLOW)),
                ]),
                Line::from(vec![
                    Span::styled("Album:  ", Style::default().fg(theme::OVERLAY0)),
                    Span::styled(cand.album.chars().take(28).collect::<String>(), Style::default().fg(theme::TEXT)),
                ]),
                Line::from(vec![
                    Span::styled("Artist: ", Style::default().fg(theme::OVERLAY0)),
                    Span::styled(cand.artist.chars().take(28).collect::<String>(), Style::default().fg(theme::SUBTEXT0)),
                ]),
            ]
        } else {
            Vec::new()
        }
    };
    frame.render_widget(Paragraph::new(meta_lines), right_rows[1]);
}

pub fn render_cover_file_picker_modal(frame: &mut Frame, app: &mut App) {
    let screen_area = frame.area();
    let popup_width = 92.min(screen_area.width.saturating_sub(2));
    let popup_height = 22.min(screen_area.height.saturating_sub(2));

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

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(55), // Left: folder + entries
            Constraint::Percentage(45), // Right: live image preview
        ])
        .split(popup_area.inner(Margin { vertical: 1, horizontal: 1 }));

    let left_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Current directory
            Constraint::Min(5),    // File / folder entries
            Constraint::Length(2), // Bottom hints
        ])
        .split(cols[0]);

    let block = Block::default()
        .title(" 󰋩 Select Local Cover Image (.jpg, .png, .webp) ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::GREEN).add_modifier(Modifier::BOLD));
    frame.render_widget(block, popup_area);

    // Current dir
    let curr_str = app.download.cover_file_picker.current_dir.to_string_lossy();
    let path_p = Paragraph::new(Line::from(vec![
        Span::styled("Folder: ", Style::default().fg(theme::OVERLAY0)),
        Span::styled(curr_str, Style::default().fg(theme::YELLOW).add_modifier(Modifier::BOLD)),
    ]));
    frame.render_widget(path_p, left_rows[0]);

    // Entries list
    let entries = &app.download.cover_file_picker.entries;
    let mut list_items = Vec::new();
    for (i, entry) in entries.iter().enumerate() {
        let is_selected = i == app.download.cover_file_picker.selected;
        let is_parent = app.download.cover_file_picker.current_dir.parent() == Some(entry.as_path());
        let is_dir = entry.is_dir();

        let name = if is_parent {
            ".. (Go up)".to_string()
        } else {
            entry.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "/".into())
        };

        let (icon, icon_style) = if is_parent {
            ("󰉍 ", Style::default().fg(theme::BLUE))
        } else if is_dir {
            ("󰉋 ", Style::default().fg(theme::BLUE))
        } else {
            ("󰋩 ", Style::default().fg(theme::GREEN))
        };

        let style = if is_selected {
            Style::default().fg(theme::GREEN).bg(theme::SURFACE0).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme::TEXT)
        };

        list_items.push(ListItem::new(Line::from(vec![
            Span::styled(icon, if is_selected { Style::default().fg(theme::YELLOW) } else { icon_style }),
            Span::styled(name, style),
        ])));
    }

    app.download.cover_file_picker.state.select(Some(app.download.cover_file_picker.selected));
    frame.render_stateful_widget(List::new(list_items), left_rows[1], &mut app.download.cover_file_picker.state);

    if entries.len() > 1 {
        let mut sc_state = ScrollbarState::new(entries.len()).position(app.download.cover_file_picker.selected);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .thumb_style(Style::default().fg(theme::GREEN))
                .track_style(Style::default().fg(theme::SURFACE0)),
            left_rows[1],
            &mut sc_state,
        );
    }

    let hint = Line::from(vec![
        Span::styled("[Enter: ", Style::default().fg(theme::OVERLAY0)),
        Span::styled("Open Folder / Select Image", Style::default().fg(theme::GREEN).add_modifier(Modifier::BOLD)),
        Span::styled(" | Esc: Cancel]", Style::default().fg(theme::OVERLAY0)),
    ]);
    frame.render_widget(Paragraph::new(hint).alignment(Alignment::Center), left_rows[2]);

    // Right: live preview of the highlighted image.
    let preview_block = Block::default()
        .title(" Preview ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::GREEN));
    let preview_inner = cols[1].inner(Margin { vertical: 1, horizontal: 1 });
    frame.render_widget(preview_block, cols[1]);

    if let Some(art) = &app.download.file_hover_preview {
        let max_rows = preview_inner.height;
        let max_cols = preview_inner.width;
        let mut rows = max_rows;
        let mut cols_n = rows * 2;
        if cols_n > max_cols {
            cols_n = max_cols;
            rows = (cols_n / 2).max(1);
        }
        if rows > 0 && cols_n > 0 {
            let pad_top = (max_rows.saturating_sub(rows)) / 2;
            let pad_left = (max_cols.saturating_sub(cols_n)) / 2;
            let target_rect = ratatui::layout::Rect::new(
                preview_inner.x + pad_left,
                preview_inner.y + pad_top,
                cols_n,
                rows,
            );
            if crate::cover::is_kitty_supported() {
                let buf = frame.buffer_mut();
                for y in target_rect.y..target_rect.bottom() {
                    for x in target_rect.x..target_rect.right() {
                        if let Some(cell) = buf.cell_mut((x, y)) {
                            cell.set_skip(true);
                        }
                    }
                }
                app.kitty_cover_rect = Some(target_rect);
            } else {
                let preview_lines = art.render_halfblocks(cols_n, rows);
                frame.render_widget(Paragraph::new(preview_lines).alignment(Alignment::Center), preview_inner);
                app.kitty_cover_rect = None;
            }
        } else {
            app.kitty_cover_rect = None;
        }
    } else {
        app.kitty_cover_rect = None;
        let msg = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled("[ No Preview ]", Style::default().fg(theme::OVERLAY0))),
            Line::from(Span::styled("Highlight an image file", Style::default().fg(theme::SUBTEXT0))),
        ])
        .alignment(Alignment::Center);
        frame.render_widget(msg, preview_inner);
    }
}
