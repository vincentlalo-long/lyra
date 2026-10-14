//! Mouse support: drag the seekbar to seek, scroll on it to ±5s.
//!
//! The seekbar rect is published every frame by `ui::controls` into
//! `App::seekbar_rect`. Clicks/drags map `column -> ratio -> seek_to()`.
//! Seeking itself stays debounced in `audio::seek` (150ms), so continuous
//! drags only reopen the file once the user pauses.

use std::time::Duration;

use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

use crate::app::App;

/// Map a terminal column to a 0.0-1.0 seek ratio inside `rect`.
/// Returns `None` when the rect has zero width.
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

/// Handle one mouse event. Always returns `true` (mouse never quits).
/// Returns whether the event was consumed (used for future overlay logic).
pub fn handle_mouse(app: &mut App, ev: MouseEvent) -> bool {
    match ev.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            if modal_open(app) {
                return false;
            }
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
            false
        }
        MouseEventKind::Drag(MouseButton::Left) => {
            if !app.seeking_drag {
                return false;
            }
            if let Some(rect) = app.seekbar_rect {
                // Clamp vertically: dragging slightly above/below still seeks.
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
        MouseEventKind::ScrollUp => {
            if modal_open(app) {
                return false;
            }
            if let Some(rect) = app.seekbar_rect {
                if ev.column >= rect.x && ev.column < rect.x + rect.width && ev.row == rect.y {
                    app.audio.seek_forward(5);
                    return true;
                }
            }
            false
        }
        MouseEventKind::ScrollDown => {
            if modal_open(app) {
                return false;
            }
            if let Some(rect) = app.seekbar_rect {
                if ev.column >= rect.x && ev.column < rect.x + rect.width && ev.row == rect.y {
                    app.audio.seek_backward(5);
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
        // Past the end clamps.
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
    fn test_mouse_down_starts_drag_and_seeks() {
        let dir = std::env::temp_dir();
        if let Ok(mut app) = App::new(&dir) {
            app.audio.duration = Some(Duration::from_secs(200));
            app.seekbar_rect = Some(Rect { x: 10, y: 5, width: 20, height: 1 });
            let ev = MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: 20,
                row: 5,
                modifiers: crossterm::event::KeyModifiers::empty(),
            };
            assert!(handle_mouse(&mut app, ev));
            assert!(app.seeking_drag);
            // Midpoint of a 20-wide bar starting at x=10 -> ratio 0.5 -> 100s pending.
            assert_eq!(app.audio.position(), Duration::from_secs(100));
            let up = MouseEvent {
                kind: MouseEventKind::Up(MouseButton::Left),
                column: 20,
                row: 5,
                modifiers: crossterm::event::KeyModifiers::empty(),
            };
            assert!(handle_mouse(&mut app, up));
            assert!(!app.seeking_drag);
        }
    }

    #[test]
    fn test_mouse_down_outside_bar_ignored() {
        let dir = std::env::temp_dir();
        if let Ok(mut app) = App::new(&dir) {
            app.audio.duration = Some(Duration::from_secs(200));
            app.seekbar_rect = Some(Rect { x: 10, y: 5, width: 20, height: 1 });
            let ev = MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: 2,
                row: 2,
                modifiers: crossterm::event::KeyModifiers::empty(),
            };
            assert!(!handle_mouse(&mut app, ev));
            assert!(!app.seeking_drag);
        }
    }
}
