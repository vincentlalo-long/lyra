//! Genre tags: hand-curated sidecar + embedded ID3 TCON, sidecar wins.
//!
//! The sidecar (`~/.config/lyra/genres.json`, `{abs_path: [genres]}`) covers
//! the existing library and personal mood tags; TCON frames (written for new
//! downloads by the Python pipeline) cover the rest. TCON reads are cached
//! per session because parsing full ID3 tags (incl. cover art) is slow on
//! large libraries.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use id3::TagLike;

fn sidecar_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|h| {
                let mut p = PathBuf::from(h);
                p.push(".config");
                p
            })
        })?;
    Some(base.join("lyra").join("genres.json"))
}

fn split_genre_list(raw: &str) -> Vec<String> {
    raw.split([',', '/', ';', '|'])
        .map(|g| g.trim().to_string())
        .filter(|g| !g.is_empty())
        .collect()
}

pub struct GenreDB {
    /// Absolute-path string -> hand-curated genres. Reloaded on picker open.
    sidecar: HashMap<String, Vec<String>>,
    /// Absolute-path string -> parsed TCON genres (session cache).
    tcon_cache: HashMap<String, Vec<String>>,
}

impl GenreDB {
    /// Empty DB for unit tests (no disk, no TCON reads).
    #[cfg(test)]
    pub fn new_empty_for_tests() -> Self {
        Self {
            sidecar: HashMap::new(),
            tcon_cache: HashMap::new(),
        }
    }

    /// DB preloaded with sidecar entries for unit tests.
    #[cfg(test)]
    pub fn with_sidecar_entries_for_tests(
        entries: impl IntoIterator<Item = (String, Vec<String>)>,
    ) -> Self {
        Self {
            sidecar: entries.into_iter().collect(),
            tcon_cache: HashMap::new(),
        }
    }

    pub fn load() -> Self {
        let mut db = Self {
            sidecar: HashMap::new(),
            tcon_cache: HashMap::new(),
        };
        db.reload_sidecar();
        db
    }

    /// Returns all distinct genres recorded in the sidecar database, preserving case.
    pub fn recorded_genres(&self) -> Vec<String> {
        let mut out = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for genres in self.sidecar.values() {
            for g in genres {
                let trimmed = g.trim();
                if !trimmed.is_empty() && seen.insert(trimmed.to_lowercase()) {
                    out.push(trimmed.to_string());
                }
            }
        }
        out
    }

    /// Re-reads the sidecar file (picks up external edits, e.g. from
    /// `lyra tag-genre` / `lyra fill-genres` while the TUI runs).
    pub fn reload_sidecar(&mut self) {
        self.sidecar.clear();
        let Some(path) = sidecar_path() else {
            return;
        };
        let Ok(text) = std::fs::read_to_string(path) else {
            return;
        };
        let Ok(map): Result<HashMap<String, serde_json::Value>, _> =
            serde_json::from_str(&text)
        else {
            return;
        };
        for (k, v) in map {
            let genres: Vec<String> = match &v {
                serde_json::Value::Array(arr) => arr
                    .iter()
                    .filter_map(|g| g.as_str())
                    .map(|g| g.trim().to_string())
                    .filter(|g| !g.is_empty())
                    .collect(),
                serde_json::Value::String(s) if !s.trim().is_empty() => {
                    vec![s.trim().to_string()]
                }
                _ => continue,
            };
            if !genres.is_empty() {
                self.sidecar.insert(k, genres);
            }
        }
    }

    pub fn canonical_key(song: &Path) -> String {
        if let Ok(canon) = std::fs::canonicalize(song) {
            return canon.to_string_lossy().into_owned();
        }
        if song.is_absolute() {
            return song.to_string_lossy().into_owned();
        }
        if let Ok(cwd) = std::env::current_dir() {
            let abs = cwd.join(song);
            if let Ok(canon) = std::fs::canonicalize(&abs) {
                return canon.to_string_lossy().into_owned();
            }
            return abs.to_string_lossy().into_owned();
        }
        song.to_string_lossy().into_owned()
    }

    /// Flexible lookup in sidecar: tries exact key, canonicalized path, absolute path,
    /// and suffix filename matching so relative/browser paths never miss.
    pub fn find_sidecar_genres(&self, song: &Path) -> Option<Vec<String>> {
        let raw = song.to_string_lossy();
        if let Some(g) = self.sidecar.get(raw.as_ref()) {
            return Some(g.clone());
        }

        let canon = Self::canonical_key(song);
        if let Some(g) = self.sidecar.get(&canon) {
            return Some(g.clone());
        }

        // Fuzzy match by file name + parent directory
        if let Some(file_name) = song.file_name() {
            for (k, v) in &self.sidecar {
                let kp = Path::new(k);
                if kp.file_name() == Some(file_name) {
                    let k_parent = kp.parent().and_then(|p| p.file_name());
                    let s_parent = song.parent().and_then(|p| p.file_name());
                    if k_parent == s_parent || s_parent.is_none() || k_parent.is_none() {
                        return Some(v.clone());
                    }
                }
            }
        }

        None
    }

