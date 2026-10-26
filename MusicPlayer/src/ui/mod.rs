mod browser;
mod controls;
mod cover;
#[cfg(feature = "download")]
mod dir_picker;
#[cfg(feature = "download")]
mod download;
mod extensions;
#[cfg(feature = "genre")]
mod genre;
#[cfg(feature = "genre")]
mod genre_tagger;
mod help;
mod loading;
mod lyrics;
mod playlist;
mod playlist_io;
mod plugins;
mod queue;
mod scan_dialog;
mod spotlight;
mod theme_picker;
mod lyric_search;
mod visualizer;

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

#[cfg(feature = "download")]
use std::path::PathBuf;
use crate::app::{App, ViewMode};

pub fn render(frame: &mut Frame, app: &mut App) {
    let layout_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Sleek 1-line top bar (saves 2 lines for body)
            Constraint::Min(5),    // Dual Panel Body / Download View
            Constraint::Length(4), // Footer Controls
        ])
        .split(frame.area());

    if app.is_searching {
        let t = app.theme.clone();
        let search_line = Line::from(vec![
            Span::styled(" 🔍 Search: ", Style::default().fg(t.warning).add_modifier(Modifier::BOLD)),
            Span::styled(&app.search_query, Style::default().fg(t.text).add_modifier(Modifier::BOLD)),
            Span::styled("█", Style::default().fg(t.warning)),
            Span::raw("   "),
            Span::styled("[Enter: Play | Esc: Cancel]", Style::default().fg(t.faint)),
        ]);
        frame.render_widget(Paragraph::new(search_line), layout_chunks[0]);
    } else {
        let left_required = 76;
        let right_width = if app.toast_text().is_some() || !app.search_query.is_empty() {
            44.min(layout_chunks[0].width.saturating_sub(left_required))
        } else {
            34.min(layout_chunks[0].width.saturating_sub(left_required))
        };

        let top_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Min(left_required),
                Constraint::Length(right_width),
            ])
            .split(layout_chunks[0]);

        let mut left_spans = vec![
            Span::styled(" 󰎆 LYRA ", Style::default().fg(app.theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled("│ ", Style::default().fg(app.theme.border)),
        ];

        let t = app.theme.clone();
        let push_tab = |spans: &mut Vec<Span<'_>>,
                        key: &str,
                        icon: &str,
                        label: &str,
                        selected: bool| {
            if selected {
                let style = Style::default()
                    .fg(t.selected_fg)
                    .bg(t.selected_bg)
                    .add_modifier(Modifier::BOLD);
                spans.push(Span::styled(format!(" [{key}]"), style));
                if !icon.is_empty() {
                    spans.push(Span::styled(format!(" {icon}"), style));
                }
                spans.push(Span::styled(format!(" {label} "), style));
            } else {
                let key_style = Style::default()
                    .fg(t.accent_dim)
                    .add_modifier(Modifier::BOLD);
                let body_style = Style::default().fg(t.muted);
                spans.push(Span::styled(format!(" [{key}]"), key_style));
                if !icon.is_empty() {
                    spans.push(Span::styled(format!(" {icon}"), body_style));
                }
                spans.push(Span::styled(format!(" {label} "), body_style));
            }
        };

        let is_compact = layout_chunks[0].width < 96;
        let queue_label = if app.queue.is_empty() {
            "Queue".to_string()
        } else {
            format!("Queue ({})", app.queue.len())
        };
        if is_compact {
            push_tab(&mut left_spans, "1", "", "Playlist", app.view_mode == ViewMode::Playlist);
            left_spans.push(Span::raw(" "));
            push_tab(&mut left_spans, "2", "", &queue_label, app.view_mode == ViewMode::Queue);
            left_spans.push(Span::raw(" "));
            push_tab(&mut left_spans, "3", "", "Browser", app.view_mode == ViewMode::Browser);
            left_spans.push(Span::raw(" "));
            push_tab(&mut left_spans, "4", "", "Extensions", app.view_mode == ViewMode::Extensions);
            left_spans.push(Span::raw(" "));
            push_tab(&mut left_spans, "5", "", "Plugins", app.view_mode == ViewMode::Plugins);
        } else {
            push_tab(&mut left_spans, "1", "󰲸", "Playlist", app.view_mode == ViewMode::Playlist);
            left_spans.push(Span::raw(" "));
            push_tab(&mut left_spans, "2", "󰐗", &queue_label, app.view_mode == ViewMode::Queue);
            left_spans.push(Span::raw(" "));
            push_tab(&mut left_spans, "3", "󰉋", "Browser", app.view_mode == ViewMode::Browser);
            left_spans.push(Span::raw(" "));
            push_tab(&mut left_spans, "4", "󱊄", "Extensions", app.view_mode == ViewMode::Extensions);
            left_spans.push(Span::raw(" "));
            push_tab(&mut left_spans, "5", "󰏖", "Plugins", app.view_mode == ViewMode::Plugins);
        }

        frame.render_widget(Paragraph::new(Line::from(left_spans)), top_chunks[0]);

        let mut right_spans = Vec::new();
        if let Some(toast) = app.toast_text() {
            right_spans.push(Span::styled(
                format!("󰇚 {toast}  "),
                Style::default().fg(app.theme.warning).add_modifier(Modifier::BOLD),
            ));
        } else if !app.search_query.is_empty() {
            right_spans.push(Span::styled(
                format!("🔍 \"{}\"  ", app.search_query),
                Style::default().fg(app.theme.warning).add_modifier(Modifier::BOLD),
            ));
        }
        right_spans.push(Span::styled(
            "[q: Quit | /: Search | ?: Help] ",
            Style::default().fg(app.theme.faint),
        ));

        frame.render_widget(
            Paragraph::new(Line::from(right_spans)).alignment(Alignment::Right),
            top_chunks[1],
        );
    }

    if app.view_mode == ViewMode::Extensions {
        extensions::render(frame, app, layout_chunks[1]);
    } else if app.show_lyrics && app.plugin_enabled("lyrics") && app.view_mode != ViewMode::Plugins {
        // Karaoke Mode (Lyric ON): Cover art and compact playlist on the left,
        // lyrics and optional visualizer on the right (scoped to the lyrics column width).
        let body_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(24), // Left: Compact width for cover art & playlist
                Constraint::Percentage(76), // Right: Area for synchronized lyrics & visualizer
            ])
            .split(layout_chunks[1]);

        let left_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(38), // Upper: Playlist, Queue, or Browser
                Constraint::Percentage(62), // Lower: Cover Art
            ])
            .split(body_chunks[0]);

        match app.view_mode {
            ViewMode::Playlist => playlist::render(frame, app, left_chunks[0]),
            ViewMode::Queue => queue::render(frame, app, left_chunks[0]),
            ViewMode::Browser => browser::render(frame, app, left_chunks[0]),
            ViewMode::Extensions => {}
            ViewMode::Plugins => {}
        }

        cover::render(frame, app, left_chunks[1]);

        let right_area = body_chunks[1];
        if app.visualizer && right_area.height >= 14 {
            let viz_h = if right_area.height >= 30 {
                7
            } else if right_area.height >= 22 {
                6
            } else {
                5
            };
            let right_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(8),
                    Constraint::Length(viz_h),
                ])
                .split(right_area);

            lyrics::render(frame, app, right_chunks[0]);
            visualizer::render(frame, app, right_chunks[1]);
        } else {
            lyrics::render(frame, app, right_area);
        }
    } else {
        // Studio Mode (Lyric OFF): Cover Art + Spotlight specs on the left,
        // songs table + optional visualizer on the right.
        let available_h = layout_chunks[1].height;
        let available_w = layout_chunks[1].width;
        let spotlight_height = 10.min(available_h.saturating_sub(12));
        let max_left = available_w.saturating_sub(44);
        let left_w = ((available_h.saturating_sub(spotlight_height)) * 2 + 6).clamp(34, 48).min(max_left);
        let cover_h = (left_w / 2).saturating_sub(2).clamp(12, 20).min(available_h.saturating_sub(spotlight_height));

        let body_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(left_w), // Left: Cover art and spotlight
                Constraint::Min(40),        // Right: Songs table / Browser / Plugins
            ])
            .split(layout_chunks[1]);

        let left_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(cover_h),       // Upper: Cover Art
                Constraint::Min(spotlight_height), // Lower: Spotlight & Specs
            ])
            .split(body_chunks[0]);

        cover::render(frame, app, left_chunks[0]);
        spotlight::render(frame, app, left_chunks[1]);

        let right_area = body_chunks[1];
        if app.visualizer && right_area.height >= 14 {
            let viz_h = if right_area.height >= 30 {
                7
            } else if right_area.height >= 22 {
                6
            } else {
                5
            };
            let right_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(8),
                    Constraint::Length(viz_h),
                ])
                .split(right_area);

            match app.view_mode {
                ViewMode::Playlist => playlist::render(frame, app, right_chunks[0]),
                ViewMode::Queue => queue::render(frame, app, right_chunks[0]),
                ViewMode::Browser => browser::render(frame, app, right_chunks[0]),
                ViewMode::Extensions => {}
                ViewMode::Plugins => plugins::render(frame, app, right_chunks[0]),
            }
            visualizer::render(frame, app, right_chunks[1]);
        } else {
            match app.view_mode {
                ViewMode::Playlist => playlist::render(frame, app, right_area),
                ViewMode::Queue => queue::render(frame, app, right_area),
                ViewMode::Browser => browser::render(frame, app, right_area),
                ViewMode::Extensions => {}
                ViewMode::Plugins => plugins::render(frame, app, right_area),
            }
        }
    }

    controls::render(frame, app, layout_chunks[2]);

    // Popup overlays
    if app.scanner.is_scanning {
        loading::render(frame, &app.scanner, &app.theme);
    } else if let Some(ref found) = app.scanner.pending_result {
        scan_dialog::render(frame, found.len(), &app.theme);
    }

    #[cfg(feature = "genre")]
    if app.genre_picker.is_some() {
        genre::render(frame, app);
    }

    #[cfg(feature = "genre")]
    if app.genre_tagger.is_some() {
        genre_tagger::render(frame, app);
    }

    if app.show_help {
        help::render(frame, app);
    }

    if app.playlist_io_modal.is_some() {
        playlist_io::render(frame, app);
    }

    if app.theme_picker_modal.is_some() {
        theme_picker::render(frame, app);
    }

    if app.lyric_search_modal.is_some() {
        lyric_search::render(frame, app);
    }
}

