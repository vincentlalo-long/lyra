//! Mouse support: seekbar drag, playlist/browser/queue click, volume bar.
//!
//! Gated behind the `mouse` Cargo feature. When the feature is disabled or
//! `app.mouse_enabled` is false, all mouse events are ignored.
//!
//! Double-click on a track row starts playback immediately; single-click
//! selects the track. Clicking a directory in the browser descends into it;
//! clicking an audio file queues and plays it.
//!
//! Row rects are published by the UI renderers each frame into
//! `App::{playlist_row_rects, browser_row_rects, queue_row_rects}`.

#![cfg(feature = "mouse")]

use std::time::{Duration, Instant};

use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

use crate::app::{App, ViewMode};

const DOUBLE_CLICK_MAX_MS: u64 = 300;

/// Map a terminal column to a 0.0-1.0 seek ratio inside `rect`.
pub fn ratio_for_column(column: u16, rect: Rect) -> Option<f32> {
    if rect.width == 0 {
        return None;
    }
    let rel = column.saturating_sub(rect.x) as f32 / rect.width as f32;
    Some(rel.clamp(0.0, 1.0))
}

/// Convert a ratio + total duration to an absolute seek target.
pub fn ratio_to_target(ratio: f32, total: Duration) -> Duration {
    let secs = total.as_secs_f32() * ratio.clamp(0.0, 1.0);
    Duration::from_secs_f32(secs)
}

/// Find the row index whose rect contains `(column, row)`.
fn find_row(rects: &[(u16, Rect, usize)], column: u16, row: u16) -> Option<usize> {
    rects.iter().find_map(|(r, rect, idx)| {
        if *r == row
            && column >= rect.x
            && column < rect.x.saturating_add(rect.width)
        {
            Some(*idx)
        } else {
            None
        }
    })
}

fn modal_open(app: &App) -> bool {
    if app.show_help || app.browser_modal.is_some() || app.scanner.is_scanning {
        return true;
    }
    #[cfg(feature = "genre")]
    if app.genre_picker.is_some() || app.genre_tagger.is_some() {
        return true;
    }
    #[cfg(feature = "download")]
    if app.download.show_metadata_form
        || app.download.show_cover_picker_modal
        || app.download.show_cover_file_picker
        || app.download.show_crop_modal
        || app.download.show_dir_picker
    {
        return true;
    }
    false
}

fn seek_to_ratio(app: &mut App, ratio: f32) {
    if let Some(total) = app.audio.duration {
        if total.as_secs_f32() > 0.0 {
            app.audio.seek_to(ratio_to_target(ratio, total));
        }
    }
}