    fn read_tcon(song: &Path) -> Vec<String> {
        use id3::TagLike;
        let Ok(tag) = id3::Tag::read_from_path(song) else {
            return Vec::new();
        };
        let raw = tag
            .genre_parsed()
            .map(|c| c.into_owned())
            .or_else(|| tag.genre().map(|g| g.to_string()))
            .unwrap_or_default();
        split_genre_list(&raw)
    }

    /// Genres for one song: sidecar wins, otherwise embedded TCON (cached).
    pub fn genres_for(&mut self, song: &Path) -> Vec<String> {
        let canon_key = Self::canonical_key(song);
        if let Some(g) = self.find_sidecar_genres(song) {
            return g;
        }
        if let Some(g) = self.tcon_cache.get(&canon_key) {
            return g.clone();
        }
        let genres = Self::read_tcon(song);
        self.tcon_cache.insert(canon_key, genres.clone());
        genres
    }

    /// Case-insensitive exact genre membership.
    pub fn matches(&mut self, song: &Path, genre: &str) -> bool {
        let want = genre.trim().to_lowercase();
        self.genres_for(song)
            .iter()
            .any(|g| g.to_lowercase() == want)
    }

    /// Distinct genres with song counts for the given songs slice.
    pub fn all_genres(&mut self, songs: &[PathBuf]) -> Vec<(String, usize)> {
        let mut counts: HashMap<String, (String, usize)> = HashMap::new();
        for song in songs {
            let mut seen_here = std::collections::HashSet::new();
            for g in self.genres_for(song) {
                let lower = g.to_lowercase();
                if seen_here.insert(lower.clone()) {
                    counts
                        .entry(lower)
                        .and_modify(|(_, n)| *n += 1)
                        .or_insert((g, 1));
                }
            }
        }
        let mut out: Vec<(String, usize)> = counts.into_values().collect();
        out.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        out
    }

    /// Returns all known genres across the entire library: combines all tags recorded
    /// in the sidecar database (regardless of whether song is in current playlist)
    /// plus embedded ID3 tags from current playlist songs.
    pub fn all_known_genres(&mut self, current_playlist: &[PathBuf]) -> Vec<(String, usize)> {
        let mut counts: HashMap<String, (String, usize)> = HashMap::new();

        // 1. All genres in the sidecar database
        for genres in self.sidecar.values() {
            let mut seen_here = std::collections::HashSet::new();
            for g in genres {
                let trimmed = g.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let lower = trimmed.to_lowercase();
                if seen_here.insert(lower.clone()) {
                    counts
                        .entry(lower)
                        .and_modify(|(_, n)| *n += 1)
                        .or_insert((trimmed.to_string(), 1));
                }
            }
        }

        // 2. Extra genres found on songs in current playlist not in sidecar
        for song in current_playlist {
            if self.find_sidecar_genres(song).is_some() {
                continue;
            }
            let mut seen_here = std::collections::HashSet::new();
            for g in self.genres_for(song) {
                let trimmed = g.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let lower = trimmed.to_lowercase();
                if seen_here.insert(lower.clone()) {
                    counts
                        .entry(lower)
                        .and_modify(|(_, n)| *n += 1)
                        .or_insert((trimmed.to_string(), 1));
                }
            }
        }

        let mut out: Vec<(String, usize)> = counts.into_values().collect();
        out.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        out
    }

    /// Finds all songs matching any of the specified genres from both current playlist
    /// and the sidecar database (if file exists on disk).
    pub fn songs_matching_genres(&mut self, genres: &[String], current_playlist: &[PathBuf]) -> Vec<PathBuf> {
        let mut result = Vec::new();
        let mut seen = std::collections::HashSet::new();

        // First add matches from the active playlist
        for p in current_playlist {
            if genres.iter().any(|g| self.matches(p, g)) {
                let canon = std::fs::canonicalize(p).unwrap_or_else(|_| p.clone());
                if seen.insert(canon) {
                    result.push(p.clone());
                }
            }
        }

        // Also add matches from the sidecar database that exist on disk
        let sidecar_keys: Vec<String> = self.sidecar.keys().cloned().collect();
        for key in sidecar_keys {
            let p = PathBuf::from(&key);
            if p.is_file() && genres.iter().any(|g| self.matches(&p, g)) {
                let canon = std::fs::canonicalize(&p).unwrap_or_else(|_| p.clone());
                if seen.insert(canon) {
                    result.push(p);
                }
            }
        }

        result
    }

