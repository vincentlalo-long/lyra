mod browser;
mod controls;
mod cover;
mod dir_picker;
mod download;
mod help;
mod loading;
mod lyrics;
mod playlist;
mod queue;

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::{
    app::{App, ViewMode},
    theme,
};

pub fn render(frame: &mut Frame, app: &mut App) {
    let layout_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(5),    // Dual Panel Body / Download View
            Constraint::Length(4), // Footer Controls
        ])
        .split(frame.area());

    // Header block with help & search filter on the border.
    // A fresh toast (queue ops, day-list saves) temporarily replaces the hints.
    let mut title_spans = Vec::new();
    if let Some(toast) = app.toast_text() {
        title_spans.push(Span::styled(
            format!(" 󰇚 {toast} "),
            Style::default().fg(theme::YELLOW).add_modifier(Modifier::BOLD),
        ));
    }
    if !app.search_query.is_empty() {
        title_spans.push(Span::styled(
            format!(" 🔍 \"{}\" [Esc: Clear] ", app.search_query),
            Style::default().fg(theme::YELLOW).add_modifier(Modifier::BOLD),
        ));
    }
    title_spans.extend(vec![
        Span::styled("[", Style::default().fg(theme::OVERLAY0)),
        Span::styled("q", Style::default().fg(theme::YELLOW).add_modifier(Modifier::BOLD)),
        Span::styled(": Quit | ", Style::default().fg(theme::TEXT)),
        Span::styled("/", Style::default().fg(theme::YELLOW).add_modifier(Modifier::BOLD)),
        Span::styled(": Search | ", Style::default().fg(theme::TEXT)),
        Span::styled("?", Style::default().fg(theme::YELLOW).add_modifier(Modifier::BOLD)),
        Span::styled(": Help", Style::default().fg(theme::TEXT)),
        Span::styled("] ", Style::default().fg(theme::OVERLAY0)),
    ]);

    let header_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::BLUE))
        .title(Line::from(title_spans))
        .title_alignment(Alignment::Right);

    let inner_header = header_block.inner(layout_chunks[0]);
    frame.render_widget(header_block, layout_chunks[0]);

    if app.is_searching {
        let search_line = Line::from(vec![
            Span::styled(" 🔍 Search: ", Style::default().fg(theme::YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled(&app.search_query, Style::default().fg(theme::TEXT).add_modifier(Modifier::BOLD)),
            Span::styled("█", Style::default().fg(theme::YELLOW)),
            Span::raw("   "),
            Span::styled("[Enter: Play | Esc: Cancel]", Style::default().fg(theme::OVERLAY0)),
        ]);
        frame.render_widget(Paragraph::new(search_line), inner_header);
    } else {
        let tab_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(11),  // Brand
                Constraint::Ratio(1, 4), // Tab 1
                Constraint::Ratio(1, 4), // Tab 2
                Constraint::Ratio(1, 4), // Tab 3
                Constraint::Ratio(1, 4), // Tab 4
            ])
            .split(inner_header);

        frame.render_widget(
            Paragraph::new(Span::styled(" 󰎆 LYRA", Style::default().fg(theme::BLUE).add_modifier(Modifier::BOLD))),
            tab_chunks[0],
        );

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
        let download_tab_style = if app.view_mode == ViewMode::Download {
            Style::default().fg(theme::MAUVE).bg(theme::SURFACE0).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme::OVERLAY0)
        };

        let playlist_text = if app.view_mode == ViewMode::Playlist { " [1] 󰲸 Playlist " } else { "[1] 󰲸 Playlist" };
        let queue_text = if app.view_mode == ViewMode::Queue { " [2] 󰐗 Queue " } else { "[2] 󰐗 Queue" };
        let browser_text = if app.view_mode == ViewMode::Browser { " [3] 󰉋 Browser " } else { "[3] 󰉋 Browser" };
        let download_text = if app.view_mode == ViewMode::Download { " [4] 󰇚 Download " } else { "[4] 󰇚 Download" };

        frame.render_widget(
            Paragraph::new(Span::styled(playlist_text, playlist_tab_style)).alignment(Alignment::Center),
            tab_chunks[1],
        );
        frame.render_widget(
            Paragraph::new(Span::styled(queue_text, queue_tab_style)).alignment(Alignment::Center),
            tab_chunks[2],
        );
        frame.render_widget(
            Paragraph::new(Span::styled(browser_text, browser_tab_style)).alignment(Alignment::Center),
            tab_chunks[3],
        );
        frame.render_widget(
            Paragraph::new(Span::styled(download_text, download_tab_style)).alignment(Alignment::Center),
            tab_chunks[4],
        );
    }

    if app.view_mode == ViewMode::Download {
        download::render(frame, app, layout_chunks[1]);
    } else {
        // Dual Panel Body: 25% Left (Track List + Album Art), 75% Right (Lyrics)
        let body_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25), // Left: Playlist/Browser + Album Art (snug fit)
                Constraint::Percentage(75), // Right: Lyrics Karaoke
            ])
            .split(layout_chunks[1]);

        let left_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(50), // Upper: Playlist or Browser
                Constraint::Percentage(50), // Lower: 1:1 Album Art
            ])
            .split(body_chunks[0]);

        match app.view_mode {
            ViewMode::Playlist => playlist::render(frame, app, left_chunks[0]),
            ViewMode::Queue => queue::render(frame, app, left_chunks[0]),
            ViewMode::Browser => browser::render(frame, app, left_chunks[0]),
            ViewMode::Download => {}
        }

        cover::render(frame, app, left_chunks[1]);
        lyrics::render(frame, app, body_chunks[1]);
    }

    controls::render(frame, app, layout_chunks[2]);

    // Popup overlays
    if app.scanner.is_scanning {
        loading::render(frame, &app.scanner);
    }

    if app.show_help {
        help::render(frame);
    }
}

pub fn post_render(app: &mut App) -> std::io::Result<()> {
    if !crate::cover::is_kitty_supported() {
        return Ok(());
    }

    let mut stdout = std::io::stdout();

    // Cover art only lives in the Playlist/Queue/Browser split view. In the
    // Download view (or under any modal) there is no cover rect: drop any
    // stale Kitty image so it can't linger over unrelated UI.
    let cover_visible = app.view_mode != ViewMode::Download
        && !app.show_help
        && !app.scanner.is_scanning
        && !app.download.show_metadata_form
        && !app.download.show_dir_picker;
    if !cover_visible {
        if app.last_kitty_rendered.is_some() {
            crate::cover::clear_kitty_image(&mut stdout)?;
            app.last_kitty_rendered = None;
        }
        app.kitty_cover_rect = None;
        return Ok(());
    }

    if let (Some(rect), Some(cover_art)) = (app.kitty_cover_rect, &app.cover) {
        if let Some(b64) = &cover_art.png_base64 {
            let current_state = (app.current_playing_path.clone(), rect);
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
