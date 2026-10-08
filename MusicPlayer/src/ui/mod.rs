mod browser;
mod controls;
mod cover;
#[cfg(feature = "download")]
mod dir_picker;
#[cfg(feature = "download")]
mod download;
#[cfg(feature = "genre")]
mod genre;
#[cfg(feature = "genre")]
mod genre_tagger;
mod help;
mod loading;
mod lyrics;
mod playlist;
mod plugins;
mod queue;
mod scan_dialog;
mod spotlight;

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

#[cfg(feature = "download")]
use std::path::PathBuf;
use crate::{
    app::{App, ViewMode},
    theme,
};

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
        let search_line = Line::from(vec![
            Span::styled(" 🔍 Search: ", Style::default().fg(theme::YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled(&app.search_query, Style::default().fg(theme::TEXT).add_modifier(Modifier::BOLD)),
            Span::styled("█", Style::default().fg(theme::YELLOW)),
            Span::raw("   "),
            Span::styled("[Enter: Play | Esc: Cancel]", Style::default().fg(theme::OVERLAY0)),
        ]);
        frame.render_widget(Paragraph::new(search_line), layout_chunks[0]);
    } else {
        let right_width = if app.toast_text().is_some() || !app.search_query.is_empty() {
            50.min(layout_chunks[0].width.saturating_sub(48))
        } else {
            36.min(layout_chunks[0].width.saturating_sub(48))
        };

        let top_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Min(48),
                Constraint::Length(right_width),
            ])
            .split(layout_chunks[0]);

        let mut left_spans = vec![
            Span::styled(" 󰎆 LYRA ", Style::default().fg(theme::BLUE).add_modifier(Modifier::BOLD)),
            Span::styled("│ ", Style::default().fg(theme::SURFACE1)),
        ];

        let playlist_tab_style = if app.view_mode == ViewMode::Playlist {
            Style::default().fg(theme::MAUVE).bg(theme::SURFACE0).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme::OVERLAY0)
        };
        let queue_tab_style = if app.view_mode == ViewMode::Queue {
            Style::default().fg(theme::MAUVE).bg(theme::SURFACE0).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme::OVERLAY0)
        };
        let browser_tab_style = if app.view_mode == ViewMode::Browser {
            Style::default().fg(theme::MAUVE).bg(theme::SURFACE0).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme::OVERLAY0)
        };

        left_spans.push(Span::styled(" [1] 󰲸 Playlist ", playlist_tab_style));
        left_spans.push(Span::raw(" "));
        left_spans.push(Span::styled(" [2] 󰐗 Queue ", queue_tab_style));
        left_spans.push(Span::raw(" "));
        left_spans.push(Span::styled(" [3] 󰉋 Browser ", browser_tab_style));

        #[cfg(feature = "download")]
        {
            let download_tab_style = if app.view_mode == ViewMode::Download {
                Style::default().fg(theme::MAUVE).bg(theme::SURFACE0).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme::OVERLAY0)
            };
            left_spans.push(Span::raw(" "));
            left_spans.push(Span::styled(" [4] 󰇚 Download ", download_tab_style));
        }

        left_spans.push(Span::raw(" "));
        let plugin_tab_num = if cfg!(feature = "download") { "5" } else { "4" };
        let plugins_tab_style = if app.view_mode == ViewMode::Plugins {
            Style::default().fg(theme::MAUVE).bg(theme::SURFACE0).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme::OVERLAY0)
        };
        left_spans.push(Span::styled(format!(" [{plugin_tab_num}] 󰏖 Plugins "), plugins_tab_style));

        frame.render_widget(Paragraph::new(Line::from(left_spans)), top_chunks[0]);

        let mut right_spans = Vec::new();
        if let Some(toast) = app.toast_text() {
            right_spans.push(Span::styled(
                format!("󰇚 {toast}  "),
                Style::default().fg(theme::YELLOW).add_modifier(Modifier::BOLD),
            ));
        } else if !app.search_query.is_empty() {
            right_spans.push(Span::styled(
                format!("🔍 \"{}\"  ", app.search_query),
                Style::default().fg(theme::YELLOW).add_modifier(Modifier::BOLD),
            ));
        }
        right_spans.push(Span::styled(
            "[q: Quit | /: Search | ?: Help] ",
            Style::default().fg(theme::OVERLAY0),
        ));

        frame.render_widget(
            Paragraph::new(Line::from(right_spans)).alignment(Alignment::Right),
            top_chunks[1],
        );
    }

    // Set when the `download` plugin is compiled in and its view is active.
    #[cfg(feature = "download")]
    let in_download_view = app.view_mode == ViewMode::Download;
    #[cfg(not(feature = "download"))]
    let in_download_view = false;

    if in_download_view {
        #[cfg(feature = "download")]
        download::render(frame, app, layout_chunks[1]);
    } else if app.show_lyrics {
        // Karaoke Mode (Lyric ON): Cover Art hẹp lại (giảm chiều ngang) và cao hơn
        let body_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(24), // Left: Giảm chiều ngang xuống 24%
                Constraint::Percentage(76), // Right: Tăng độ rộng cho Lyrics Karaoke
            ])
            .split(layout_chunks[1]);

        let left_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(38), // Upper: Playlist or Browser
                Constraint::Percentage(62), // Lower: Tăng chiều cao Cover Art (62%)
            ])
            .split(body_chunks[0]);

        match app.view_mode {
            ViewMode::Playlist => playlist::render(frame, app, left_chunks[0]),
            ViewMode::Queue => queue::render(frame, app, left_chunks[0]),
            ViewMode::Browser => browser::render(frame, app, left_chunks[0]),
            #[cfg(feature = "download")]
            ViewMode::Download => {}
            ViewMode::Plugins => plugins::render(frame, app, left_chunks[0]),
        }

        cover::render(frame, app, left_chunks[1]);
        lyrics::render(frame, app, body_chunks[1]);
    } else {
        // Studio Mode (Lyric OFF): Cover Art rộng hơn (+width) và bớt cao (-height) để không bị bóp ảnh
        let available_h = layout_chunks[1].height;
        let available_w = layout_chunks[1].width;
        let spotlight_height = 10.min(available_h.saturating_sub(12));
        let max_left = available_w.saturating_sub(44);
        let left_w = ((available_h.saturating_sub(spotlight_height)) * 2 + 6).clamp(34, 48).min(max_left);
        let cover_h = (left_w / 2).saturating_sub(2).clamp(12, 20).min(available_h.saturating_sub(spotlight_height));

        let body_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(left_w), // Trái: Rộng hơn để ảnh thoải mái, không bị bóp
                Constraint::Min(40),        // Phải: Bảng bài hát đa cột / Browser / Plugins
            ])
            .split(layout_chunks[1]);

        let left_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(cover_h),       // Trên: Cover Art giảm tí độ cao
                Constraint::Min(spotlight_height), // Dưới: Spotlight & Specs nhận phần còn lại
            ])
            .split(body_chunks[0]);

        cover::render(frame, app, left_chunks[0]);
        spotlight::render(frame, app, left_chunks[1]);

        match app.view_mode {
            ViewMode::Playlist => playlist::render(frame, app, body_chunks[1]),
            ViewMode::Queue => queue::render(frame, app, body_chunks[1]),
            ViewMode::Browser => browser::render(frame, app, body_chunks[1]),
            #[cfg(feature = "download")]
            ViewMode::Download => {}
            ViewMode::Plugins => plugins::render(frame, app, body_chunks[1]),
        }
    }

    controls::render(frame, app, layout_chunks[2]);

    // Popup overlays
    if app.scanner.is_scanning {
        loading::render(frame, &app.scanner);
    } else if let Some(ref found) = app.scanner.pending_result {
        scan_dialog::render(frame, found.len());
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
}

pub fn post_render(app: &mut App) -> std::io::Result<()> {
    if !crate::cover::is_kitty_supported() {
        return Ok(());
    }

    let mut stdout = std::io::stdout();

    // Cover art lives either in:
    // 1. Cover Art Picker modal in download view (using app.download.cover_preview_art)
    // 2. Playlist/Queue/Browser split view (using app.cover)
    #[cfg(feature = "download")]
    let is_cover_modal = app.download.show_cover_picker_modal;
    #[cfg(not(feature = "download"))]
    let _is_cover_modal = false;

    #[cfg(feature = "download")]
    let in_download_overlay = (app.view_mode == ViewMode::Download
        || app.download.show_metadata_form
        || app.download.show_dir_picker)
        && !is_cover_modal;
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
    let (target_art, state_path) = if is_cover_modal {
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