    /// Saves genres for a song to the sidecar JSON file and ID3 tag.
    pub fn set_genres_for_song(&mut self, song: &Path, genres: Vec<String>) -> std::io::Result<()> {
        let canon_key = Self::canonical_key(song);
        let raw_key = song.to_string_lossy().into_owned();
        let cleaned: Vec<String> = genres.into_iter().map(|g| g.trim().to_string()).filter(|g| !g.is_empty()).collect();

        if cleaned.is_empty() {
            self.sidecar.remove(&canon_key);
            self.sidecar.remove(&raw_key);
        } else {
            self.sidecar.insert(canon_key.clone(), cleaned.clone());
            if raw_key != canon_key {
                self.sidecar.insert(raw_key.clone(), cleaned.clone());
            }
        }
        self.tcon_cache.insert(canon_key.clone(), cleaned.clone());
        self.tcon_cache.insert(raw_key, cleaned.clone());

        // 1. Save to sidecar JSON (filter to canonical/clean entries)
        if let Some(path) = sidecar_path() {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let json_text = serde_json::to_string_pretty(&self.sidecar).unwrap_or_default();
            let _ = std::fs::write(path, json_text);
        }

        // 2. Best-effort update ID3 TCON tag in the MP3 file
        if song.is_file() {
            let mut tag = id3::Tag::read_from_path(song).unwrap_or_else(|_| id3::Tag::new());
            if cleaned.is_empty() {
                tag.remove_genre();
            } else {
                tag.set_genre(cleaned.join(", "));
            }
            let _ = tag.write_to_path(song, id3::Version::Id3v24);
        }

        Ok(())
    }
}

/// Cursor & multiple selection state for the genre picker modal (`f`).
pub struct GenrePicker {
    pub genres: Vec<(String, usize)>,
    pub selected: usize,
    pub selected_set: std::collections::HashSet<String>,
}

impl GenrePicker {
    pub fn new(genres: Vec<(String, usize)>) -> Self {
        Self {
            genres,
            selected: 0,
            selected_set: std::collections::HashSet::new(),
        }
    }

    pub fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn move_down(&mut self) {
        // Upper bound is genres.len() which represents the Apply Filter button
        let max_idx = self.genres.len();
        if self.selected < max_idx {
            self.selected += 1;
        }
    }

    pub fn is_apply_button_selected(&self) -> bool {
        self.selected == self.genres.len()
    }

    pub fn toggle_current(&mut self) {
        if let Some((g, _)) = self.genres.get(self.selected) {
            let key = g.to_lowercase();
            if self.selected_set.contains(&key) {
                self.selected_set.remove(&key);
            } else {
                self.selected_set.insert(key);
            }
        }
    }

    pub fn is_selected(&self, genre: &str) -> bool {
        self.selected_set.contains(&genre.to_lowercase())
    }

    #[allow(dead_code)]
    pub fn toggle_all(&mut self) {
        if self.selected_set.len() >= self.genres.len() {
            self.selected_set.clear();
        } else {
            for (g, _) in &self.genres {
                self.selected_set.insert(g.to_lowercase());
            }
        }
    }

    #[allow(dead_code)]
    pub fn selected_genre(&self) -> Option<&str> {
        self.genres.get(self.selected).map(|(g, _)| g.as_str())
    }

    /// Returns list of picked genres. If none explicitly checked with Space/Enter,
    /// falls back to the single item at the cursor.
    pub fn picked_genres(&self) -> Vec<String> {
        if !self.selected_set.is_empty() {
            self.genres
                .iter()
                .filter(|(g, _)| self.selected_set.contains(&g.to_lowercase()))
                .map(|(g, _)| g.clone())
                .collect()
        } else if let Some((g, _)) = self.genres.get(self.selected.min(self.genres.len().saturating_sub(1))) {
            vec![g.clone()]
        } else {
            Vec::new()
        }
    }
}

/// State for tagging / editing genre for a track interactively in Playlist / Browser
pub struct GenreTaggerState {
    pub song_path: PathBuf,
    pub song_title: String,
    pub input: String,
    pub suggestions: Vec<String>,
    pub selected_suggestion: usize,
    pub focus_suggestions: bool,
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_genre_list() {
        assert_eq!(split_genre_list("Hip-Hop/Rap"), vec!["Hip-Hop", "Rap"]);
        assert_eq!(split_genre_list("lo-fi; rain | night"), vec!["lo-fi", "rain", "night"]);
        assert_eq!(split_genre_list("Pop, Rock"), vec!["Pop", "Rock"]);
        assert!(split_genre_list("  ").is_empty());
    }

