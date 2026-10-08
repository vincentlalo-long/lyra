use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Gauge, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};
use crate::{app::App, theme};

pub fn render(frame: &mut Frame, app: &mut App, area: Rect) {
    let vert_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Search / URL Input Box
            Constraint::Length(5),  // Active Download Progress Card (or status)
            Constraint::Min(5),     // Search Results / Guidelines
        ])
        .split(area);

    render_input_bar(frame, app, vert_chunks[0]);
    render_download_status(frame, app, vert_chunks[1]);
    render_search_results(frame, app, vert_chunks[2]);

    if app.download.show_metadata_form {
        render_metadata_form(frame, app);
    }

    if app.download.show_dir_picker {
        crate::ui::dir_picker::render(frame, app);
    }
}

fn render_input_bar(frame: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.download.is_input_active && !app.download.show_metadata_form && !app.download.show_dir_picker;
    let border_color = if is_focused { theme::YELLOW } else { theme::BLUE };

    let mut spans = vec![
        Span::styled("󰍉 ", Style::default().fg(theme::BLUE).add_modifier(Modifier::BOLD)),
    ];

    if app.download.query.is_empty() && !is_focused {
        spans.push(Span::styled(
            "Search song name (e.g. 'Noi Nay Co Anh') or paste YouTube URL...",
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
    } else {
        Line::from(vec![
            Span::styled("[", Style::default().fg(theme::OVERLAY0)),
            Span::styled("Enter", Style::default().fg(theme::YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled(": Search/Go | ", Style::default().fg(theme::TEXT)),
            Span::styled("Esc", Style::default().fg(theme::YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled(": Clear]", Style::default().fg(theme::TEXT)),
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

fn render_pipeline_spans<'a>(current_stage: &str) -> Line<'a> {
    let stages: &[(&str, &[&str])] = &[
        ("1. Connect", &["connect", "fetching"]),
        ("2. Stream", &["downloading", "stream"]),
        ("3. Convert", &["converting", "mp3"]),
        ("4. Lyrics", &["lyrics", "lrc"]),
        ("5. Cover", &["cover", "art"]),
        ("6. Tag", &["tagging", "id3", "metadata"]),
    ];

    let lower = current_stage.to_lowercase();
    let mut current_idx = 0;
    for (i, (_, keys)) in stages.iter().enumerate() {
        if keys.iter().any(|k| lower.contains(k)) {
            current_idx = i;
            break;
        }
    }

    let mut spans = vec![Span::styled("Stage: ", Style::default().fg(theme::OVERLAY0))];
    for (i, (name, _)) in stages.iter().enumerate() {
        if i > 0 {
            let sep_style = if i <= current_idx {
                Style::default().fg(theme::GREEN)
            } else {
                Style::default().fg(theme::SURFACE2)
            };
            spans.push(Span::styled("  ", sep_style));
        }

        if i < current_idx {
            spans.push(Span::styled(*name, Style::default().fg(theme::GREEN)));
        } else if i == current_idx {
            spans.push(Span::styled(
                format!("[{name}]"),
                Style::default().fg(theme::YELLOW).add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::styled(*name, Style::default().fg(theme::OVERLAY0)));
        }
    }

    Line::from(spans)
}

fn render_download_status(frame: &mut Frame, app: &App, area: Rect) {
    if app.download.is_downloading {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme::YELLOW).add_modifier(Modifier::BOLD))
            .title(" 󰑮 Downloading Track ")
            .title(Line::from(vec![
                Span::styled("[", Style::default().fg(theme::OVERLAY0)),
                Span::styled("Esc", Style::default().fg(theme::RED).add_modifier(Modifier::BOLD)),
                Span::styled(": Cancel Download] ", Style::default().fg(theme::TEXT)),
            ]))
            .title_alignment(Alignment::Right);

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Task Title & Speed/ETA
                Constraint::Length(1), // Gauge
                Constraint::Length(1), // Stage detail
            ])
            .split(inner);

        let mut header_spans = vec![
            Span::styled("Track: ", Style::default().fg(theme::OVERLAY0)),
            Span::styled(
                format!("{} - {}", app.download.active_artist, app.download.active_title),
                Style::default().fg(theme::YELLOW).add_modifier(Modifier::BOLD),
            ),
        ];

        if !app.download.speed.is_empty() {
            header_spans.push(Span::raw("   "));
            header_spans.push(Span::styled("󰛴 ", Style::default().fg(theme::GREEN)));
            header_spans.push(Span::styled(
                &app.download.speed,
                Style::default().fg(theme::GREEN).add_modifier(Modifier::BOLD),
            ));
        }

        if !app.download.eta.is_empty() {
            header_spans.push(Span::raw("   "));
            header_spans.push(Span::styled("󰔟 ETA: ", Style::default().fg(theme::MAUVE)));
            header_spans.push(Span::styled(
                &app.download.eta,
                Style::default().fg(theme::MAUVE),
            ));
        }

        frame.render_widget(Paragraph::new(Line::from(header_spans)), chunks[0]);

        // Shown value crawls slowly toward 95% while stalled (lyrics / iTunes /
        // tagging stages emit no percentages), real events snap past it.
        let pct = app.download.display_pct();
        let stalled = app.download.is_stalled();
        let (gauge_label, gauge_style) = if pct > 0 && !stalled {
            let label = if !app.download.speed.is_empty() && !app.download.eta.is_empty() {
                format!("{pct}% • {} • ETA {}", app.download.speed, app.download.eta)
            } else {
                format!("{pct}%")
            };
            (
                label,
                Style::default().fg(theme::BLUE).bg(theme::SURFACE0),
            )
        } else {
            let stage_lbl = if app.download.current_stage.is_empty() {
                "Connecting to YouTube..."
            } else {
                &app.download.current_stage
            };
            // Tilde marks the crawling estimate so it never poses as a real measurement.
            let pct_lbl = if pct > 0 { format!(" (~{pct}%)") } else { " (0%)".to_string() };
            (
                format!("󰑮 {stage_lbl}{pct_lbl}"),
                Style::default().fg(theme::YELLOW).bg(theme::SURFACE0),
            )
        };

        let gauge = Gauge::default()
            .gauge_style(gauge_style)
            .percent(pct)
            .label(Span::styled(gauge_label, Style::default().fg(theme::TEXT).add_modifier(Modifier::BOLD)));
        frame.render_widget(gauge, chunks[1]);

        let stage_line = if chunks[2].width >= 65 {
            render_pipeline_spans(&app.download.current_stage)
        } else {
            Line::from(vec![
                Span::styled("Stage: ", Style::default().fg(theme::OVERLAY0)),
                Span::styled(&app.download.current_stage, Style::default().fg(theme::TEXT).add_modifier(Modifier::BOLD)),
            ])
        };
        frame.render_widget(Paragraph::new(stage_line), chunks[2]);
    } else if let Some(ref err_msg) = app.download.last_error {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme::RED).add_modifier(Modifier::BOLD))
            .title(" ✘ Download Failed ")
            .title(Line::from(vec![
                Span::styled("[", Style::default().fg(theme::OVERLAY0)),
                Span::styled("Esc", Style::default().fg(theme::YELLOW).add_modifier(Modifier::BOLD)),
                Span::styled(": Dismiss] ", Style::default().fg(theme::TEXT)),
            ]))
            .title_alignment(Alignment::Right);

        let p = Paragraph::new(vec![
            Line::from(vec![
                Span::styled("✘ Error: ", Style::default().fg(theme::RED).add_modifier(Modifier::BOLD)),
                Span::styled(err_msg, Style::default().fg(theme::RED).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(Span::styled(
                "Check YouTube link or internet connection. Press / to search again or Esc to dismiss.",
                Style::default().fg(theme::OVERLAY0),
            )),
        ]).block(block);
        frame.render_widget(p, area);
    } else if let Some(ref done_msg) = app.download.last_completed {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme::GREEN))
            .title(" ✔ Download Complete ")
            .title(Line::from(vec![
                Span::styled("[", Style::default().fg(theme::OVERLAY0)),
                Span::styled("Tab", Style::default().fg(theme::YELLOW).add_modifier(Modifier::BOLD)),
                Span::styled(": Go to Playlist] ", Style::default().fg(theme::TEXT)),
            ]))
            .title_alignment(Alignment::Right);

        let p = Paragraph::new(vec![
            Line::from(vec![
                Span::styled("✔ Finished: ", Style::default().fg(theme::GREEN).add_modifier(Modifier::BOLD)),
                Span::styled(done_msg, Style::default().fg(theme::TEXT).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(Span::styled("Saved to library with 1:1 cover art & synced lyrics. Ready to play!", Style::default().fg(theme::GREEN))),
        ]).block(block);
        frame.render_widget(p, area);
    } else {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme::SURFACE2))
            .title(" 󰇚 Download Task ");

        let p = Paragraph::new(vec![
            Line::from(Span::styled("No active download.", Style::default().fg(theme::OVERLAY0))),
            Line::from(Span::styled("Search for songs above or paste a direct YouTube link to begin.", Style::default().fg(theme::OVERLAY0))),
        ]).block(block);
        frame.render_widget(p, area);
    }
}

fn render_search_results(frame: &mut Frame, app: &mut App, area: Rect) {
    let block = Block::default()
        .title(" 󰄠 Search Results ")
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

fn render_metadata_form(frame: &mut Frame, app: &App) {
    let screen_area = frame.area();
    let popup_width = 74.min(screen_area.width.saturating_sub(4));
    let popup_height = 21.min(screen_area.height.saturating_sub(2));

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

    let inner = popup_area.inner(Margin { vertical: 1, horizontal: 2 });
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Field 0: Title
            Constraint::Length(2), // Field 1: Artist
            Constraint::Length(2), // Field 2: Album
            Constraint::Length(2), // Field 3: Save to (Directory)
            Constraint::Length(2), // Field 4: Cover Mode
            Constraint::Length(2), // Field 5: Lyrics Mode
            Constraint::Length(2), // Field 6: Submit Button
            Constraint::Length(1), // Actions & Hotkey hints
        ])
        .split(inner);

    let dir_str = app.download.form_dir.to_string_lossy().to_string();
    let cover_str = if app.download.is_cover_loading {
        "󰑮 Searching studio covers...".to_string()
    } else if app.download.cover_candidates.is_empty() {
        "< YouTube thumbnail fallback > (no studio art found)".to_string()
    } else {
        match app.download.selected_cover {
            None => {
                let best = &app.download.cover_candidates[0];
                format!(
                    "< Auto: {} {} ({:.0}) [{}/{}] > (←/→ to pick)",
                    best.source,
                    best.album.chars().take(24).collect::<String>(),
                    best.score,
                    1,
                    app.download.cover_candidates.len(),
                )
            }
            Some(i) => {
                let c = &app.download.cover_candidates[i.min(app.download.cover_candidates.len() - 1)];
                format!(
                    "< {} {} ({:.0}) [{}/{}] > (←/→ to pick)",
                    c.source,
                    c.album.chars().take(24).collect::<String>(),
                    c.score,
                    i + 1,
                    app.download.cover_candidates.len(),
                )
            }
        }
    };
    let lyrics_str = match app.download.form_lyrics_mode {
        0 => "< All (Manual + Auto) > (Space to toggle)".to_string(),
        1 => "< Creator Only (No Auto) > (Space to toggle)".to_string(),
        _ => "< Disabled > (Space to toggle)".to_string(),
    };

    let fields = [
        ("Title:   ", &app.download.form_title),
        ("Artist:  ", &app.download.form_artist),
        ("Album:   ", &app.download.form_album),
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
            Span::styled(format!("  {label} "), label_style),
            Span::styled("[ ", if is_active { Style::default().fg(theme::YELLOW) } else { Style::default().fg(theme::OVERLAY0) }),
            Span::styled(*val, style),
        ];

        // For active text fields (Title, Artist, Album), show cursor █
        if is_active && idx < 3 {
            spans.push(Span::styled("█", Style::default().fg(theme::YELLOW)));
        }

        spans.push(Span::styled(" ]", if is_active { Style::default().fg(theme::YELLOW) } else { Style::default().fg(theme::OVERLAY0) }));

        if idx == 3 {
            spans.push(Span::styled(" [󰉋 Space/Enter: Browse]", Style::default().fg(theme::GREEN).add_modifier(Modifier::BOLD)));
        }
        if idx == 4 && !app.download.cover_candidates.is_empty() {
            if let Some(c) = app.download.selected_cover
                .and_then(|i| app.download.cover_candidates.get(i))
                .or(app.download.cover_candidates.first())
            {
                spans.push(Span::styled(
                    format!(" {} - {}", c.artist, c.title),
                    Style::default().fg(theme::OVERLAY0),
                ));
            }
        }

        frame.render_widget(Paragraph::new(Line::from(spans)), rows[idx]);
    }

    // Field 6: Submit button row
    let is_submit_active = app.download.form_field_idx == 6;
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
    frame.render_widget(Paragraph::new(Line::from(submit_spans)).alignment(Alignment::Center), rows[6]);

    // Row 7: Hotkey hints
    let actions = Line::from(vec![
        Span::styled("[Tab/↑/↓: ", Style::default().fg(theme::OVERLAY0)),
        Span::styled("Navigate", Style::default().fg(theme::BLUE)),
        Span::styled(" | Enter: ", Style::default().fg(theme::OVERLAY0)),
        Span::styled("Next/Select", Style::default().fg(theme::TEXT)),
        Span::styled(" | Ctrl+U: ", Style::default().fg(theme::OVERLAY0)),
        Span::styled("Clear", Style::default().fg(theme::YELLOW)),
        Span::styled(" | Ctrl+Enter: ", Style::default().fg(theme::OVERLAY0)),
        Span::styled("Download", Style::default().fg(theme::GREEN).add_modifier(Modifier::BOLD)),
        Span::styled(" | Esc: ", Style::default().fg(theme::OVERLAY0)),
        Span::styled("Cancel]", Style::default().fg(theme::RED)),
    ]);
    frame.render_widget(Paragraph::new(actions).alignment(Alignment::Center), rows[7]);
}
