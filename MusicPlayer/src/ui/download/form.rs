use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};
use crate::{app::App, theme};

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

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" 󰇚 Review & Edit Metadata (Mode 2) ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::YELLOW).add_modifier(Modifier::BOLD));
    frame.render_widget(block, popup_area);

    let inner = popup_area.inner(Margin { vertical: 1, horizontal: 1 });

    // Split horizontally if wide enough: Left = Form Fields, Right = Live Cover Preview Box
    let (form_area, preview_area) = if has_preview_col {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Min(52),       // Left: Form inputs
                Constraint::Length(32),    // Right: Live artwork preview
            ])
            .split(inner);
        (cols[0], Some(cols[1]))
    } else {
        (inner, None)
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
            Constraint::Min(1),    // Actions & Hotkey hints
        ])
        .split(form_area);

    let dir_str = app.download.form_dir.to_string_lossy().to_string();
    let cover_str = if let Some(path) = &app.download.form_custom_cover {
        let fname = std::path::Path::new(path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.clone());
        format!("< 󰉋 Local: {fname} >")
    } else if app.download.is_cover_loading {
        "󰑮 Searching covers...".to_string()
    } else if app.download.cover_candidates.is_empty() {
        "< YouTube thumbnail fallback >".to_string()
    } else {
        match app.download.selected_cover {
            None => {
                let best = &app.download.cover_candidates[0];
                format!(
                    "< Auto: {} {} ({:.0}) [{}/{}] >",
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
                    "< {} {} ({:.0}) [{}/{}] >",
                    c.source,
                    c.album.chars().take(16).collect::<String>(),
                    c.score,
                    i + 1,
                    app.download.cover_candidates.len(),
                )
            }
        }
    };

    let lyrics_str = match app.download.form_lyrics_mode {
        0 => "< All (Manual + Auto) >".to_string(),
        1 => "< Creator Only (No Auto) >".to_string(),
        _ => "< Disabled >".to_string(),
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
            Style::default().fg(theme::YELLOW).bg(theme::SURFACE0).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme::TEXT)
        };

        let label_style = if is_active {
            Style::default().fg(theme::YELLOW).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme::BLUE)
        };

        let mut spans = vec![
            Span::styled(format!(" {label} "), label_style),
            Span::styled("[ ", if is_active { Style::default().fg(theme::YELLOW) } else { Style::default().fg(theme::OVERLAY0) }),
        ];

        if val.is_empty() && idx == 3 && !is_active {
            spans.push(Span::styled("Auto/iTunes (or type: Pop, Ballad)", Style::default().fg(theme::OVERLAY0)));
        } else {
            spans.push(Span::styled(*val, style));
        }

        // For active text fields (Title, Artist, Album, Genre), show cursor █
        if is_active && idx < 4 {
            spans.push(Span::styled("█", Style::default().fg(theme::YELLOW)));
        }

        spans.push(Span::styled(" ]", if is_active { Style::default().fg(theme::YELLOW) } else { Style::default().fg(theme::OVERLAY0) }));

        if idx == 2 && !app.download.existing_albums.is_empty() {
            let count = app.download.existing_albums.len();
            let badge_text = if let Some(i) = app.download.selected_album_idx {
                format!(" [󰀥 ←/→ {}/{}]", i + 1, count)
            } else {
                format!(" [󰀥 ←/→ {} albums]", count)
            };
            spans.push(Span::styled(badge_text, Style::default().fg(theme::PEACH)));
        }
        if idx == 3 && !app.download.suggested_genres.is_empty() {
            let count = app.download.suggested_genres.len();
            let badge_text = if let Some(i) = app.download.selected_genre_idx {
                format!(" [󰠃 ←/→ {}/{}]", i + 1, count)
            } else {
                " [󰠃 ←/→ genres]".to_string()
            };
            spans.push(Span::styled(badge_text, Style::default().fg(theme::ROSE)));
        }

        if idx == 4 && !has_preview_col {
            spans.push(Span::styled(" [󰉋 Browse]", Style::default().fg(theme::GREEN).add_modifier(Modifier::BOLD)));
        }
        if idx == 5 && !has_preview_col {
            spans.push(Span::styled(" [󰍉 Preview]", Style::default().fg(theme::PURPLE).add_modifier(Modifier::BOLD)));
        }

        frame.render_widget(Paragraph::new(Line::from(spans)), rows[idx]);
    }

    // Field 7: Submit button row
    let is_submit_active = app.download.form_field_idx == 7;
    let submit_spans = if is_submit_active {
        vec![
            Span::styled("▶ ", Style::default().fg(theme::GREEN).add_modifier(Modifier::BOLD)),
            Span::styled(" [ 󰐊 Start Download ] ", Style::default().fg(theme::BASE).bg(theme::GREEN).add_modifier(Modifier::BOLD)),
            Span::styled(" ◀", Style::default().fg(theme::GREEN).add_modifier(Modifier::BOLD)),
        ]
    } else {
        vec![
            Span::raw("  "),
            Span::styled("[ 󰐊 Start Download ]", Style::default().fg(theme::GREEN).add_modifier(Modifier::BOLD)),
            Span::styled(" (Press Enter to start)", Style::default().fg(theme::OVERLAY0)),
        ]
    };
    frame.render_widget(Paragraph::new(Line::from(submit_spans)).alignment(Alignment::Center), rows[7]);

    // Row 8: Hotkey hints
    let actions = match app.download.form_field_idx {
        2 => Line::from(vec![
            Span::styled("[←/→: ", Style::default().fg(theme::OVERLAY0)),
            Span::styled(format!("Cycle Albums ({})", app.download.existing_albums.len()), Style::default().fg(theme::PEACH).add_modifier(Modifier::BOLD)),
            Span::styled(" | Tab/↓: ", Style::default().fg(theme::OVERLAY0)),
            Span::styled("Next", Style::default().fg(theme::TEXT)),
            Span::styled(" | Ctrl+Enter: ", Style::default().fg(theme::OVERLAY0)),
            Span::styled("Download", Style::default().fg(theme::GREEN).add_modifier(Modifier::BOLD)),
            Span::styled(" | Esc: ", Style::default().fg(theme::OVERLAY0)),
            Span::styled("Cancel]", Style::default().fg(theme::RED)),
        ]),
        3 => Line::from(vec![
            Span::styled("[←/→: ", Style::default().fg(theme::OVERLAY0)),
            Span::styled("Cycle Suggested Genres", Style::default().fg(theme::ROSE).add_modifier(Modifier::BOLD)),
            Span::styled(" | Tab/↓: ", Style::default().fg(theme::OVERLAY0)),
            Span::styled("Next", Style::default().fg(theme::TEXT)),
            Span::styled(" | Ctrl+Enter: ", Style::default().fg(theme::OVERLAY0)),
            Span::styled("Download", Style::default().fg(theme::GREEN).add_modifier(Modifier::BOLD)),
            Span::styled(" | Esc: ", Style::default().fg(theme::OVERLAY0)),
            Span::styled("Cancel]", Style::default().fg(theme::RED)),
        ]),
        5 => Line::from(vec![
            Span::styled("[←/→: ", Style::default().fg(theme::OVERLAY0)),
            Span::styled("Cycle Art", Style::default().fg(theme::YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled(" | p: ", Style::default().fg(theme::OVERLAY0)),
            Span::styled("Full Picker", Style::default().fg(theme::PURPLE).add_modifier(Modifier::BOLD)),
            Span::styled(" | b: ", Style::default().fg(theme::OVERLAY0)),
            Span::styled("Browse File", Style::default().fg(theme::BLUE)),
            Span::styled(" | Ctrl+Enter: ", Style::default().fg(theme::OVERLAY0)),
            Span::styled("Download", Style::default().fg(theme::GREEN).add_modifier(Modifier::BOLD)),
            Span::styled(" | Esc: ", Style::default().fg(theme::OVERLAY0)),
            Span::styled("Cancel]", Style::default().fg(theme::RED)),
        ]),
        _ => Line::from(vec![
            Span::styled("[Tab/↑/↓: ", Style::default().fg(theme::OVERLAY0)),
            Span::styled("Nav", Style::default().fg(theme::BLUE)),
            Span::styled(" | Enter: ", Style::default().fg(theme::OVERLAY0)),
            Span::styled("Next", Style::default().fg(theme::TEXT)),
            Span::styled(" | Ctrl+Enter: ", Style::default().fg(theme::OVERLAY0)),
            Span::styled("Download", Style::default().fg(theme::GREEN).add_modifier(Modifier::BOLD)),
            Span::styled(" | Esc: ", Style::default().fg(theme::OVERLAY0)),
            Span::styled("Cancel]", Style::default().fg(theme::RED)),
        ]),
    };
    frame.render_widget(Paragraph::new(actions).alignment(Alignment::Center), rows[8]);

    // --- Right Column: Live Cover Preview Box ---
    if let Some(prev_area) = preview_area {
        let prev_block = Block::default()
            .title(" 󰋩 Cover Preview ")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme::PURPLE).add_modifier(Modifier::BOLD));
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
                let pad = (prev_chunks[0].height.saturating_sub(2)) / 2;
                let mut lines = Vec::new();
                for _ in 0..pad {
                    lines.push(Line::raw(""));
                }
                lines.push(Line::from(Span::styled("󰑮 Searching...", Style::default().fg(theme::YELLOW).add_modifier(Modifier::BOLD))));
                lines.push(Line::from(Span::styled("Fetching studio art", Style::default().fg(theme::OVERLAY0))));
                frame.render_widget(Paragraph::new(lines).alignment(Alignment::Center), prev_chunks[0]);
            } else if let Some(art) = &app.download.cover_preview_art {
                let w = prev_chunks[0].width;
                let h = prev_chunks[0].height;
                let halfblock_lines = art.render_halfblocks(w, h);
                frame.render_widget(Paragraph::new(halfblock_lines), prev_chunks[0]);

                let (info_text, info_color) = if let Some(cand_idx) = app.download.selected_cover {
                    if let Some(cand) = app.download.cover_candidates.get(cand_idx) {
                        (format!("{} (score {:.0}) [{}/{}]", cand.source, cand.score, cand_idx + 1, app.download.cover_candidates.len()), theme::GREEN)
                    } else {
                        ("Selected candidate".to_string(), theme::TEXT)
                    }
                } else if app.download.form_custom_cover.is_some() {
                    ("Custom local image".to_string(), theme::BLUE)
                } else if let Some(best) = app.download.cover_candidates.first() {
                    (format!("Auto: {} (score {:.0}) [1/{}]", best.source, best.score, app.download.cover_candidates.len()), theme::GREEN)
                } else {
                    ("YouTube thumbnail".to_string(), theme::YELLOW)
                };

                let footer_lines = vec![
                    Line::from(Span::styled(info_text, Style::default().fg(info_color).add_modifier(Modifier::BOLD))),
                    Line::from(vec![
                        Span::styled("[←/→: ", Style::default().fg(theme::OVERLAY0)),
                        Span::styled("Cycle", Style::default().fg(theme::YELLOW)),
                        Span::styled(" | p: ", Style::default().fg(theme::OVERLAY0)),
                        Span::styled("Picker]", Style::default().fg(theme::PURPLE)),
                    ]),
                ];
                frame.render_widget(Paragraph::new(footer_lines).alignment(Alignment::Center), prev_chunks[1]);
            } else {
                let pad = (prev_chunks[0].height.saturating_sub(2)) / 2;
                let mut lines = Vec::new();
                for _ in 0..pad {
                    lines.push(Line::raw(""));
                }
                lines.push(Line::from(Span::styled("󰝚 YouTube Video Art", Style::default().fg(theme::OVERLAY0))));
                lines.push(Line::from(Span::styled("[Press 'p' to search]", Style::default().fg(theme::PURPLE))));
                frame.render_widget(Paragraph::new(lines).alignment(Alignment::Center), prev_chunks[0]);
            }
        }
    }
}
