use std::{path::Path, process::Command};
use id3::TagLike;

/// Notification display time in milliseconds.
const NOTIFY_TIMEOUT_MILLIS: &str = "4000";

/// (artist, title, album) for a track.
/// Prefers embedded ID3 tags, falls back to parsing the `Artist - Title.mp3`
/// filename convention, and finally to the bare file stem.
pub fn track_meta(song_path: &Path) -> (String, String, String) {
    if let Ok(tag) = id3::Tag::read_from_path(song_path) {
        let artist = tag.artist().unwrap_or("").trim().to_string();
        let title = tag.title().unwrap_or("").trim().to_string();
        let album = tag.album().unwrap_or("").trim().to_string();
        if !title.is_empty() {
            return (artist, title, album);
        }
    }

    let stem = song_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .trim();

    if let Some((left, right)) = stem.split_once(" - ") {
        let (artist, title) = (left.trim(), right.trim());
        if !title.is_empty() {
            return (artist.to_string(), title.to_string(), String::new());
        }
    }

    (String::new(), stem.to_string(), String::new())
}

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
    use std::path::PathBuf;

    #[test]
    fn test_track_meta_parses_artist_dash_title_filename() {
        let (artist, title, _) =
            track_meta(&PathBuf::from("/music/Daft Punk - One More Time.mp3"));
        assert_eq!(artist, "Daft Punk");
        assert_eq!(title, "One More Time");
    }

    #[test]
    fn test_track_meta_falls_back_to_stem() {
        let (artist, title, _) = track_meta(&PathBuf::from("/music/lonely.mp3"));
        assert_eq!(artist, "");
        assert_eq!(title, "lonely");
    }

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
