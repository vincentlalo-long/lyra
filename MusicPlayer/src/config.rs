//! Startup settings from `~/.config/lyra/config.toml` (v1: music folder only).
//!
//! Resolution priority for the music folder: **CLI arg > config > CWD**,
//! so existing usage (`lyra ~/Music`) never changes behavior.

use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

/// Minimal example written to the config path only as documentation
/// (never auto-created, to avoid surprising the user with new files).
#[allow(dead_code)]
const CONFIG_EXAMPLE: &str = "# lyra settings\n# music_folder = \"~/Music\"\n# download_dir = \"~/Music/Downloads\"\n";

/// On-disk schema (all fields optional; unknown fields ignored).
#[derive(Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct FileConfig {
    pub music_folder: Option<String>,
    pub download_dir: Option<String>,
    pub search_limit: Option<usize>,
}

fn config_path() -> Option<PathBuf> {
    // Test override: keeps `cargo test` hermetic so unit tests never
    // clobber the user's real `~/.config/lyra/config.toml`.
    // (Used by explicit-path helpers below; the global writers are
    // stubbed out entirely under `cfg(test)`.)
    if let Some(p) = std::env::var_os("LYRA_CONFIG_FILE") {
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
    Some(base.join("lyra").join("config.toml"))
}

/// Expands `~`/`$HOME` prefixes; relative paths stay relative to `cwd`.
/// Pure function for testability (`home` injected instead of read from env).
pub fn expand_path(input: &str, home: &str, cwd: &Path) -> PathBuf {
    let expanded = if input == "~" {
        home.to_string()
    } else if let Some(rest) = input.strip_prefix("~/") {
        format!("{home}/{rest}")
    } else if let Some(rest) = input.strip_prefix("$HOME/") {
        format!("{home}/{rest}")
    } else {
        input.to_string()
    };
    let p = PathBuf::from(expanded);
    if p.is_relative() {
        cwd.join(p)
    } else {
        p
    }
}

pub struct Config {
    /// Raw `music_folder` value from the file, if present.
    pub music_folder: Option<String>,
    /// Raw `download_dir` value from the file, if present.
    pub download_dir: Option<String>,
    /// Saved YouTube search result limit, if present.
    pub search_limit: Option<usize>,
}

impl Config {
    /// Loads the config file. Missing/unreadable/invalid files quietly
    /// yield defaults (a music player must start even with a broken config).
    pub fn load() -> Self {
        let text = config_path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .unwrap_or_default();
        let file: FileConfig = toml::from_str(&text).unwrap_or_default();
        Self {
            music_folder: file.music_folder,
            download_dir: file.download_dir,
            search_limit: file.search_limit,
        }
    }

    /// Save or update setting in config file (~/.config/lyra/config.toml).
    /// Unknown keys already present in the file are preserved.
    pub fn save_setting(key: &str, value: &str) -> std::io::Result<()> {
        #[cfg(test)]
        {
            // Hermetic tests: App-level tests assert in-memory state only.
            // Never touch the user's real config file from `cargo test`.
            // (Persistence itself is covered by `save_setting_to` tests below.)
            let _ = (key, value);
            return Ok(());
        }
        #[cfg(not(test))]
        {
            if let Some(path) = config_path() {
                Self::save_setting_to(&path, key, value)?;
            }
            Ok(())
        }
    }

    /// Explicit-path variant used by tests and future callers.
    /// Preserves unknown keys (merges into the existing TOML table
    /// instead of round-tripping through the minimal `FileConfig` struct).
    pub fn save_setting_to(path: &Path, key: &str, value: &str) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                let _ = std::fs::create_dir_all(parent);
            }
        }
        let text = std::fs::read_to_string(path).unwrap_or_default();
        let mut table: toml::Table = toml::from_str(&text).unwrap_or_default();
        match key {
            "music_folder" | "download_dir" => {
                table.insert(key.to_string(), toml::Value::String(value.to_string()));
            }
            // Numeric setting (YouTube search result limit).
            "search_limit" => {
                if let Ok(n) = value.parse::<i64>() {
                    if (1..=50).contains(&n) {
                        table.insert(key.to_string(), toml::Value::Integer(n));
                    }
                }
            }
            _ => {}
        }
        let serialized = toml::to_string_pretty(&table)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        std::fs::write(path, serialized)?;
        Ok(())
    }

    /// Explicit-path loader (test helper; mirrors [`Config::load`]).
    #[cfg(test)]
    pub fn load_from(path: &Path) -> Self {
        let text = std::fs::read_to_string(path).unwrap_or_default();
        let file: FileConfig = toml::from_str(&text).unwrap_or_default();
        Self {
            music_folder: file.music_folder,
            download_dir: file.download_dir,
            search_limit: file.search_limit,
        }
    }

    /// Resolves the effective download folder.
    pub fn resolve_download_dir(&self, fallback: &Path) -> (PathBuf, String) {
        let home = std::env::var("HOME").unwrap_or_default();
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        if let Some(raw) = &self.download_dir {
            let p = expand_path(raw, &home, &cwd);
            return (p, raw.clone());
        }
        let default_raw = "~/Music/Downloads";
        let default_p = expand_path(default_raw, &home, &cwd);
        if default_p.exists() {
            (default_p, default_raw.to_string())
        } else {
            (fallback.to_path_buf(), fallback.to_string_lossy().to_string())
        }
    }

    /// Documented path for help text / onboarding.
    #[allow(dead_code)]
    pub fn example() -> &'static str {
        CONFIG_EXAMPLE
    }

    /// Resolves the effective music folder. Returns the folder plus an
    /// optional warning (shown as a toast) when falling back.
    pub fn resolve_music_folder(&self, cli_arg: Option<PathBuf>) -> (PathBuf, Option<String>) {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        if let Some(arg) = cli_arg {
            return (arg, None);
        }
        if let Some(raw) = &self.music_folder {
            let home = std::env::var("HOME").unwrap_or_default();
            let p = expand_path(raw, &home, &cwd);
            if p.exists() {
                return (p, None);
            }
            return (
                cwd,
                Some(format!("Music folder missing, using current dir: {}", raw)),
            );
        }
        // Fallback: check standard ~/Music directory before cwd
        let home = std::env::var("HOME").unwrap_or_default();
        if !home.is_empty() {
            let default_music = PathBuf::from(&home).join("Music");
            if default_music.exists() && default_music.is_dir() {
                return (default_music, None);
            }
        }
        (cwd, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toml_parsing_tolerates_comments_and_unknown_fields() {
        let ok: FileConfig = toml::from_str(
            "# comment\nmusic_folder = \"~/Music\" # inline\nunknown_key = 1\n",
        )
        .unwrap();
        assert_eq!(ok.music_folder, Some("~/Music".to_string()));

        let empty: FileConfig = toml::from_str("").unwrap();
        assert_eq!(empty.music_folder, None);

        let broken: FileConfig = toml::from_str("music_folder = [unclosed").unwrap_or_default();
        assert_eq!(broken.music_folder, None);

        let wrong_type: FileConfig =
            toml::from_str("music_folder = 42").unwrap_or_default();
        assert_eq!(wrong_type.music_folder, None);
    }

    #[test]
    fn test_expand_path() {
        let cwd = Path::new("/work");
        assert_eq!(expand_path("~/Music", "/home/u", cwd), PathBuf::from("/home/u/Music"));
        assert_eq!(expand_path("~", "/home/u", cwd), PathBuf::from("/home/u"));
        assert_eq!(expand_path("$HOME/mp3", "/home/u", cwd), PathBuf::from("/home/u/mp3"));
        assert_eq!(expand_path("/abs/x", "/home/u", cwd), PathBuf::from("/abs/x"));
        assert_eq!(expand_path("rel/x", "/home/u", cwd), PathBuf::from("/work/rel/x"));
    }

    #[test]
    fn test_resolve_priority_cli_wins() {
        let cfg = Config {
            music_folder: Some("/cfg/music".to_string()),
            download_dir: None,
            search_limit: None,
        };
        let (p, w) = cfg.resolve_music_folder(Some(PathBuf::from("/cli/music")));
        assert_eq!(p, PathBuf::from("/cli/music"));
        assert!(w.is_none());
    }

    #[test]
    fn test_resolve_missing_config_dir_falls_back_with_warning() {
        let cfg = Config {
            music_folder: Some("/nonexistent-lyra-dir-xyz".to_string()),
            download_dir: None,
            search_limit: None,
        };
        let (p, w) = cfg.resolve_music_folder(None);
        assert!(w.is_some());
        assert!(p.exists() || p == std::env::current_dir().unwrap());
    }

    #[test]
    fn test_resolve_download_dir() {
        let cfg = Config {
            music_folder: None,
            download_dir: Some("~/Custom/Downloads".to_string()),
            search_limit: None,
        };
        let (p, raw) = cfg.resolve_download_dir(Path::new("/fallback"));
        assert_eq!(raw, "~/Custom/Downloads");
        assert!(p.to_string_lossy().ends_with("Custom/Downloads"));
    }

    #[test]
    fn test_save_and_reload_roundtrip_via_explicit_path() {
        // Regression test for "music directory resets on restart":
        // a saved music_folder must survive a load (quit -> relaunch).
        let dir = std::env::temp_dir().join(format!("lyra_cfg_test_{}", std::process::id()));
        let path = dir.join("config.toml");
        let _ = std::fs::remove_dir_all(&dir);

        Config::save_setting_to(&path, "music_folder", "/tmp/mytest_music").unwrap();
        Config::save_setting_to(&path, "download_dir", "/tmp/mytest_music/dl").unwrap();
        Config::save_setting_to(&path, "search_limit", "25").unwrap();
        let cfg = Config::load_from(&path);
        assert_eq!(cfg.music_folder, Some("/tmp/mytest_music".to_string()));
        assert_eq!(cfg.download_dir, Some("/tmp/mytest_music/dl".to_string()));
        assert_eq!(cfg.search_limit, Some(25));

        // Overwriting one key must not drop the other.
        Config::save_setting_to(&path, "music_folder", "/tmp/other").unwrap();
        let cfg = Config::load_from(&path);
        assert_eq!(cfg.music_folder, Some("/tmp/other".to_string()));
        assert_eq!(cfg.download_dir, Some("/tmp/mytest_music/dl".to_string()));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_save_preserves_unknown_keys() {
        let dir = std::env::temp_dir().join(format!("lyra_cfg_unknown_{}", std::process::id()));
        let path = dir.join("config.toml");
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(&path, "music_folder = \"/a\"\nfuture_option = \"keep-me\"\n").unwrap();

        Config::save_setting_to(&path, "music_folder", "/b").unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("future_option"), "unknown keys must survive saves: {text}");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
