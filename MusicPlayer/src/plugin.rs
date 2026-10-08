//! Plugin & Environment Management data structures for Lyra
//! Provides real state for enabled/disabled/removed plugins, configurable paths,
//! and community plugin download preview/mockup.

use std::path::Path;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ManagedPlugin {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub enabled: bool,
    pub is_removed: bool,
    pub is_builtin: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigPathItem {
    pub id: String,
    pub label: String,
    pub path: String,
    pub description: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DownloadablePlugin {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub is_installed: bool,
}

impl ManagedPlugin {
    pub fn default_plugins() -> Vec<Self> {
        vec![
            Self {
                id: "download".into(),
                name: "YouTube Audio Downloader".into(),
                version: "v0.3.2".into(),
                description: "YouTube audio search, yt-dlp downloader & ID3 artwork embedder".into(),
                enabled: cfg!(feature = "download"),
                is_removed: false,
                is_builtin: true,
            },
            Self {
                id: "mpris".into(),
                name: "MPRIS Dynamic Island Bridge".into(),
                version: "v1.1.0".into(),
                description: "Desktop media keys, playerctl and Dynamic Island D-Bus widget".into(),
                enabled: cfg!(feature = "mpris"),
                is_removed: false,
                is_builtin: true,
            },
            Self {
                id: "notify".into(),
                name: "Desktop Notifications".into(),
                version: "v0.8.5".into(),
                description: "Native OS notification toasts on track change (dunst/mako/swaync)".into(),
                enabled: cfg!(feature = "notify"),
                is_removed: false,
                is_builtin: true,
            },
            Self {
                id: "genre".into(),
                name: "Genre Classifier & Tagger".into(),
                version: "v0.5.1".into(),
                description: "Sidecar genre database and offline ID3 TCON tag classifier".into(),
                enabled: cfg!(feature = "genre"),
                is_removed: false,
                is_builtin: true,
            },
            Self {
                id: "lyrics".into(),
                name: "LRC Karaoke Lyrics Engine".into(),
                version: "v1.0.4".into(),
                description: "LRC format parser & synchronized dual-panel karaoke lyrics renderer".into(),
                enabled: true,
                is_removed: false,
                is_builtin: true,
            },
            Self {
                id: "daylists".into(),
                name: "Smart Daylists & Queue History".into(),
                version: "v0.9.0".into(),
                description: "Auto-saves daily listening queue and history snapshots to disk".into(),
                enabled: true,
                is_removed: false,
                is_builtin: true,
            },
            Self {
                id: "artwork".into(),
                name: "Cover Art Engine".into(),
                version: "v0.4.5".into(),
                description: "Multi-source album artwork extraction, caching & kitty renderer".into(),
                enabled: true,
                is_removed: false,
                is_builtin: true,
            },
        ]
    }
}

impl ConfigPathItem {
    pub fn default_paths(music_folder: &Path) -> Vec<Self> {
        let music_dir_str = music_folder.to_string_lossy().to_string();
        vec![
            Self {
                id: "music_folder".into(),
                label: "Music Library Path".into(),
                path: music_dir_str,
                description: "Primary library folder scanned for MP3 tracks".into(),
            },
            Self {
                id: "download_dir".into(),
                label: "YouTube Download Path".into(),
                path: "~/Music/Downloads".into(),
                description: "Destination folder where downloaded YouTube songs are saved".into(),
            },
            Self {
                id: "config_file".into(),
                label: "Config File Path".into(),
                path: "~/.config/lyra/config.toml".into(),
                description: "Global TOML configuration file for Lyra".into(),
            },
            Self {
                id: "daylists_dir".into(),
                label: "Daylists Storage Path".into(),
                path: "~/.config/lyra/daylists/".into(),
                description: "Folder where daily queue states and snapshots are persisted".into(),
            },
        ]
    }
}

impl DownloadablePlugin {
    pub fn available_community_plugins() -> Vec<Self> {
        vec![
            Self {
                id: "ocr-video-lyrics".into(),
                name: "OCR Video HardCode Lyric to lrc".into(),
                version: "Coming Soon".into(),
                author: "@vincentlalo-long".into(),
                description: "Extract hardcoded subtitles from music videos via OCR into synced .lrc lyrics (Coming Soon)".into(),
                is_installed: false,
            },
        ]
    }
}