    #[test]
    fn test_sidecar_wins_over_tcon_and_reload() {
        let dir = std::env::temp_dir().join("lyra-genre-test");
        let lyra_dir = dir.join("lyra");
        let _ = std::fs::create_dir_all(&lyra_dir);
        std::fs::write(lyra_dir.join("genres.json"), r#"{"song": ["side-a", "side-b"], "stray": "x", "bad": 42}"#).unwrap();

        // Point XDG_CONFIG_HOME at our temp dir (restored afterwards).
        let old = std::env::var_os("XDG_CONFIG_HOME");
        unsafe {
            std::env::set_var("XDG_CONFIG_HOME", &dir);
        }

        let mut db = GenreDB::load();
        assert_eq!(db.genres_for(Path::new("song")), vec!["side-a", "side-b"]);
        assert!(db.genres_for(Path::new("untagged")).is_empty());
        assert!(db.matches(Path::new("song"), "SIDE-A"));

        match old {
            Some(v) => unsafe { std::env::set_var("XDG_CONFIG_HOME", v) },
            None => unsafe { std::env::remove_var("XDG_CONFIG_HOME") },
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_all_genres_counts_sorted() {
        // TCON reads need real files; use sidecar-only entries (fake paths OK
        // because sidecar wins before any disk read).
        let mut db = GenreDB {
            sidecar: [
                ("a".to_string(), vec!["Lo-Fi".to_string(), "rain".to_string()]),
                ("b".to_string(), vec!["lo-fi".to_string()]),
                ("c".to_string(), vec!["Jazz".to_string()]),
            ]
            .into_iter()
            .collect(),
            tcon_cache: HashMap::new(),
        };
        let songs = ["a", "b", "c"].iter().map(PathBuf::from).collect::<Vec<_>>();
        let all = db.all_genres(&songs);
        assert_eq!(all[0], ("Lo-Fi".to_string(), 2)); // case-folded count
        assert_eq!(all.len(), 3);
        assert!(db.matches(&PathBuf::from("a"), "RAIN"));
    }

    #[test]
    fn test_genre_picker_multiple_choice() {
        let genres = vec![
            ("Pop".to_string(), 5),
            ("Rock".to_string(), 3),
            ("Jazz".to_string(), 2),
        ];
        let mut picker = GenrePicker::new(genres);
        // Default: nothing selected -> cursor fallback
        assert_eq!(picker.picked_genres(), vec!["Pop".to_string()]);
        assert!(!picker.is_selected("Pop"));

        // Toggle cursor item (Pop)
        picker.toggle_current();
        assert!(picker.is_selected("Pop"));
        assert_eq!(picker.picked_genres(), vec!["Pop".to_string()]);

        // Move to Rock and toggle
        picker.move_down();
        picker.toggle_current();
        assert!(picker.is_selected("Rock"));
        let picked = picker.picked_genres();
        assert_eq!(picked.len(), 2);
        assert!(picked.contains(&"Pop".to_string()));
        assert!(picked.contains(&"Rock".to_string()));

        // Toggle all (currently not all selected, so selects all)
        picker.toggle_all();
        assert_eq!(picker.picked_genres().len(), 3);
        // Toggle all again (all are selected, so clears all)
        picker.toggle_all();
        assert_eq!(picker.selected_set.len(), 0);

        // Test navigation to Apply button
        assert!(!picker.is_apply_button_selected());
        picker.move_down();
        picker.move_down();
        assert!(picker.is_apply_button_selected());
    }

    #[test]
    fn test_all_known_genres_and_fuzzy_path_lookup() {
        let mut db = GenreDB {
            sidecar: [
                ("/home/user/Music/Sub/LIT piano.mp3".to_string(), vec!["Anime".to_string(), "Chill".to_string()]),
            ]
            .into_iter()
            .collect(),
            tcon_cache: HashMap::new(),
        };

        // Even if current_playlist is completely empty, all_known_genres must return Anime & Chill!
        let empty_playlist: Vec<PathBuf> = Vec::new();
        let known = db.all_known_genres(&empty_playlist);
        assert_eq!(known.len(), 2);
        assert!(known.iter().any(|(g, c)| g == "Anime" && *c == 1));
        assert!(known.iter().any(|(g, c)| g == "Chill" && *c == 1));

        // Fuzzy path matching:
        // A relative path with just the file name should still match!
        let rel_path = Path::new("LIT piano.mp3");
        assert!(db.matches(rel_path, "ANIME"));
        assert!(db.matches(rel_path, "CHILL"));
        assert!(!db.matches(rel_path, "POP"));

        // Recorded genres check
        let rec = db.recorded_genres();
        assert_eq!(rec.len(), 2);
        assert!(rec.contains(&"Anime".to_string()));
        assert!(rec.contains(&"Chill".to_string()));
    }
}
