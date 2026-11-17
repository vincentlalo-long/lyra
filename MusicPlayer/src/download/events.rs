use serde::Deserialize;

use std::path::PathBuf;

fn default_source() -> String {
    "youtube".to_string()
}

#[derive(Debug, Clone, Deserialize)]
pub struct SearchItem {
    pub id: String,
    /// Downloadable URL. Official music results (Spotify/Deezer/iTunes)
    /// leave this empty until the user picks the track; `resolve` then
    /// maps it to a matching YouTube video.
    pub url: String,
    pub title: String,
    pub artist: String,
    pub uploader: String,
    pub duration: String,
    /// Metadata provider: spotify/musicbrainz/deezer/itunes/youtube.
    #[serde(default = "default_source")]
    pub source: String,
    #[serde(default)]
    pub album: String,
    #[serde(default)]
    pub year: String,
    #[serde(default)]
    pub cover_url: String,
    #[serde(default)]
    pub duration_secs: u64,
    /// LRCLIB synced-lyric availability, when known.
    #[serde(default)]
    pub has_lyrics: Option<bool>,
}

impl SearchItem {
    /// True when this result still needs a playable URL resolved.
    pub fn needs_resolve(&self) -> bool {
        self.source != "youtube" || self.url.trim().is_empty()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct CoverCandidate {
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub artist: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub album: String,
    #[serde(default)]
    pub year: String,
    #[serde(default)]
    pub cover_url: String,
    #[serde(default)]
    pub score: f32,
    #[serde(default)]
    pub cached_path: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DownloadRequest {
    pub url: String,
    pub output_dir: PathBuf,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub genre: Option<String>,
    pub cover_mode: String,
    /// Studio provider set for the final download (`auto`/`all`/`web`/
    /// `itunes`/`deezer`/`caa`/`youtube`). Python validates + falls back.
    pub cover_source: String,
    /// Crop focal point (0..1) for `center_crop`, chosen in the crop editor.
    pub cover_focus: Option<(f32, f32)>,
    pub cover_url: Option<String>,
    pub custom_cover: Option<String>,
    pub no_lyrics: bool,
    pub no_auto_lyrics: bool,
    /// Specific lyric language code to download (e.g. "vi", "en"). None = priority fallback.
    pub lyrics_lang: Option<String>,
}

#[derive(Debug, Clone)]
pub enum DownloadEvent {
    Searching,
    SearchResults(Vec<SearchItem>),
    InfoLoaded {
        url: String,
        title: String,
        artist: String,
        /// Canonical album from the official provider / LRCLIB.
        album: Option<String>,
        /// Original metadata source (spotify/musicbrainz/deezer/itunes/youtube).
        source: Option<String>,
        /// Official cover art, used as the default download cover.
        cover_url: Option<String>,
        /// LRCLIB synced-lyric availability.
        has_lyrics: Option<bool>,
        /// Language codes available for lyric download.
        lyric_langs: Vec<String>,
        /// Track duration in seconds (for LRCLIB probing/ranking).
        duration_secs: u64,
    },
    /// Subtitle availability for the form's Lyrics row
    /// (probed in background when the metadata form opens).
    LyricsProbe {
        /// Preferred provider for this result: "lrclib" or "youtube".
        source: String,
        has_manual: bool,
        has_auto: bool,
        /// Available subtitle language codes from yt-dlp info.
        available_langs: Vec<String>,
    },
    /// A remote cover image was staged to the local disk cache
    /// for crop editing (prefetch had missed it).
    ImageStaged {
        path: String,
    },
    CoverLoading,
    CoverResults(Vec<CoverCandidate>),
    Progress {
        percent: f32,
        speed: String,
        eta: String,
        stage: String,
    },
    Status(String),
    Completed {
        title: String,
        artist: String,
        mp3_path: String,
        lrc_path: Option<String>,
        /// Language code of the downloaded lyrics (e.g. "vi", "en").
        lyric_lang: Option<String>,
        cover_source: Option<String>,
    },
    Error(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_item_defaults_to_youtube_and_needs_no_resolve() {
        let item: SearchItem = serde_json::from_str(
            r#"{"id":"yt1","url":"https://youtu.be/x","title":"T","artist":"A","uploader":"U","duration":"03:00"}"#,
        )
        .unwrap();
        assert_eq!(item.source, "youtube");
        assert_eq!(item.duration_secs, 0);
        assert!(!item.needs_resolve());
    }

    #[test]
    fn test_official_search_item_needs_resolve() {
        let item: SearchItem = serde_json::from_str(
            r#"{"id":"deezer:1","url":"","title":"T","artist":"A","uploader":"U",
                "duration":"03:00","source":"deezer","album":"Alb","duration_secs":180}"#,
        )
        .unwrap();
        assert_eq!(item.source, "deezer");
        assert_eq!(item.album, "Alb");
        assert_eq!(item.duration_secs, 180);
        assert!(item.needs_resolve());
    }
}
