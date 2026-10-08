use serde::Deserialize;

use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
pub struct SearchItem {
    pub id: String,
    pub url: String,
    pub title: String,
    pub artist: String,
    pub uploader: String,
    pub duration: String,
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
    pub cover_url: Option<String>,
    pub custom_cover: Option<String>,
    pub no_lyrics: bool,
    pub no_auto_lyrics: bool,
}

#[derive(Debug, Clone)]
pub enum DownloadEvent {
    Searching,
    SearchResults(Vec<SearchItem>),
    InfoLoaded {
        url: String,
        title: String,
        artist: String,
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
        cover_source: Option<String>,
    },
    Error(String),
}
