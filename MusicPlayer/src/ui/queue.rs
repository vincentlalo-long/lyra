use ratatui::{
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};
use crate::{
    app::App,
    theme,
    ui::playlist::{fit_width, marquee_text},
};

/// Render the Queue tab matching the exact multi-column table format of the Playlist tab.
/// Deleting here (`d`) only unqueues; files on disk are never touched.
pub fn render(frame: &mut Frame, app: &mut App, area: Rect) {
    // Selection change resets marquee scroll timer
    if app.queue_scroll_tracker.0 != app.queue.selected {
        app.queue_scroll_tracker = (app.queue.selected, std::time::Instant::now());
    }
    let scroll_elapsed = app.queue_scroll_tracker.1.elapsed();

    if !app.queue.items.is_empty() && app.queue.selected >= app.queue.items.len() {
        app.queue.selected = app.queue.items.len().saturating_sub(1);
    }

    let count_label = app.queue.len();
    let loop_tag = if app.queue.loop_enabled { " [Loop]" } else { "" };
    let title = format!(" 󰐗 Queue ({count_label}){loop_tag} ");
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::BLUE));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height == 0 || inner.width == 0 {
        return;
    }

    let items_to_render: Vec<(usize, std::path::PathBuf, bool, bool)> = app
        .queue
        .items
        .iter()
        .enumerate()
        .map(|(pos, song_path)| {
            let is_playing = app.current_playing_path.as_ref() == Some(song_path);
            let is_selected = pos == app.queue.selected;
            (pos, song_path.clone(), is_playing, is_selected)
        })
        .collect();

    let mut list_items = Vec::new();
    let mut selected_pos = None;

    if area.width >= 55 {
        // Table layout with strictly aligned columns:
        // content_w reserves 1 cell on the right to keep clear of the scrollbar
        let content_w = (inner.width as usize).saturating_sub(1);
        // Fixed widths: # (6 cols) + 4 inter-column gaps (4 cols) + Time (5 cols) = 15 cols
        let remaining = content_w.saturating_sub(15);
        let title_w = (remaining * 44 / 100).max(12);
        let artist_w = (remaining * 28 / 100).max(10);
        let album_w = remaining.saturating_sub(title_w + artist_w).max(8);

        let header_spans = vec![
            Span::styled("  #   ", Style::default().fg(theme::OVERLAY0).add_modifier(Modifier::BOLD)),
            Span::raw(" "),
            Span::styled(fit_width("Title", title_w), Style::default().fg(theme::OVERLAY0).add_modifier(Modifier::BOLD)),
            Span::raw(" "),
            Span::styled(fit_width("Artist", artist_w), Style::default().fg(theme::OVERLAY0).add_modifier(Modifier::BOLD)),
            Span::raw(" "),
            Span::styled(fit_width("Album", album_w), Style::default().fg(theme::OVERLAY0).add_modifier(Modifier::BOLD)),
            Span::raw(" "),
            Span::styled(fit_width("Time", 5), Style::default().fg(theme::OVERLAY0).add_modifier(Modifier::BOLD)),
        ];
        let header_line = Line::from(header_spans);

        for (pos, song_path, is_playing, is_selected) in items_to_render {
            let song_name = match song_path.file_stem() {
                Some(name) => name.to_string_lossy(),
                None => "Unknown".into(),
            };

            if is_selected {
                selected_pos = Some(pos);
            }

            let (icon, icon_style) = if is_selected && is_playing {
                let sym = if app.audio.is_paused { "󰏤 " } else { "▶ " };
                (
                    sym,
                    Style::default()
                        .fg(theme::YELLOW)
                        .bg(theme::SURFACE0)
                        .add_modifier(Modifier::BOLD),
                )
            } else if is_selected {
                (
                    " ",
                    Style::default().fg(theme::YELLOW).add_modifier(Modifier::BOLD),
                )
            } else if is_playing {
                let sym = if app.audio.is_paused { "󰏤 " } else { "▶ " };
                (
                    sym,
                    Style::default().fg(theme::GREEN).add_modifier(Modifier::BOLD),
                )
            } else {
                ("  ", Style::default().fg(theme::TEXT))
            };

            let num = if app.queue.len() >= 100 {
                format!("{:03} ", pos + 1)
            } else {
                format!("{:02}. ", pos + 1)
            };

            let (style, artist_style, album_style, dur_style, num_style, space_style) = if is_selected && is_playing {
                (
                    Style::default()
                        .fg(theme::GREEN)
                        .bg(theme::SURFACE0)
                        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                    Style::default().fg(theme::ROSE).bg(theme::SURFACE0),
                    Style::default().fg(theme::SUBTEXT0).bg(theme::SURFACE0),
                    Style::default().fg(theme::YELLOW).bg(theme::SURFACE0),
                    Style::default().fg(theme::OVERLAY0).bg(theme::SURFACE0),
                    Style::default().bg(theme::SURFACE0),
                )
            } else if is_selected {
                (
                    Style::default()
                        .fg(theme::YELLOW)
                        .bg(theme::SURFACE0)
                        .add_modifier(Modifier::BOLD),
                    Style::default().fg(theme::ROSE).bg(theme::SURFACE0),
                    Style::default().fg(theme::SUBTEXT0).bg(theme::SURFACE0),
                    Style::default().fg(theme::YELLOW).bg(theme::SURFACE0),
                    Style::default().fg(theme::OVERLAY0).bg(theme::SURFACE0),
                    Style::default().bg(theme::SURFACE0),
                )
            } else if is_playing {
                (
                    Style::default().fg(theme::GREEN).add_modifier(Modifier::BOLD),
                    Style::default().fg(theme::ROSE),
                    Style::default().fg(theme::SUBTEXT0),
                    Style::default().fg(theme::YELLOW),
                    Style::default().fg(theme::OVERLAY0),
                    Style::default(),
                )
            } else {
                (
                    Style::default().fg(theme::TEXT),
                    Style::default().fg(theme::ROSE),
                    Style::default().fg(theme::SUBTEXT0),
                    Style::default().fg(theme::YELLOW),
                    Style::default().fg(theme::OVERLAY0),
                    Style::default(),
                )
            };

            let meta = app.song_meta(&song_path);
            let display_title = if meta.title.is_empty() {
                song_name.to_string()
            } else {
                meta.title
            };

            let t_formatted = if is_selected {
                marquee_text(&display_title, title_w, scroll_elapsed, 250, 1200)
            } else {
                fit_width(&display_title, title_w)
            };

            let a_formatted = fit_width(if meta.artist.is_empty() { "—" } else { &meta.artist }, artist_w);
            let alb_formatted = fit_width(if meta.album.is_empty() { "—" } else { &meta.album }, album_w);
            let dur_formatted = fit_width(&crate::meta::format_duration(meta.duration), 5);

            let line_spans = vec![
                Span::styled(icon, icon_style),
                Span::styled(num, num_style),
                Span::styled(" ", space_style),
                Span::styled(t_formatted, style),
                Span::styled(" ", space_style),
                Span::styled(a_formatted, artist_style),
                Span::styled(" ", space_style),
                Span::styled(alb_formatted, album_style),
                Span::styled(" ", space_style),
                Span::styled(dur_formatted, dur_style),
            ];
            list_items.push(ListItem::new(Line::from(line_spans)));
        }

        if app.queue.items.is_empty() {
            list_items.push(ListItem::new(Line::from(vec![
                Span::styled("  (empty queue)", Style::default().fg(theme::OVERLAY0)),
            ])));
            list_items.push(ListItem::new(Line::from(vec![
                Span::styled("  a: queue song   A: play next", Style::default().fg(theme::OVERLAY0)),
            ])));
        }

        let list_area = if inner.height >= 2 {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(1), Constraint::Min(1)])
                .split(inner);
            frame.render_widget(Paragraph::new(header_line), chunks[0]);
            chunks[1]
        } else {
            inner
        };

        let list_widget = List::new(list_items);
        app.queue.state.select(selected_pos);
        frame.render_stateful_widget(list_widget, list_area, &mut app.queue.state);

        if app.queue.len() > 1 {
            let mut scrollbar_state = ScrollbarState::new(app.queue.len())
                .position(selected_pos.unwrap_or(0));
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None)
                .thumb_style(Style::default().fg(theme::BLUE))
                .track_style(Style::default().fg(theme::SURFACE0));
            frame.render_stateful_widget(
                scrollbar,
                list_area.inner(Margin { vertical: 0, horizontal: 0 }),
                &mut scrollbar_state,
            );
        }
    } else {
        // Compact single-column view (< 55 cols)
        let avail = (inner.width as usize).saturating_sub(7);

        for (pos, song_path, is_playing, is_selected) in items_to_render {
            let song_name = match song_path.file_stem() {
                Some(name) => name.to_string_lossy(),
                None => "Unknown".into(),
            };

            if is_selected {
                selected_pos = Some(pos);
            }

            let (icon, icon_style) = if is_selected && is_playing {
                let sym = if app.audio.is_paused { "󰏤 " } else { "▶ " };
                (
                    sym,
                    Style::default()
                        .fg(theme::YELLOW)
                        .bg(theme::SURFACE0)
                        .add_modifier(Modifier::BOLD),
                )
            } else if is_selected {
                (
                    " ",
                    Style::default().fg(theme::YELLOW).add_modifier(Modifier::BOLD),
                )
            } else if is_playing {
                let sym = if app.audio.is_paused { "󰏤 " } else { "▶ " };
                (
                    sym,
                    Style::default().fg(theme::GREEN).add_modifier(Modifier::BOLD),
                )
            } else {
                ("  ", Style::default().fg(theme::TEXT))
            };

            let num = format!("{:02}. ", pos + 1);

            let (style, num_style) = if is_selected && is_playing {
                (
                    Style::default()
                        .fg(theme::GREEN)
                        .bg(theme::SURFACE0)
                        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                    Style::default().fg(theme::OVERLAY0).bg(theme::SURFACE0),
                )
            } else if is_selected {
                (
                    Style::default()
                        .fg(theme::YELLOW)
                        .bg(theme::SURFACE0)
                        .add_modifier(Modifier::BOLD),
                    Style::default().fg(theme::OVERLAY0).bg(theme::SURFACE0),
                )
            } else if is_playing {
                (
                    Style::default().fg(theme::GREEN).add_modifier(Modifier::BOLD),
                    Style::default().fg(theme::OVERLAY0),
                )
            } else {
                (
                    Style::default().fg(theme::TEXT),
                    Style::default().fg(theme::OVERLAY0),
                )
            };

            let meta = app.song_meta(&song_path);
            let display_title = if meta.title.is_empty() {
                song_name.to_string()
            } else {
                meta.title
            };

            let t_formatted = if is_selected {
                marquee_text(&display_title, avail, scroll_elapsed, 250, 1200)
            } else {
                fit_width(&display_title, avail)
            };

            list_items.push(ListItem::new(Line::from(vec![
                Span::styled(icon, icon_style),
                Span::styled(num, num_style),
                Span::styled(t_formatted, style),
            ])));
        }

        if app.queue.items.is_empty() {
            list_items.push(ListItem::new(Line::from(vec![
                Span::styled("  (empty queue)", Style::default().fg(theme::OVERLAY0)),
            ])));
        }

        let list_widget = List::new(list_items);
        app.queue.state.select(selected_pos);
        frame.render_stateful_widget(list_widget, inner, &mut app.queue.state);

        if app.queue.len() > 1 {
            let mut scrollbar_state = ScrollbarState::new(app.queue.len())
                .position(selected_pos.unwrap_or(0));
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None)
                .thumb_style(Style::default().fg(theme::BLUE))
                .track_style(Style::default().fg(theme::SURFACE0));
            frame.render_stateful_widget(
                scrollbar,
                inner.inner(Margin { vertical: 0, horizontal: 0 }),
                &mut scrollbar_state,
            );
        }
    }
}
