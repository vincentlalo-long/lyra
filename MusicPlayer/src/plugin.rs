//! Plugin & Environment Management data structures for Lyra
//! Provides real state for enabled/disabled/removed plugins, configurable paths,
//! and community plugin download preview/mockup.

use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct ManagedPlugin {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub enabled: bool,
    #[serde(default)]
    pub is_removed: bool,
    #[serde(default)]
    pub is_builtin: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigPathItem {
    pub id: String,
    pub label: String,
    pub path: String,
    pub description: String,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct DownloadablePlugin {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    #[serde(default)]
    pub is_installed: bool,
    /// Direct download URL for the plugin archive (`.tar.gz`) or a single
    /// `.py` file. Also accepts `file://` URLs (useful for tests and local
    /// mirrors). Empty = no remote source; install falls back to a local
    /// copy from the `entry` directory (repo checkout).
    #[serde(default)]
    pub url: String,
    /// Expected hex sha256 of the downloaded file. Empty = skip verification.
    #[serde(default)]
    pub sha256: String,
    /// Minimum lyra version required, e.g. `"v0.2.6"`. Empty = any version.
    #[serde(default, rename = "min_lyra_version")]
    pub min_version: String,
}

/// Current binary version (`MusicPlayer/Cargo.toml`), e.g. `"0.2.6"`.
pub const LYRA_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Stable identity for every built-in plugin.
///
/// Compile-time feature gates, the Plugin Manager metadata, and every runtime
/// `App::plugin_enabled` check derive from this one enum, so an id can never
/// silently drift from the feature that ships it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PluginId {
    Download,
    Mpris,
    Notify,
    Genre,
    Lyrics,
    Daylists,
    Artwork,
    Visualizer,
    Updater,
}

impl PluginId {
    /// Every built-in plugin, in Plugin Manager display order.
    pub const ALL: [Self; 9] = [
        Self::Download,
        Self::Mpris,
        Self::Notify,
        Self::Genre,
        Self::Lyrics,
        Self::Daylists,
        Self::Artwork,
        Self::Visualizer,
        Self::Updater,
    ];

    /// Stable id persisted in `plugins.json`; never rename an existing one.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Download => "download",
            Self::Mpris => "mpris",
            Self::Notify => "notify",
            Self::Genre => "genre",
            Self::Lyrics => "lyrics",
            Self::Daylists => "daylists",
            Self::Artwork => "artwork",
            Self::Visualizer => "visualizer",
            Self::Updater => "updater",
        }
    }

    /// True when the plugin's code was compiled into this binary.
    pub const fn is_compiled(self) -> bool {
        match self {
            Self::Download => cfg!(feature = "download"),
            Self::Mpris => cfg!(feature = "mpris"),
            Self::Notify => cfg!(feature = "notify"),
            Self::Genre => cfg!(feature = "genre"),
            Self::Updater => cfg!(feature = "update"),
            Self::Lyrics | Self::Daylists | Self::Artwork | Self::Visualizer => true,
        }
    }

    /// Looks up a built-in plugin by persisted id (`None` for community ids).
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|id| id.as_str() == name)
    }
}

impl std::fmt::Display for PluginId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

fn parse_version_numbers(v: &str) -> Vec<u64> {
    v.trim()
        .trim_start_matches(['v', 'V'])
        .split('.')
        .map(|part| {
            part.chars()
                .take_while(|c| c.is_ascii_digit())
                .collect::<String>()
                .parse()
                .unwrap_or(0)
        })
        .collect()
}

/// Compares two `vX.Y.Z` versions. Returns Greater/Equal/Less.
pub fn cmp_versions(a: &str, b: &str) -> std::cmp::Ordering {
    let pa = parse_version_numbers(a);
    let pb = parse_version_numbers(b);
    let n = pa.len().max(pb.len()).max(1);
    for i in 0..n {
        let x = pa.get(i).copied().unwrap_or(0);
        let y = pb.get(i).copied().unwrap_or(0);
        match x.cmp(&y) {
            std::cmp::Ordering::Equal => continue,
            other => return other,
        }
    }
    std::cmp::Ordering::Equal
}

/// Errors when the running binary is older than `min` (empty = always Ok).
pub fn check_min_version(min: &str) -> Result<(), String> {
    if min.trim().is_empty() {
        return Ok(());
    }
    if cmp_versions(LYRA_VERSION, min) == std::cmp::Ordering::Less {
        return Err(format!(
            "requires lyra {min} (you have v{LYRA_VERSION}); please update lyra first"
        ));
    }
    Ok(())
}

