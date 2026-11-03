use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};
use unicode_width::UnicodeWidthStr;
use crate::app::App;
use crate::ui::playlist::{fit_width, slice_visual_window};

/// Computes the horizontally scrolled text before/after the caret so that the
/// caret always stays visible inside `width` columns (caret glyph included).
fn active_field_window(value: &str, caret: usize, width: usize) -> (String, String) {
    if width == 0 {
        return (String::new(), String::new());
    }
    let text_width = width.saturating_sub(1);
    let caret = caret.min(value.chars().count());
    let before_text: String = value.chars().take(caret).collect();
    let caret_col = UnicodeWidthStr::width(before_text.as_str());
    let visible_before = caret_col.min(text_width);
    let start = caret_col - visible_before;
    let before = slice_visual_window(value, start, visible_before);
    let after = slice_visual_window(
        value,
        start + visible_before,
        text_width.saturating_sub(visible_before),
    );
    (before, after)
}

pub(super) fn render_metadata_form(frame: &mut Frame, app: &mut App) {
    let screen_area = frame.area();
    let has_preview_col = screen_area.width >= 82;
    let popup_width = if has_preview_col {
        90.min(screen_area.width.saturating_sub(4))
    } else {
        74.min(screen_area.width.saturating_sub(4))
    };
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
        .title(" 󰇚 Review & Edit Metadata (Mode 2) ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(app.theme.border_focused));
    frame.render_widget(block, popup_area);

    let inner = popup_area.inner(Margin { vertical: 1, horizontal: 1 });

    // Split vertically: Top = Form Fields (+ optional Preview column), Bottom = Full-width Actions & Hotkey hints
    let modal_sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(16),   // Top: Inputs & Preview
            Constraint::Length(2), // Bottom: Hotkey guide (full width, never covered by cover image!)
        ])
        .split(inner);

    let content_area = modal_sections[0];
    let footer_area = modal_sections[1];

    // Split horizontally if wide enough: Left = Form Fields, Right = Live Cover Preview Box
    let (form_area, preview_area) = if has_preview_col {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Min(52),    // Left: Form inputs
                Constraint::Length(32), // Right: Live artwork preview
            ])
            .split(content_area);
        (cols[0], Some(cols[1]))
    } else {
        (content_area, None)
    };

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Field 0: Title
            Constraint::Length(2), // Field 1: Artist
            Constraint::Length(2), // Field 2: Album
            Constraint::Length(2), // Field 3: Genre
            Constraint::Length(2), // Field 4: Save to (Directory)
            Constraint::Length(2), // Field 5: Cover Mode
            Constraint::Length(2), // Field 6: Lyrics Mode
            Constraint::Length(2), // Field 7: Submit Button
        ])
        .split(form_area);

    let dir_str = app.download.form_dir.to_string_lossy().to_string();
    // Cover mode (how the final art is processed): auto/itunes/blur_pad/center_crop.
    // Local files go through the same mode, so the picker preview matches the result.
    let mode_badge = match app.download.form_cover_mode.as_str() {
        "itunes" => "[studio]",
        "blur_pad" => "[blur]",
        "center_crop" => "[crop]",
        _ => "[auto]",
    };
    let src_badge = format!("<{}>", app.download.cover_source);
    let src_badge = if let Some((fx, fy)) = app.download.cover_focus {
        format!("<{}> ⦿({fx:.2},{fy:.2})", app.download.cover_source)
    } else {
        src_badge
    };
    let cover_str = if let Some(path) = &app.download.form_custom_cover {
        let fname = std::path::Path::new(path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.clone());
        format!("< 󰉋 Local: {fname} > {mode_badge} {src_badge}")
    } else if app.download.is_cover_loading {
        format!("󰑮 Searching covers... {mode_badge} {src_badge}")
    } else if app.download.cover_candidates.is_empty() {
        format!("< YouTube thumbnail fallback > {mode_badge} {src_badge}")
    } else {
        match app.download.selected_cover {
            None => {
                let best = &app.download.cover_candidates[0];
                format!(
                    "< Auto: {} {} ({:.0}) [{}/{}] > {mode_badge} {src_badge}",
                    best.source,
                    best.album.chars().take(16).collect::<String>(),
                    best.score,
                    1,
                    app.download.cover_candidates.len(),
                )
            }
            Some(i) => {
                let c = &app.download.cover_candidates[i.min(app.download.cover_candidates.len() - 1)];
                format!(
                    "< {} {} ({:.0}) [{}/{}] > {mode_badge} {src_badge}",
                    c.source,
                    c.album.chars().take(16).collect::<String>(),
                    c.score,
                    i + 1,
                    app.download.cover_candidates.len(),
                )
            }
        }
    };

    let lyrics_base = match app.download.form_lyrics_mode {
        0 => "< All (Manual + Auto) >".to_string(),
        1 => "< Creator Only (No Auto) >".to_string(),
        _ => "< Disabled >".to_string(),
    };
    // Selected lyric language (from probe + Up/Down cycling on field 6).
    let lyrics_lang_str = if !app.download.available_lyric_langs.is_empty() {
        let lang = &app.download.available_lyric_langs[app.download.selected_lyric_lang_idx
            .min(app.download.available_lyric_langs.len() - 1)];
        format!(" lang={lang}")
    } else {
        String::new()
    };
    // Subtitle probe badge: tells whether .lrc download is likely.
    let lyrics_str = match app.download.lyrics_probe {
        None => format!("{lyrics_base}{lyrics_lang_str} [probing LRC…]"),
        Some((true, _)) => format!("{lyrics_base}{lyrics_lang_str} [LRC ✓ manual]"),
        Some((false, true)) => format!("{lyrics_base}{lyrics_lang_str} [LRC ~ auto]"),
        Some((false, false)) => format!("{lyrics_base}{lyrics_lang_str} [no LRC]"),
    };

    let fields = [
        ("Title:   ", &app.download.form_title),
        ("Artist:  ", &app.download.form_artist),
        ("Album:   ", &app.download.form_album),
        ("Genre:   ", &app.download.form_genre),
        ("Save to: ", &dir_str),
        ("Cover:   ", &cover_str),
        ("Lyrics:  ", &lyrics_str),
    ];

    for (idx, (label, val)) in fields.iter().enumerate() {
        let is_active = app.download.form_field_idx == idx;
        let style = if is_active {
            Style::default().fg(app.theme.warning).bg(app.theme.selected_bg).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(app.theme.text)
        };

        let label_style = if is_active {
            Style::default().fg(app.theme.warning).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(app.theme.info)
        };

        // Badge printed after the value. Built first so its display width can
        // be reserved, otherwise a long value would push it off-screen.
        let badge: Option<(String, Color)> = match idx {
            1 if !app.download.existing_artists.is_empty() => {
                let count = app.download.existing_artists.len();
                let text = if let Some(i) = app.download.selected_artist_idx {
                    format!(" [󰠃 Ctrl+←/→ {}/{}]", i + 1, count)
                } else {
                    format!(" [󰠃 Ctrl+←/→ {} artists]", count)
                };
                Some((text, app.theme.accent_dim))
            }
            2 if !app.download.existing_albums.is_empty() => {
                let count = app.download.existing_albums.len();
                let text = if let Some(i) = app.download.selected_album_idx {
                    format!(" [󰀥 Ctrl+←/→ {}/{}]", i + 1, count)
                } else {
                    format!(" [󰀥 Ctrl+←/→ {} albums]", count)
                };
                Some((text, app.theme.hint))
            }
            3 if !app.download.suggested_genres.is_empty() => {
                let count = app.download.suggested_genres.len();
                let text = if let Some(i) = app.download.selected_genre_idx {
                    format!(" [󰠃 Ctrl+←/→ {}/{} (type ',' to add)]", i + 1, count)
                } else {
                    " [󰠃 Ctrl+←/→ genres (type ',' to add)]".to_string()
                };
                Some((text, app.theme.accent_dim))
            }
            4 if !has_preview_col => Some((" [󰉋 Browse]".to_string(), app.theme.success)),
            5 if !has_preview_col => Some((" [󰍉 Preview]".to_string(), app.theme.accent)),
            _ => None,
        };

        let label_width = UnicodeWidthStr::width(*label) + 2;
        let badge_width = badge
            .as_ref()
            .map(|(text, _)| UnicodeWidthStr::width(text.as_str()))
            .unwrap_or(0);
        let avail = (rows[idx].width as usize)
            .saturating_sub(label_width + 4 + badge_width);

        let mut spans = vec![
            Span::styled(format!(" {label} "), label_style),
            Span::styled("[ ", if is_active { Style::default().fg(app.theme.warning) } else { Style::default().fg(app.theme.faint) }),
        ];

        if is_active && idx < 4 {
            // Active text fields scroll horizontally so the caret is always
            // visible, even when the value is wider than the input box.
            let val_str: &str = val.as_str();
            let caret = app.download.form_cursor.min(val_str.chars().count());
            if avail > 0 {
                let (before, after) = active_field_window(val_str, caret, avail);
                spans.push(Span::styled(before, style));
                spans.push(Span::styled("█", Style::default().fg(app.theme.warning)));
                spans.push(Span::styled(after, style));
            }
        } else if val.is_empty() && idx == 3 && !is_active {
            spans.push(Span::styled("Auto/iTunes (or type: Pop, Ballad)", Style::default().fg(app.theme.faint)));
        } else if avail > 0 {
            // Inactive/long values get a trailing ellipsis instead of a hard cut.
            spans.push(Span::styled(fit_width(val.as_str(), avail), style));
        }

        spans.push(Span::styled(" ]", if is_active { Style::default().fg(app.theme.warning) } else { Style::default().fg(app.theme.faint) }));

        if let Some((text, color)) = badge {
            spans.push(Span::styled(text, Style::default().fg(color)));
        }

        frame.render_widget(Paragraph::new(Line::from(spans)), rows[idx]);
    }

    // Field 7: Submit button row
    let is_submit_active = app.download.form_field_idx == 7;
    let submit_spans = if is_submit_active {
        vec![
            Span::styled("▶ ", Style::default().fg(app.theme.success).add_modifier(Modifier::BOLD)),
            Span::styled(" [ 󰐊 Start Download ] ", Style::default().fg(app.theme.ink).bg(app.theme.success).add_modifier(Modifier::BOLD)),
            Span::styled(" ◀", Style::default().fg(app.theme.success).add_modifier(Modifier::BOLD)),
        ]
    } else {
        vec![
            Span::raw("  "),
            Span::styled("[ 󰐊 Start Download ]", Style::default().fg(app.theme.success).add_modifier(Modifier::BOLD)),
            Span::styled(" (Press Enter to start)", Style::default().fg(app.theme.faint)),
        ]
    };
    frame.render_widget(Paragraph::new(Line::from(submit_spans)).alignment(Alignment::Center), rows[7]);

    // Full-width Bottom Footer: Hotkey hints across the entire modal width (never covered by artwork!)
    let actions = match app.download.form_field_idx {
        0 => Line::from(vec![
            Span::styled("[←/→: ", Style::default().fg(app.theme.faint)),
            Span::styled("Move Cursor", Style::default().fg(app.theme.text)),
            Span::styled(" | Tab/↓: ", Style::default().fg(app.theme.faint)),
            Span::styled("Next", Style::default().fg(app.theme.text)),
            Span::styled(" | Ctrl+Enter: ", Style::default().fg(app.theme.faint)),
            Span::styled("Download", Style::default().fg(app.theme.success).add_modifier(Modifier::BOLD)),
            Span::styled(" | Esc: ", Style::default().fg(app.theme.faint)),
            Span::styled("Cancel]", Style::default().fg(app.theme.danger)),
        ]),
        1 => Line::from(vec![
            Span::styled("[←/→: ", Style::default().fg(app.theme.faint)),
            Span::styled("Move Cursor", Style::default().fg(app.theme.text)),
            Span::styled(" | Ctrl+←/→: ", Style::default().fg(app.theme.faint)),
            Span::styled(format!("Cycle Artists ({})", app.download.existing_artists.len()), Style::default().fg(app.theme.accent_dim).add_modifier(Modifier::BOLD)),
            Span::styled(" | Tab/↓: ", Style::default().fg(app.theme.faint)),
            Span::styled("Next", Style::default().fg(app.theme.text)),
            Span::styled(" | Ctrl+Enter: ", Style::default().fg(app.theme.faint)),
            Span::styled("Download", Style::default().fg(app.theme.success).add_modifier(Modifier::BOLD)),
            Span::styled(" | Esc: ", Style::default().fg(app.theme.faint)),
            Span::styled("Cancel]", Style::default().fg(app.theme.danger)),
        ]),
        2 => Line::from(vec![
            Span::styled("[←/→: ", Style::default().fg(app.theme.faint)),
            Span::styled("Move Cursor", Style::default().fg(app.theme.text)),
            Span::styled(" | Ctrl+←/→: ", Style::default().fg(app.theme.faint)),
            Span::styled(format!("Cycle Albums ({})", app.download.existing_albums.len()), Style::default().fg(app.theme.hint).add_modifier(Modifier::BOLD)),
            Span::styled(" | Tab/↓: ", Style::default().fg(app.theme.faint)),
            Span::styled("Next", Style::default().fg(app.theme.text)),
            Span::styled(" | Ctrl+Enter: ", Style::default().fg(app.theme.faint)),
            Span::styled("Download", Style::default().fg(app.theme.success).add_modifier(Modifier::BOLD)),
            Span::styled(" | Esc: ", Style::default().fg(app.theme.faint)),
            Span::styled("Cancel]", Style::default().fg(app.theme.danger)),
        ]),
        3 => Line::from(vec![
            Span::styled("[←/→: ", Style::default().fg(app.theme.faint)),
            Span::styled("Move Cursor", Style::default().fg(app.theme.text)),
            Span::styled(" | Ctrl+←/→: ", Style::default().fg(app.theme.faint)),
            Span::styled("Cycle Genre", Style::default().fg(app.theme.accent_dim).add_modifier(Modifier::BOLD)),
            Span::styled(" | , : ", Style::default().fg(app.theme.faint)),
            Span::styled("+More Genres", Style::default().fg(app.theme.warning).add_modifier(Modifier::BOLD)),
            Span::styled(" | Tab/↓: ", Style::default().fg(app.theme.faint)),
            Span::styled("Next", Style::default().fg(app.theme.text)),
            Span::styled(" | Ctrl+Enter: ", Style::default().fg(app.theme.faint)),
            Span::styled("Download", Style::default().fg(app.theme.success).add_modifier(Modifier::BOLD)),
            Span::styled(" | Esc: ", Style::default().fg(app.theme.faint)),
            Span::styled("Cancel]", Style::default().fg(app.theme.danger)),
        ]),
        5 => Line::from(vec![
            Span::styled("[←/→: ", Style::default().fg(app.theme.faint)),
            Span::styled("Cycle Art", Style::default().fg(app.theme.warning).add_modifier(Modifier::BOLD)),
            Span::styled(" | m: ", Style::default().fg(app.theme.faint)),
            Span::styled("Mode", Style::default().fg(app.theme.hint).add_modifier(Modifier::BOLD)),
            Span::styled(" | s: ", Style::default().fg(app.theme.faint)),
            Span::styled("Source", Style::default().fg(app.theme.info).add_modifier(Modifier::BOLD)),
            Span::styled(" | r: ", Style::default().fg(app.theme.faint)),
            Span::styled("Crop", Style::default().fg(app.theme.hint).add_modifier(Modifier::BOLD)),
            Span::styled(" | p: ", Style::default().fg(app.theme.faint)),
            Span::styled("Full Picker", Style::default().fg(app.theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled(" | b: ", Style::default().fg(app.theme.faint)),
            Span::styled("Browse File", Style::default().fg(app.theme.info)),
            Span::styled(" | Ctrl+Enter: ", Style::default().fg(app.theme.faint)),
            Span::styled("Download", Style::default().fg(app.theme.success).add_modifier(Modifier::BOLD)),
            Span::styled(" | Esc: ", Style::default().fg(app.theme.faint)),
            Span::styled("Cancel]", Style::default().fg(app.theme.danger)),
        ]),
        6 => {
            let has_langs = !app.download.available_lyric_langs.is_empty();
            let mut spans = vec![
                Span::styled("[←/→: ", Style::default().fg(app.theme.faint)),
                Span::styled("Cycle Mode", Style::default().fg(app.theme.warning).add_modifier(Modifier::BOLD)),
            ];
            if has_langs {
                spans.push(Span::styled(" | ↑/↓: ", Style::default().fg(app.theme.faint)));
                spans.push(Span::styled("Lang", Style::default().fg(app.theme.accent).add_modifier(Modifier::BOLD)));
            }
            spans.extend([
                Span::styled(" | Tab/↓: ", Style::default().fg(app.theme.faint)),
                Span::styled("Next", Style::default().fg(app.theme.text)),
                Span::styled(" | Ctrl+Enter: ", Style::default().fg(app.theme.faint)),
                Span::styled("Download", Style::default().fg(app.theme.success).add_modifier(Modifier::BOLD)),
                Span::styled(" | Esc: ", Style::default().fg(app.theme.faint)),
                Span::styled("Cancel]", Style::default().fg(app.theme.danger)),
            ]);
            Line::from(spans)
        }
        _ => Line::from(vec![
            Span::styled("[Tab/↑/↓: ", Style::default().fg(app.theme.faint)),
            Span::styled("Nav", Style::default().fg(app.theme.info)),
            Span::styled(" | Enter: ", Style::default().fg(app.theme.faint)),
            Span::styled("Next", Style::default().fg(app.theme.text)),
            Span::styled(" | Ctrl+Enter: ", Style::default().fg(app.theme.faint)),
            Span::styled("Download", Style::default().fg(app.theme.success).add_modifier(Modifier::BOLD)),
            Span::styled(" | Esc: ", Style::default().fg(app.theme.faint)),
            Span::styled("Cancel]", Style::default().fg(app.theme.danger)),
        ]),
    };
    frame.render_widget(Paragraph::new(actions).alignment(Alignment::Center), footer_area);

    // --- Right Column: Live Cover Preview Box ---
    if let Some(prev_area) = preview_area {
        let prev_block = Block::default()
            .title(" 󰋩 Cover Preview ")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(app.theme.border_focused));
        let prev_inner = prev_block.inner(prev_area);
        frame.render_widget(prev_block, prev_area);

        if prev_inner.width >= 4 && prev_inner.height >= 4 {
            let prev_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(6),    // Image display
                    Constraint::Length(2), // Candidate info footer
                ])
                .split(prev_inner);

            if app.download.is_cover_loading {
                app.kitty_cover_rect = None;
                let pad = (prev_chunks[0].height.saturating_sub(2)) / 2;
                let mut lines = Vec::new();
                for _ in 0..pad {
                    lines.push(Line::raw(""));
                }
                lines.push(Line::from(Span::styled("󰑮 Searching...", Style::default().fg(app.theme.warning).add_modifier(Modifier::BOLD))));
                lines.push(Line::from(Span::styled("Fetching studio art", Style::default().fg(app.theme.faint))));
                frame.render_widget(Paragraph::new(lines).alignment(Alignment::Center), prev_chunks[0]);
            } else if let Some(art) = &app.download.cover_preview_art {
                let w = prev_chunks[0].width;
                let h = prev_chunks[0].height;

                // High-resolution rendering via Kitty Graphics Protocol when supported
                if crate::cover::is_kitty_supported() {
                    for y in prev_chunks[0].top()..prev_chunks[0].bottom() {
                        for x in prev_chunks[0].left()..prev_chunks[0].right() {
                            if let Some(cell) = frame.buffer_mut().cell_mut((x, y)) {
                                cell.set_skip(true);
                            }
                        }
                    }
                    app.kitty_cover_rect = Some(prev_chunks[0]);
                } else {
                    let halfblock_lines = art.render_halfblocks(w, h);
                    frame.render_widget(Paragraph::new(halfblock_lines), prev_chunks[0]);
                    app.kitty_cover_rect = None;
                }

                let (info_text, info_color) = if let Some(cand_idx) = app.download.selected_cover {
                    if let Some(cand) = app.download.cover_candidates.get(cand_idx) {
                        (format!("{} (score {:.0}) [{}/{}]", cand.source, cand.score, cand_idx + 1, app.download.cover_candidates.len()), app.theme.success)
                    } else {
                        ("Selected candidate".to_string(), app.theme.text)
                    }
                } else if app.download.form_custom_cover.is_some() {
                    ("Custom local image".to_string(), app.theme.info)
                } else if let Some(best) = app.download.cover_candidates.first() {
                    (format!("Auto: {} (score {:.0}) [1/{}]", best.source, best.score, app.download.cover_candidates.len()), app.theme.success)
                } else {
                    ("YouTube thumbnail".to_string(), app.theme.warning)
                };

                let footer_lines = vec![
                    Line::from(Span::styled(info_text, Style::default().fg(info_color).add_modifier(Modifier::BOLD))),
                    Line::from(vec![
                        Span::styled("[←/→: ", Style::default().fg(app.theme.faint)),
                        Span::styled("Cycle", Style::default().fg(app.theme.warning)),
                        Span::styled(" | p: ", Style::default().fg(app.theme.faint)),
                        Span::styled("Picker]", Style::default().fg(app.theme.accent)),
                    ]),
                ];
                frame.render_widget(Paragraph::new(footer_lines).alignment(Alignment::Center), prev_chunks[1]);
            } else {
                app.kitty_cover_rect = None;
                let pad = (prev_chunks[0].height.saturating_sub(2)) / 2;
                let mut lines = Vec::new();
                for _ in 0..pad {
                    lines.push(Line::raw(""));
                }
                lines.push(Line::from(Span::styled("󰝚 YouTube Video Art", Style::default().fg(app.theme.faint))));
                lines.push(Line::from(Span::styled("[Press 'p' to search]", Style::default().fg(app.theme.accent))));
                frame.render_widget(Paragraph::new(lines).alignment(Alignment::Center), prev_chunks[0]);
            }
        }
    } else {
        app.kitty_cover_rect = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rendered_width(before: &str, after: &str) -> usize {
        UnicodeWidthStr::width(format!("{before}█{after}").as_str())
    }

    #[test]
    fn short_value_keeps_caret_after_text() {
        let (before, after) = active_field_window("Short", 5, 20);
        assert_eq!(before, "Short");
        assert_eq!(rendered_width(&before, &after), 20);
    }

    #[test]
    fn long_value_scrolls_to_keep_caret_visible() {
        let value = "Castle | House | NCS - Copyright Free Music";
        let caret = value.chars().count();
        let (before, after) = active_field_window(value, caret, 12);
        assert_eq!(rendered_width(&before, &after), 12);
        assert!(format!("{before}█{after}").ends_with('█'));
    }

    #[test]
    fn caret_in_middle_scrolls_window() {
        let value = "abcdefghijklmnopqrstuvwxyz";
        let (before, after) = active_field_window(value, 20, 10);
        assert_eq!(rendered_width(&before, &after), 10);
        assert_eq!(before, "lmnopqrst");
        assert!(after.is_empty());
    }

    #[test]
    fn unicode_value_uses_display_columns() {
        let value = "Ánh Đèn Sân Khấu";
        let caret = value.chars().count();
        let (before, after) = active_field_window(value, caret, 8);
        assert_eq!(rendered_width(&before, &after), 8);
    }

    #[test]
    fn zero_width_returns_empty_window() {
        let (before, after) = active_field_window("value", 2, 0);
        assert!(before.is_empty());
        assert!(after.is_empty());
    }
}