/// Handle one mouse event. Returns whether the event was consumed.
/// When `mouse` feature is disabled or `app.mouse_enabled` is false,
/// always returns `false` (event falls through).
pub fn handle_mouse(app: &mut App, ev: MouseEvent) -> bool {
    if !app.mouse_enabled {
        return false;
    }

    match ev.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            if modal_open(app) {
                return false;
            }

            // Seekbar click/drag start.
            if let Some(rect) = app.seekbar_rect {
                if ev.column >= rect.x
                    && ev.column < rect.x + rect.width
                    && ev.row == rect.y
                {
                    if let Some(ratio) = ratio_for_column(ev.column, rect) {
                        app.seeking_drag = true;
                        seek_to_ratio(app, ratio);
                        return true;
                    }
                }
            }

            // Volume bar click.
            if let Some(rect) = app.volume_bar_rect {
                if ev.column >= rect.x
                    && ev.column < rect.x + rect.width
                    && ev.row == rect.y
                {
                    if let Some(ratio) = ratio_for_column(ev.column, rect) {
                        let vol = ratio.clamp(0.0, 1.0);
                        app.audio.set_volume_absolute(vol);
                        app.set_toast(format!("Volume: {:.0}%", vol * 100.0));
                        return true;
                    }
                }
            }

            // Playlist row click.
            if app.view_mode == ViewMode::Playlist && !app.playlist_row_rects.is_empty() {
                if let Some(idx) = find_row(&app.playlist_row_rects, ev.column, ev.row) {
                    let now = Instant::now();
                    let is_double = app.last_click.map_or(false, |(t, last_idx, last_mode)| {
                        last_mode == ViewMode::Playlist
                            && last_idx == idx
                            && now.duration_since(t).as_millis() <= DOUBLE_CLICK_MAX_MS as u128
                    });
                    app.last_click = Some((now, idx, ViewMode::Playlist));

                    app.playlist.selected = idx;
                    if is_double {
                        app.play_selected(idx);
                        app.set_toast(format!("Playing: {}", idx + 1));
                    } else {
                        app.set_toast(format!("Selected: {}", idx + 1));
                    }
                    return true;
                }
            }

            // Browser row click.
            if app.view_mode == ViewMode::Browser && !app.browser_row_rects.is_empty() {
                if let Some(idx) = find_row(&app.browser_row_rects, ev.column, ev.row) {
                    let now = Instant::now();
                    let is_double = app.last_click.map_or(false, |(t, last_idx, last_mode)| {
                        last_mode == ViewMode::Browser
                            && last_idx == idx
                            && now.duration_since(t).as_millis() <= DOUBLE_CLICK_MAX_MS as u128
                    });
                    app.last_click = Some((now, idx, ViewMode::Browser));

                    app.browser.selected = idx;
                    if is_double {
                        app.browser_open_or_play(idx);
                        app.set_toast(format!("Opened/Playing: {}", idx + 1));
                    } else {
                        app.set_toast(format!("Selected: {}", idx + 1));
                    }
                    return true;
                }
            }

            // Queue row click.
            if app.view_mode == ViewMode::Queue && !app.queue_row_rects.is_empty() {
                if let Some(idx) = find_row(&app.queue_row_rects, ev.column, ev.row) {
                    let now = Instant::now();
                    let is_double = app.last_click.map_or(false, |(t, last_idx, last_mode)| {
                        last_mode == ViewMode::Queue
                            && last_idx == idx
                            && now.duration_since(t).as_millis() <= DOUBLE_CLICK_MAX_MS as u128
                    });
                    app.last_click = Some((now, idx, ViewMode::Queue));

                    app.queue_selected(idx);
                    if is_double {
                        app.play_queue_index(idx);
                        app.set_toast(format!("Playing queue: {}", idx + 1));
                    } else {
                        app.set_toast(format!("Selected queue: {}", idx + 1));
                    }
                    return true;
                }
            }

            false
        }
        MouseEventKind::Drag(MouseButton::Left) => {
            if !app.seeking_drag {
                return false;
            }
            if let Some(rect) = app.seekbar_rect {
                if ev.row.saturating_sub(rect.y) <= 1 || rect.y.saturating_sub(ev.row) <= 1 {
                    if let Some(ratio) = ratio_for_column(ev.column, rect) {
                        seek_to_ratio(app, ratio);
                        return true;
                    }
                }
            }
            false
        }
        MouseEventKind::Up(MouseButton::Left) => {
            if app.seeking_drag {
                app.seeking_drag = false;
                return true;
            }
            false
        }
        MouseEventKind::ScrollDown => {
            if modal_open(app) {
                return false;
            }
            // Seekbar scroll: seek backward 5s.
            if let Some(rect) = app.seekbar_rect {
                if ev.column >= rect.x
                    && ev.column < rect.x + rect.width
                    && ev.row == rect.y
                {
                    app.audio.seek_backward(5);
                    return true;
                }
            }
            // Playlist scroll.
            if app.view_mode == ViewMode::Playlist && !app.playlist_row_rects.is_empty() {
                if find_row(&app.playlist_row_rects, ev.column, ev.row).is_some() {
                    let n = app.playlist.songs.len();
                    if n > 0 {
                        app.playlist.selected = (app.playlist.selected + 1).min(n - 1);
                        return true;
                    }
                }
            }
            // Browser scroll.
            if app.view_mode == ViewMode::Browser && !app.browser_row_rects.is_empty() {
                if find_row(&app.browser_row_rects, ev.column, ev.row).is_some() {
                    let n = app.browser.entries.len();
                    if n > 0 {
                        app.browser.selected = (app.browser.selected + 1).min(n - 1);
                        return true;
                    }
                }
            }
            // Queue scroll.
            if app.view_mode == ViewMode::Queue && !app.queue_row_rects.is_empty() {
                if find_row(&app.queue_row_rects, ev.column, ev.row).is_some() {
                    let n = app.queue.items.len();
                    if n > 0 {
                        app.queue.selected = (app.queue.selected + 1).min(n - 1);
                        return true;
                    }
                }
            }
            false
        }
        MouseEventKind::ScrollUp => {
            if modal_open(app) {
                return false;
            }
            // Seekbar scroll: seek forward 5s.
            if let Some(rect) = app.seekbar_rect {
                if ev.column >= rect.x
                    && ev.column < rect.x + rect.width
                    && ev.row == rect.y
                {
                    app.audio.seek_forward(5);
                    return true;
                }
            }
            // Playlist scroll.
            if app.view_mode == ViewMode::Playlist && !app.playlist_row_rects.is_empty() {
                if find_row(&app.playlist_row_rects, ev.column, ev.row).is_some() {
                    app.playlist.selected = app.playlist.selected.saturating_sub(1);
                    return true;
                }
            }
            // Browser scroll.
            if app.view_mode == ViewMode::Browser && !app.browser_row_rects.is_empty() {
                if find_row(&app.browser_row_rects, ev.column, ev.row).is_some() {
                    app.browser.selected = app.browser.selected.saturating_sub(1);
                    return true;
                }
            }
            // Queue scroll.
            if app.view_mode == ViewMode::Queue && !app.queue_row_rects.is_empty() {
                if find_row(&app.queue_row_rects, ev.column, ev.row).is_some() {
                    app.queue.selected = app.queue.selected.saturating_sub(1);
                    return true;
                }
            }
            false
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ratio_for_column_edges() {
        let rect = Rect { x: 10, y: 5, width: 20, height: 1 };
        assert_eq!(ratio_for_column(10, rect), Some(0.0));
        assert_eq!(ratio_for_column(30, rect), Some(1.0));
        assert_eq!(ratio_for_column(99, rect), Some(1.0));
        assert_eq!(ratio_for_column(0, rect), Some(0.0));
        let zero = Rect { x: 0, y: 0, width: 0, height: 1 };
        assert_eq!(ratio_for_column(0, zero), None);
    }

    #[test]
    fn test_ratio_to_target_midpoint() {
        let total = Duration::from_secs(200);
        assert_eq!(ratio_to_target(0.5, total), Duration::from_secs(100));
        assert_eq!(ratio_to_target(0.0, total), Duration::ZERO);
        assert_eq!(ratio_to_target(1.0, total), total);
        assert_eq!(ratio_to_target(2.0, total), total);
    }

    #[test]
    fn test_find_row() {
        let rects = vec![
            (5, Rect { x: 0, y: 5, width: 20, height: 1 }, 0),
            (6, Rect { x: 0, y: 6, width: 20, height: 1 }, 1),
        ];
        assert_eq!(find_row(&rects, 5, 5), Some(0));
        assert_eq!(find_row(&rects, 5, 6), Some(1));
        assert_eq!(find_row(&rects, 25, 5), None);
        assert_eq!(find_row(&rects, 5, 7), None);
    }

    #[test]
    fn test_mouse_disabled_ignored() {
        let dir = std::env::temp_dir();
        if let Ok(mut app) = App::new(&dir) {
            app.mouse_enabled = false;
            app.audio.duration = Some(Duration::from_secs(200));
            app.seekbar_rect = Some(Rect { x: 10, y: 5, width: 20, height: 1 });
            let ev = MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: 20,
                row: 5,
                modifiers: crossterm::event::KeyModifiers::empty(),
            };
            assert!(!handle_mouse(&mut app, ev));
            assert!(!app.seeking_drag);
        }
    }
}
