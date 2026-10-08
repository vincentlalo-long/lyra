use std::{path::Path, process::Command};
use crate::meta::track_meta;

/// Notification display time in milliseconds.
const NOTIFY_TIMEOUT_MILLIS: &str = "4000";

/// Sends a "now playing" notification via the Freedesktop Notifications spec
/// (`org.freedesktop.Notifications` on the session bus).
///
/// Works with any spec-compliant server — quickshell, mako, dunst, swaync —
/// because delivery is just D-Bus, not tied to a specific daemon.
///
/// Best-effort by design: every failure (missing `notify-send`, no server
/// running, D-Bus down) is silently ignored so playback never breaks.
/// Set `LYRA_NO_NOTIFY=1` to disable.
pub fn send_track_notification(song_path: &Path) {
    if std::env::var_os("LYRA_NO_NOTIFY").is_some() {
        return;
    }

    let (artist, title, album) = track_meta(song_path);
    if title.is_empty() {
        return;
    }

    let summary = if artist.is_empty() {
        title
    } else {
        format!("{artist} — {title}")
    };

    let mut cmd = Command::new("notify-send");
    cmd.args(["-a", "lyra", "-t", NOTIFY_TIMEOUT_MILLIS, &summary]);
    if !album.is_empty() {
        cmd.arg(&album);
    }
    // Intentionally ignore the result: notifications must never fail playback.
    let _ = cmd.output();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_send_track_notification_never_panics() {
        // Must not panic even when everything fails; keep the test run silent.
        unsafe {
            std::env::set_var("LYRA_NO_NOTIFY", "1");
        }
        send_track_notification(Path::new("/nonexistent/ghost.mp3"));
        unsafe {
            std::env::remove_var("LYRA_NO_NOTIFY");
        }
    }
}
