//! Core song metadata for a track (ID3 tags + filename fallback + duration).
//!
//! Shared by the playlist table, spotlight, `notify` and `mpris` plugins so
//! none of them re-reads files on every UI frame. Callers should cache
//! [`SongMeta`] per path (see `App::song_meta`) instead of calling the
//! readers directly in render code.
use std::{
    fs::File,
    io::BufReader,
    path::Path,
    time::Duration,
};
use id3::TagLike;
use rodio::{Decoder, Source};

/// (artist, title, album) for a track.
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

/// Cached per-file metadata for list/table rendering.
#[derive(Debug, Clone, Default)]
pub struct SongMeta {
    pub artist: String,
    pub title: String,
    pub album: String,
    /// `None` when the duration can't be determined (shown as `--:--`).
    pub duration: Option<Duration>,
}

/// Reads [`SongMeta`] for one path: ID3 tags (fallback: filename), plus a
/// best-effort duration probe. Expensive (opens + parses the file), so
/// callers must cache the result per path — never call in a render loop.
pub fn song_meta(song_path: &Path) -> SongMeta {
    let (artist, title, album) = track_meta(song_path);
    SongMeta {
        artist,
        title,
        album,
        duration: probe_duration(song_path),
    }
}

/// Best-effort total duration via a decode-header probe.
/// Returns `None` for missing files, unknown containers, or VBR streams
/// without a seek table.
pub fn probe_duration(song_path: &Path) -> Option<Duration> {
    let file = File::open(song_path).ok()?;
    let decoder = Decoder::new(BufReader::new(file)).ok()?;
    let total = decoder.total_duration()?;
    if total.is_zero() {
        return None;
    }
    Some(total)
}

/// `mm:ss` for table cells (`--:--` when unknown).
pub fn format_duration(d: Option<Duration>) -> String {
    match d {
        Some(t) => format!("{:02}:{:02}", t.as_secs() / 60, t.as_secs() % 60),
        None => "--:--".to_string(),
    }
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
    fn test_format_duration() {
        assert_eq!(format_duration(Some(Duration::from_secs(222))), "03:42");
        assert_eq!(format_duration(Some(Duration::from_secs(61))), "01:01");
        assert_eq!(format_duration(None), "--:--");
    }

    #[test]
    fn test_probe_duration_missing_file_is_none() {
        assert_eq!(probe_duration(Path::new("/nonexistent/ghost.mp3")), None);
    }

    #[test]
    fn test_song_meta_uses_filename_fallback() {
        let m = song_meta(&PathBuf::from("/music/Daft Punk - One More Time.mp3"));
        assert_eq!(m.artist, "Daft Punk");
        assert_eq!(m.title, "One More Time");
        // Missing file: no duration, but metadata still resolves.
        assert_eq!(m.duration, None);
    }
}
