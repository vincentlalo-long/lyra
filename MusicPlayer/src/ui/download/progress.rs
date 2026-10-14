use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Gauge, Paragraph},
    Frame,
};
use crate::app::App;

fn render_pipeline_spans<'a>(current_stage: &str, theme: &crate::theme::Theme) -> Line<'a> {
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

    let mut spans = vec![Span::styled("Stage: ", Style::default().fg(theme.faint))];
    for (i, (name, _)) in stages.iter().enumerate() {
        if i > 0 {
            let sep_style = if i <= current_idx {
                Style::default().fg(theme.success)
            } else {
                Style::default().fg(theme.faint)
            };
            spans.push(Span::styled("  ", sep_style));
        }

        if i < current_idx {
            spans.push(Span::styled(*name, Style::default().fg(theme.success)));
        } else if i == current_idx {
            spans.push(Span::styled(
                format!("[{name}]"),
                Style::default().fg(theme.warning).add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::styled(*name, Style::default().fg(theme.faint)));
        }
    }

    Line::from(spans)
}


pub(super) fn render_download_status(frame: &mut Frame, app: &App, area: Rect) {
    if app.download.is_downloading {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(app.theme.border_focused))
            .title(" 󰑮 Downloading Track ")
            .title(Line::from(vec![
                Span::styled("[", Style::default().fg(app.theme.faint)),
                Span::styled("Esc", Style::default().fg(app.theme.danger).add_modifier(Modifier::BOLD)),
                Span::styled(": Cancel Download] ", Style::default().fg(app.theme.text)),
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
            Span::styled("Track: ", Style::default().fg(app.theme.faint)),
            Span::styled(
                format!("{} - {}", app.download.active_artist, app.download.active_title),
                Style::default().fg(app.theme.warning).add_modifier(Modifier::BOLD),
            ),
        ];

        if !app.download.speed.is_empty() {
            header_spans.push(Span::raw("   "));
            header_spans.push(Span::styled("󰛴 ", Style::default().fg(app.theme.success)));
            header_spans.push(Span::styled(
                &app.download.speed,
                Style::default().fg(app.theme.success).add_modifier(Modifier::BOLD),
            ));
        }

        if !app.download.eta.is_empty() {
            header_spans.push(Span::raw("   "));
            header_spans.push(Span::styled("󰔟 ETA: ", Style::default().fg(app.theme.accent)));
            header_spans.push(Span::styled(
                &app.download.eta,
                Style::default().fg(app.theme.accent),
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
                Style::default().fg(app.theme.progress_fill).bg(app.theme.progress_track),
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
                Style::default().fg(app.theme.progress_fill).bg(app.theme.progress_track),
            )
        };

        let gauge = Gauge::default()
            .gauge_style(gauge_style)
            .percent(pct)
            .label(Span::styled(gauge_label, Style::default().fg(app.theme.text).add_modifier(Modifier::BOLD)));
        frame.render_widget(gauge, chunks[1]);

        let stage_line = if chunks[2].width >= 65 {
            render_pipeline_spans(&app.download.current_stage, &app.theme)
        } else {
            Line::from(vec![
                Span::styled("Stage: ", Style::default().fg(app.theme.faint)),
                Span::styled(&app.download.current_stage, Style::default().fg(app.theme.text).add_modifier(Modifier::BOLD)),
            ])
        };
        frame.render_widget(Paragraph::new(stage_line), chunks[2]);
    } else if let Some(ref err_msg) = app.download.last_error {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(app.theme.border_focused))
            .title(" ✘ Download Failed ")
            .title(Line::from(vec![
                Span::styled("[", Style::default().fg(app.theme.faint)),
                Span::styled("Esc", Style::default().fg(app.theme.warning).add_modifier(Modifier::BOLD)),
                Span::styled(": Dismiss] ", Style::default().fg(app.theme.text)),
            ]))
            .title_alignment(Alignment::Right);

        let p = Paragraph::new(vec![
            Line::from(vec![
                Span::styled("✘ Error: ", Style::default().fg(app.theme.danger).add_modifier(Modifier::BOLD)),
                Span::styled(err_msg, Style::default().fg(app.theme.danger).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(Span::styled(
                "Check YouTube link or internet connection. Press / to search again or Esc to dismiss.",
                Style::default().fg(app.theme.faint),
            )),
        ]).block(block);
        frame.render_widget(p, area);
    } else if let Some(ref done_msg) = app.download.last_completed {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(app.theme.border_focused))
            .title(" ✔ Download Complete ")
            .title(Line::from(vec![
                Span::styled("[", Style::default().fg(app.theme.faint)),
                Span::styled("Tab", Style::default().fg(app.theme.warning).add_modifier(Modifier::BOLD)),
                Span::styled(": Go to Playlist] ", Style::default().fg(app.theme.text)),
            ]))
            .title_alignment(Alignment::Right);

        let p = Paragraph::new(vec![
            Line::from(vec![
                Span::styled("✔ Finished: ", Style::default().fg(app.theme.success).add_modifier(Modifier::BOLD)),
                Span::styled(done_msg, Style::default().fg(app.theme.text).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(Span::styled("Saved to library with 1:1 cover art & synced lyrics. Ready to play!", Style::default().fg(app.theme.success))),
        ]).block(block);
        frame.render_widget(p, area);
    } else {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(app.theme.border_focused))
            .title(" 󰇚 Download Task ");

        let p = Paragraph::new(vec![
            Line::from(Span::styled("No active download.", Style::default().fg(app.theme.faint))),
            Line::from(Span::styled("Search for songs above or paste a direct YouTube link to begin.", Style::default().fg(app.theme.faint))),
        ]).block(block);
        frame.render_widget(p, area);
    }
}

