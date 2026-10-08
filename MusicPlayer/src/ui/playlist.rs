use ratatui::{
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};
use crate::{app::App, theme};

/// Formats `s` to fit exactly `target_width` display columns.
/// If `s` exceeds `target_width`, truncates safely at character boundaries and appends `…`.
/// If `s` is shorter than `target_width`, pads with ASCII spaces to reach `target_width`.
pub fn fit_width(s: &str, target_width: usize) -> String {
    if target_width == 0 {
        return String::new();
    }
    let current_width = UnicodeWidthStr::width(s);
    if current_width > target_width {
        let max_w = target_width.saturating_sub(1);
        let mut cur_w = 0;
        let mut truncated = String::new();
        for ch in s.chars() {
            let ch_w = UnicodeWidthChar::width(ch).unwrap_or(0);
            if cur_w + ch_w > max_w {
                break;
            }
            truncated.push(ch);
            cur_w += ch_w;
        }
        truncated.push('…');
        cur_w += 1;
        if cur_w < target_width {
            truncated.push_str(&" ".repeat(target_width - cur_w));
        }
        truncated
    } else {
        let padding = target_width - current_width;
        let mut res = s.to_string();
        res.push_str(&" ".repeat(padding));
        res
    }
}

/// Slices exactly `target_width` display columns from `s` starting at visual column `start_col`.
pub fn slice_visual_window(s: &str, start_col: usize, target_width: usize) -> String {
    if target_width == 0 {
        return String::new();
    }
    let mut current_col = 0;
    let mut collected = String::new();
    let mut collected_width = 0;

    for ch in s.chars() {
        let ch_w = UnicodeWidthChar::width(ch).unwrap_or(0);
        // Character is completely before our start column
        if current_col + ch_w <= start_col {
            current_col += ch_w;
            continue;
        }

        // Multi-width char crossing start_col
        if current_col < start_col {
            collected.push(' ');
            collected_width += 1;
            current_col += ch_w;
            continue;
        }

        if collected_width + ch_w > target_width {
            break;
        }

        collected.push(ch);
        collected_width += ch_w;
        current_col += ch_w;

        if collected_width >= target_width {
            break;
        }
    }

    if collected_width < target_width {
        collected.push_str(&" ".repeat(target_width - collected_width));
    }

    collected
}

/// Smooth horizontal marquee text: if `text` visual width <= `target_width`,
/// returns `fit_width(text, target_width)`.
/// If `text` is longer, returns a sliding window of `target_width` columns from
/// `"{text}    {text}..."` based on `elapsed` time.
pub fn marquee_text(
    text: &str,
    target_width: usize,
    elapsed: std::time::Duration,
    step_ms: u128,
    pause_ms: u128,
) -> String {
    if target_width == 0 {
        return String::new();
    }
    let text_width = UnicodeWidthStr::width(text);
    if text_width <= target_width {
        return fit_width(text, target_width);
    }

    const SEP: &str = "    ";
    let sep_width = 4;
    let loop_text = format!("{text}{SEP}");
    let loop_width = text_width + sep_width;

    let pause_steps = (pause_ms / step_ms) as usize;
    let cycle_steps = pause_steps + loop_width;

    let total_steps = (elapsed.as_millis() / step_ms) as usize;
    let current_step = total_steps % cycle_steps;

    let offset = if current_step < pause_steps {
        0
    } else {
        current_step - pause_steps
    };

    let reps = (offset + target_width) / loop_width + 2;
    let repeated = loop_text.repeat(reps);

    slice_visual_window(&repeated, offset, target_width)
}