impl ManagedPlugin {
    /// Builds the Plugin Manager metadata row for a built-in plugin.
    /// `enabled` follows the compile-time feature, so metadata and gates
    /// cannot disagree.
    fn builtin(id: PluginId, name: &str, version: &str, description: &str) -> Self {
        Self {
            id: id.as_str().to_string(),
            name: name.to_string(),
            version: version.to_string(),
            description: description.to_string(),
            enabled: id.is_compiled(),
            is_removed: false,
            is_builtin: true,
        }
    }

    /// Typed id for built-in plugins; `None` for community plugins.
    pub fn plugin_id(&self) -> Option<PluginId> {
        PluginId::from_name(&self.id)
    }

    pub fn default_plugins() -> Vec<Self> {
        vec![
            Self::builtin(
                PluginId::Download,
                "YouTube Audio Downloader",
                "v0.6.0",
                "YouTube audio search, yt-dlp downloader & ID3 artwork embedder",
            ),
            Self::builtin(
                PluginId::Mpris,
                "MPRIS Dynamic Island Bridge",
                "v1.1.0",
                "Desktop media keys, playerctl and Dynamic Island D-Bus widget",
            ),
            Self::builtin(
                PluginId::Notify,
                "Desktop Notifications",
                "v0.8.5",
                "Native OS notification toasts on track change (dunst/mako/swaync)",
            ),
            Self::builtin(
                PluginId::Genre,
                "Genre Classifier & Tagger",
                "v0.5.1",
                "Sidecar genre database and offline ID3 TCON tag classifier",
            ),
            Self::builtin(
                PluginId::Lyrics,
                "LRC Karaoke Lyrics Engine",
                "v1.0.5",
                "LRC format parser & synchronized dual-panel karaoke lyrics renderer",
            ),
            Self::builtin(
                PluginId::Daylists,
                "Smart Daylists & Queue History",
                "v0.9.0",
                "Auto-saves daily listening queue and history snapshots to disk",
            ),
            Self::builtin(
                PluginId::Artwork,
                "Cover Art Engine",
                "v0.5.0",
                "Multi-source album artwork extraction, caching & kitty renderer",
            ),
            Self::builtin(
                PluginId::Visualizer,
                "Audio Visualizer",
                "v0.3.1",
                "Real-time audio waveform visualizer (ricing style) with multiple wave types and colors",
            ),
            Self::builtin(
                PluginId::Updater,
                "Update (GitHub Releases)",
                &format!("v{LYRA_VERSION}"),
                "Checks GitHub Releases for a newer Lyra; press U here to check/install (tarball builds)",
            ),
        ]
    }