pub fn post_render(app: &mut App) -> std::io::Result<()> {
    if !crate::cover::is_kitty_supported() {
        return Ok(());
    }

    let mut stdout = std::io::stdout();

    // Cover art lives either in:
    // 1. Cover Art Picker modal or Metadata review form in download view (using app.download.cover_preview_art)
    // 2. Playlist/Queue/Browser split view (using app.cover)
    #[cfg(feature = "download")]
    let is_download_cover_active = app.download.show_cover_picker_modal
        || (app.download.show_metadata_form && app.download.cover_preview_art.is_some())
        || (app.download.show_cover_file_picker && app.download.file_hover_preview.is_some());
    #[cfg(not(feature = "download"))]
    let is_download_cover_active = false;

    #[cfg(feature = "download")]
    let in_download_overlay = (app.view_mode == ViewMode::Extensions
        || app.download.show_metadata_form
        || app.download.show_dir_picker)
        && !is_download_cover_active;
    #[cfg(not(feature = "download"))]
    let in_download_overlay = false;

    let cover_visible = !in_download_overlay
        && !app.scanner.is_scanning
        && app.scanner.pending_result.is_none();
    if !cover_visible {
        if app.last_kitty_rendered.is_some() {
            crate::cover::clear_kitty_image(&mut stdout)?;
            app.last_kitty_rendered = None;
        }
        app.kitty_cover_rect = None;
        return Ok(());
    }

    #[cfg(feature = "download")]
    let (target_art, state_path) = if app.download.show_crop_modal {
        // Crop editor: live result preview (same file each move, so the
        // input handler clears the kitty cache to force a rewrite).
        (
            app.download.crop_preview.as_ref(),
            app.download.crop_source_file.clone().map(PathBuf::from),
        )
    } else if app.download.show_cover_file_picker {
        (
            app.download.file_hover_preview.as_ref(),
            app.download.file_hover_path.clone().map(PathBuf::from),
        )
    } else if is_download_cover_active {
        (
            app.download.cover_preview_art.as_ref(),
            app.download.cover_preview_path.as_ref().map(PathBuf::from),
        )
    } else {
        (app.cover.as_ref(), app.current_playing_path.clone())
    };

    #[cfg(not(feature = "download"))]
    let (target_art, state_path) = (app.cover.as_ref(), app.current_playing_path.clone());

    if let (Some(rect), Some(cover_art)) = (app.kitty_cover_rect, target_art) {
        if let Some(b64) = &cover_art.png_base64 {
            let current_state = (state_path, rect);
            if app.last_kitty_rendered.as_ref() != Some(&current_state) {
                crate::cover::clear_kitty_image(&mut stdout)?;
                crate::cover::write_kitty_image(
                    &mut stdout,
                    b64,
                    rect.x,
                    rect.y,
                    rect.width,
                    rect.height,
                )?;
                app.last_kitty_rendered = Some(current_state);
            }
            return Ok(());
        }
    }

    // No cover to display; clear any previous Kitty image
    if app.last_kitty_rendered.is_some() {
        crate::cover::clear_kitty_image(&mut stdout)?;
        app.last_kitty_rendered = None;
    }

    Ok(())
}