pub fn render(frame: &mut Frame, app: &mut App, area: Rect) {
    // Selection change resets marquee scroll timer
    if app.selected_scroll_tracker.0 != app.playlist.selected {
        app.selected_scroll_tracker = (app.playlist.selected, std::time::Instant::now());
    }
    let scroll_elapsed = app.selected_scroll_tracker.1.elapsed();

    #[cfg(feature = "genre")]
    let genres = app.genre_filters.clone();
    #[cfg(feature = "genre")]
    let filtered = app.playlist.filtered_indices(&app.search_query, &genres, &mut app.genre_db);
    #[cfg(not(feature = "genre"))]
    let filtered = app.playlist.filtered_indices(&app.search_query);

    #[cfg(feature = "genre")]
    let unfiltered = app.search_query.is_empty() && app.genre_filters.is_empty();
    #[cfg(not(feature = "genre"))]
    let unfiltered = app.search_query.is_empty();
    let count_label = if unfiltered {
        format!("{}", app.playlist.songs.len())
    } else {
        format!("{}/{}", filtered.len(), app.playlist.songs.len())
    };

    #[cfg(feature = "genre")]
    let genre_badge = if !app.genre_filters.is_empty() {
        if app.genre_filters.len() == 1 {
            format!(" [Genre: {}] ", app.genre_filters[0])
        } else {
            format!(" [Genres ({}): {}] ", app.genre_filters.len(), app.genre_filters.join(", "))
        }
    } else {
        String::new()
    };
    #[cfg(not(feature = "genre"))]
    let genre_badge = "";

    let title = format!(" 󰲸 Track List ({count_label}){genre_badge} ");
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

    let items_to_render: Vec<(usize, std::path::PathBuf, bool, bool)> = filtered
        .iter()
        .enumerate()
        .map(|(pos, &song_idx)| {
            (
                pos,
                app.playlist.songs[song_idx].clone(),
                app.playlist.playing_index == Some(song_idx),
                song_idx == app.playlist.selected,
            )
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

            let num = if filtered.len() >= 100 {
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

        if filtered.is_empty() {
            list_items.push(ListItem::new(Line::from(vec![
                Span::styled("  (empty playlist or no match)", Style::default().fg(theme::OVERLAY0)),
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
        app.playlist.state.select(selected_pos);
        frame.render_stateful_widget(list_widget, list_area, &mut app.playlist.state);

        if filtered.len() > 1 {
            let mut scrollbar_state = ScrollbarState::new(filtered.len())
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

            let display = if is_selected {
                marquee_text(&song_name, avail, scroll_elapsed, 250, 1200)
            } else {
                fit_width(&song_name, avail)
            };

            let line_spans = vec![
                Span::styled(icon, icon_style),
                Span::styled(num, num_style),
                Span::styled(display, style),
            ];
            list_items.push(ListItem::new(Line::from(line_spans)));
        }

        if filtered.is_empty() {
            list_items.push(ListItem::new(Line::from(vec![
                Span::styled("  (empty playlist)", Style::default().fg(theme::OVERLAY0)),
            ])));
        }

        let list_widget = List::new(list_items);
        app.playlist.state.select(selected_pos);
        frame.render_stateful_widget(list_widget, inner, &mut app.playlist.state);

        if filtered.len() > 1 {
            let mut scrollbar_state = ScrollbarState::new(filtered.len())
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_fit_width_padding_and_truncation() {
        assert_eq!(fit_width("Song", 8), "Song    ");
        assert_eq!(UnicodeWidthStr::width(fit_width("Song", 8).as_str()), 8);

        // Truncation with ellipsis
        let truncated = fit_width("Very Long Song Title Here", 10);
        assert_eq!(UnicodeWidthStr::width(truncated.as_str()), 10);
        assert!(truncated.ends_with('…'));

        // Vietnamese text
        let vn = fit_width("Ánh Đèn Sân Khấu", 12);
        assert_eq!(UnicodeWidthStr::width(vn.as_str()), 12);
    }

    #[test]
    fn test_slice_visual_window() {
        let text = "Hello World    Hello World    ";
        let win0 = slice_visual_window(text, 0, 5);
        assert_eq!(win0, "Hello");
        assert_eq!(UnicodeWidthStr::width(win0.as_str()), 5);

        let win6 = slice_visual_window(text, 6, 5);
        assert_eq!(win6, "World");
        assert_eq!(UnicodeWidthStr::width(win6.as_str()), 5);
    }

    #[test]
    fn test_marquee_text_when_shorter_does_not_scroll() {
        let res = marquee_text("Short", 10, Duration::from_millis(5000), 250, 1000);
        assert_eq!(res, "Short     ");
        assert_eq!(UnicodeWidthStr::width(res.as_str()), 10);
    }

    #[test]
    fn test_marquee_text_pauses_initially_then_scrolls() {
        let long_title = "Waiting For You (feat. Onionn)";
        // In the initial pause period (under 1000ms), offset is 0
        let res0 = marquee_text(long_title, 15, Duration::from_millis(500), 250, 1000);
        let res_expected_start = slice_visual_window(&format!("{long_title}    {long_title}    "), 0, 15);
        assert_eq!(res0, res_expected_start);
        assert_eq!(UnicodeWidthStr::width(res0.as_str()), 15);

        // After initial pause (e.g. 1000ms + 250ms = step 1), it advances by 1
        let res1 = marquee_text(long_title, 15, Duration::from_millis(1250), 250, 1000);
        let res_expected_step1 = slice_visual_window(&format!("{long_title}    {long_title}    "), 1, 15);
        assert_eq!(res1, res_expected_step1);
        assert_eq!(UnicodeWidthStr::width(res1.as_str()), 15);
    }
}