    /// Resolves path to ~/.config/lyra/plugins.json
    pub fn plugins_config_path() -> Option<PathBuf> {
        // Test override (see config.rs `LYRA_CONFIG_FILE`).
        if let Some(p) = std::env::var_os("LYRA_PLUGINS_FILE") {
            return Some(PathBuf::from(p));
        }
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME").map(|h| {
                    let mut p = PathBuf::from(h);
                    p.push(".config");
                    p
                })
            })?;
        Some(base.join("lyra").join("plugins.json"))
    }

    /// Loads plugins from ~/.config/lyra/plugins.json and auto-merges with current binary defaults.
    /// Preserves user enable/disable settings and custom plugins across version upgrades!
    pub fn load_and_merge_plugins() -> Vec<Self> {
        let defaults = Self::default_plugins();
        let Some(path) = Self::plugins_config_path() else {
            return defaults;
        };
        let Ok(content) = std::fs::read_to_string(&path) else {
            return defaults;
        };
        let Ok(saved) = serde_json::from_str::<Vec<Self>>(&content) else {
            return defaults;
        };

        let mut result = Vec::new();
        // 1. Process built-in plugins: update binary version/desc, but preserve user's enabled/removed state
        for mut def in defaults {
            if let Some(old) = saved.iter().find(|p| p.id == def.id) {
                def.enabled = old.enabled;
                def.is_removed = old.is_removed;
            }
            result.push(def);
        }

        // 2. Preserve custom / community plugins installed by user
        for old in saved {
            if !old.is_builtin && !result.iter().any(|p| p.id == old.id) {
                result.push(old);
            }
        }

        result
    }

    /// Persists plugins configuration to ~/.config/lyra/plugins.json
    pub fn save_plugins(plugins: &[Self]) -> std::io::Result<()> {
        #[cfg(test)]
        {
            // Hermetic tests: never clobber the user's real plugins.json.
            let _ = plugins;
            return Ok(());
        }
        #[cfg(not(test))]
        {
            if let Some(path) = Self::plugins_config_path() {
                if let Some(parent) = path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let json = serde_json::to_string_pretty(plugins)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
                std::fs::write(&path, json)?;
            }
            Ok(())
        }
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
    /// Root dir for real (downloaded/copied) community plugins:
    /// `$LYRA_DATA_HOME` (tests) > `$XDG_DATA_HOME/lyra/plugins` >
    /// `~/.local/share/lyra/plugins`. Each plugin lives in `<root>/<id>/`
    /// with a `plugin.py` entrypoint.
    pub fn data_root() -> Option<PathBuf> {
        if let Some(p) = std::env::var_os("LYRA_DATA_HOME") {
            return Some(PathBuf::from(p));
        }
        if let Some(xdg) = std::env::var_os("XDG_DATA_HOME") {
            return Some(PathBuf::from(xdg).join("lyra").join("plugins"));
        }
        std::env::var_os("HOME")
            .map(|h| PathBuf::from(h).join(".local").join("share").join("lyra").join("plugins"))
    }

    /// Installed location of this plugin (`<root>/<id>`), if a data dir exists.
    pub fn install_dir(&self) -> Option<PathBuf> {
        Self::data_root().map(|r| r.join(&self.id))
    }

    /// True when `<install_dir>/plugin.py` exists on disk.
    pub fn is_present_on_disk(&self) -> bool {
        self.install_dir()
            .map(|d| d.join("plugin.py").is_file())
            .unwrap_or(false)
    }

    /// Reconciles `is_installed` with reality: a plugin counts as installed
    /// only if its entrypoint file actually exists (old-version binaries
    /// that only flipped the mock flag get corrected on next launch).
    pub fn refresh_installed_from_disk(plugins: &mut [Self]) {
        for p in plugins.iter_mut() {
            if p.is_installed && !p.is_present_on_disk() {
                // Keep the flag if there is simply nowhere to install to
                // (no HOME); otherwise an orphaned flag is a lie — clear it.
                if Self::data_root().is_some() {
                    p.is_installed = false;
                }
            } else if !p.is_installed && p.is_present_on_disk() {
                p.is_installed = true;
            }
        }
    }

    /// Finds the plugin's `entry` directory on local disk (repo checkout or
    /// dev paths). Used when the manifest has no `url` — the plugin ships
    /// with the source tree instead of a remote archive.
    pub fn find_local_entry(entry: &str) -> Option<PathBuf> {
        if entry.trim().is_empty() {
            return None;
        }
        let rel = PathBuf::from(entry);
        let mut candidates = vec![
            rel.clone(),
            PathBuf::from("..").join(&rel),
            PathBuf::from("../..").join(&rel),
        ];
        if let Ok(exe) = std::env::current_exe() {
            let mut curr = exe.parent();
            while let Some(parent) = curr {
                candidates.push(parent.join(&rel));
                curr = parent.parent();
            }
        }
        if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
            for base in ["projects/lyra", "lyra", ".local/share/lyra"] {
                candidates.push(home.join(base).join(&rel));
            }
        }
        candidates.into_iter().find(|p| p.join("plugin.py").is_file())
    }
    pub fn available_community_plugins() -> Vec<Self> {
        // 1. Try reading registry.json from disk (development / user custom registry)
        let candidates = [
            PathBuf::from("registry.json"),
            PathBuf::from("../registry.json"),
            PathBuf::from("../../registry.json"),
        ];
        for c in &candidates {
            if let Ok(content) = std::fs::read_to_string(c) {
                if let Ok(plugins) = serde_json::from_str::<Vec<Self>>(&content) {
                    if !plugins.is_empty() {
                        return plugins;
                    }
                }
            }
        }

        if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
            let user_paths = [
                home.join("projects/lyra/registry.json"),
                home.join("lyra/registry.json"),
                home.join(".config/lyra/registry.json"),
                home.join(".local/share/lyra/registry.json"),
            ];
            for p in &user_paths {
                if let Ok(content) = std::fs::read_to_string(p) {
                    if let Ok(plugins) = serde_json::from_str::<Vec<Self>>(&content) {
                        if !plugins.is_empty() {
                            return plugins;
                        }
                    }
                }
            }
        }

        // 2. Embedded fallback at compile-time from registry.json
        const EMBEDDED_REGISTRY: &str = include_str!("../../registry.json");
        if let Ok(plugins) = serde_json::from_str::<Vec<Self>>(EMBEDDED_REGISTRY) {
            if !plugins.is_empty() {
                return plugins;
            }
        }

        // 3. Hardcoded default fallback
        vec![
            Self {
                id: "ocr-video-lyrics".into(),
                name: "OCR Video HardCode Lyric to lrc".into(),
                version: "Coming Soon".into(),
                author: "@vincentlalo-long".into(),
                description: "Extract hardcoded subtitles from music videos via OCR into synced .lrc lyrics (Coming Soon)".into(),
                is_installed: false,
                url: String::new(),
                sha256: String::new(),
                min_version: String::new(),
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cmp::Ordering;

    #[test]
    fn test_version_compare_and_gate() {
        assert_eq!(cmp_versions("v0.2.6", "v0.2.6"), Ordering::Equal);
        assert_eq!(cmp_versions("0.2.6", "v0.2.6"), Ordering::Equal);
        assert_eq!(cmp_versions("v0.2.10", "v0.2.6"), Ordering::Greater);
        assert_eq!(cmp_versions("v0.2.6", "v99.0.0"), Ordering::Less);
        assert_eq!(cmp_versions("v1.0", "v1.0.0"), Ordering::Equal);
        assert!(check_min_version("").is_ok());
        assert!(check_min_version("v0.0.1").is_ok());
        assert!(check_min_version("v99.0.0").is_err());
    }

    #[test]
    fn test_manifest_parses_new_fields_with_defaults() {
        let json = r#"[{"id":"x","name":"X","version":"v1.0.0","author":"a","description":"d","entry":"plugins/x","status":"active"}]"#;
        let v: Vec<DownloadablePlugin> = serde_json::from_str(json).unwrap();
        assert_eq!(v[0].url, "");
        assert_eq!(v[0].sha256, "");
        assert_eq!(v[0].min_version, "");
        let json2 = r#"[{"id":"y","name":"Y","version":"v1.0.0","author":"a","description":"d","entry":"plugins/y","status":"active","url":"https://example.com/y.tar.gz","sha256":"abc","min_lyra_version":"v9.9.9"}]"#;
        let w: Vec<DownloadablePlugin> = serde_json::from_str(json2).unwrap();
        assert_eq!(w[0].url, "https://example.com/y.tar.gz");
        assert_eq!(w[0].min_version, "v9.9.9");
        assert!(check_min_version(&w[0].min_version).is_err());
    }

    #[test]
    fn test_merge_plugins_preserves_user_settings_and_updates_version() {
        let defaults = ManagedPlugin::default_plugins();
        let download_def = defaults
            .iter()
            .find(|p| p.plugin_id() == Some(PluginId::Download))
            .unwrap();
        assert_eq!(download_def.version, "v0.6.0");

        // Simulate saved plugins from older version (download was v0.3.2 and user disabled it)
        let old_saved = vec![
            ManagedPlugin {
                id: "download".into(),
                name: "Old Name".into(),
                version: "v0.3.2".into(),
                description: "Old Desc".into(),
                enabled: false, // User explicitly turned this off
                is_removed: false,
                is_builtin: true,
            },
            ManagedPlugin {
                id: "custom-plugin-123".into(),
                name: "Custom Addon".into(),
                version: "v1.0.0".into(),
                description: "User installed addon".into(),
                enabled: true,
                is_removed: false,
                is_builtin: false,
            },
        ];

        let mut merged = Vec::new();
        for mut def in defaults {
            if let Some(old) = old_saved.iter().find(|p| p.id == def.id) {
                def.enabled = old.enabled;
                def.is_removed = old.is_removed;
            }
            merged.push(def);
        }
        for old in old_saved {
            if !old.is_builtin && !merged.iter().any(|p| p.id == old.id) {
                merged.push(old);
            }
        }

        let download_merged = merged
            .iter()
            .find(|p| p.plugin_id() == Some(PluginId::Download))
            .unwrap();
        // Version is upgraded to v0.6.0 from new binary:
        assert_eq!(download_merged.version, "v0.6.0");
        // But user preference (enabled = false) is strictly preserved!
        assert_eq!(download_merged.enabled, false);

        // Custom plugin is completely preserved:
        assert!(merged.iter().any(|p| p.id == "custom-plugin-123" && p.name == "Custom Addon"));
    }

    #[test]
    fn test_plugin_registry_is_consistent() {
        let defaults = ManagedPlugin::default_plugins();
        assert_eq!(defaults.len(), PluginId::ALL.len());
        for (plugin, id) in defaults.iter().zip(PluginId::ALL) {
            assert_eq!(plugin.id, id.as_str());
            assert_eq!(plugin.enabled, id.is_compiled());
            assert_eq!(plugin.plugin_id(), Some(id));
            assert!(plugin.is_builtin && !plugin.is_removed);
        }
    }

    #[test]
    fn test_plugin_id_round_trip_and_uniqueness() {
        let unique: std::collections::HashSet<&str> =
            PluginId::ALL.iter().map(|id| id.as_str()).collect();
        assert_eq!(unique.len(), PluginId::ALL.len());
        for id in PluginId::ALL {
            assert_eq!(PluginId::from_name(id.as_str()), Some(id));
        }
        assert_eq!(PluginId::from_name("community-plugin"), None);
    }
}
