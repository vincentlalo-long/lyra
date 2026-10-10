use std::path::{Path, PathBuf};
use ratatui::widgets::ListState;
#[cfg(feature = "genre")]
use crate::genre::GenreDB;

pub struct Playlist {
    pub songs: Vec<PathBuf>,
    pub selected: usize,
    pub playing_index: Option<usize>,
    pub state: ListState,
}

impl Playlist {
    pub fn new(music_folder: &Path) -> Self {
        let songs = if music_folder.exists() {
            find_mp3s_in_dir(music_folder, false)
        } else {
            Vec::new()
        };

        Self {
            songs,
            selected: 0,
            playing_index: None,
            state: ListState::default(),
        }
    }

    /// Visible song indices: `/` substring filter, ANDed with the active
    /// genre filter when the `genre` plugin is compiled in.
    #[cfg(feature = "genre")]
    pub fn filtered_indices(
        &self,
        query: &str,
        genres: &[String],
        db: &mut GenreDB,
    ) -> Vec<usize> {
        let lower_query = query.to_lowercase();
        self.songs
            .iter()
            .enumerate()
            .filter(|(_, p)| {
                let name_ok = query.is_empty()
                    || p.file_name()
                        .map(|n| n.to_string_lossy().to_lowercase().contains(&lower_query))
                        .unwrap_or(false);
                let genre_ok = genres.is_empty()
                    || genres.iter().any(|g| db.matches(p, g));
                name_ok && genre_ok
            })
            .map(|(i, _)| i)
            .collect()
    }

    /// Visible song indices: `/` substring filter only (no genre plugin).
    #[cfg(not(feature = "genre"))]
    pub fn filtered_indices(&self, query: &str) -> Vec<usize> {
        if query.is_empty() {
            (0..self.songs.len()).collect()
        } else {
            let lower_query = query.to_lowercase();
            self.songs
                .iter()
                .enumerate()
                .filter(|(_, p)| {
                    p.file_name()
                        .map(|n| n.to_string_lossy().to_lowercase().contains(&lower_query))
                        .unwrap_or(false)
                })
                .map(|(i, _)| i)
                .collect()
        }
    }

    /// Moves the cursor by `delta` inside a precomputed visible set.
    /// `wrap` cycles at the ends (next/previous), otherwise clamps (paging).
    fn move_by(&mut self, indices: &[usize], delta: isize, wrap: bool) {
        if indices.is_empty() {
            return;
        }
        let len = indices.len();
        let cur = indices.iter().position(|&i| i == self.selected).unwrap_or(0);
        let pos = if wrap {
            (cur as isize + delta).rem_euclid(len as isize) as usize
        } else if delta >= 0 {
            (cur + delta as usize).min(len - 1)
        } else {
            cur.saturating_sub((-delta) as usize)
        };
        self.selected = indices[pos];
    }

    #[cfg(feature = "genre")]
    pub fn next(&mut self, query: &str, genres: &[String], db: &mut GenreDB) {
        let indices = self.filtered_indices(query, genres, db);
        self.move_by(&indices, 1, true);
    }

    #[cfg(not(feature = "genre"))]
    pub fn next(&mut self, query: &str) {
        let indices = self.filtered_indices(query);
        self.move_by(&indices, 1, true);
    }

    #[cfg(feature = "genre")]
    pub fn previous(&mut self, query: &str, genres: &[String], db: &mut GenreDB) {
        let indices = self.filtered_indices(query, genres, db);
        self.move_by(&indices, -1, true);
    }

    #[cfg(not(feature = "genre"))]
    pub fn previous(&mut self, query: &str) {
        let indices = self.filtered_indices(query);
        self.move_by(&indices, -1, true);
    }

    #[cfg(feature = "genre")]
    pub fn page_down(&mut self, query: &str, step: usize, genres: &[String], db: &mut GenreDB) {
        let indices = self.filtered_indices(query, genres, db);
        self.move_by(&indices, step as isize, false);
    }

    #[cfg(not(feature = "genre"))]
    pub fn page_down(&mut self, query: &str, step: usize) {
        let indices = self.filtered_indices(query);
        self.move_by(&indices, step as isize, false);
    }

    #[cfg(feature = "genre")]
    pub fn page_up(&mut self, query: &str, step: usize, genres: &[String], db: &mut GenreDB) {
        let indices = self.filtered_indices(query, genres, db);
        self.move_by(&indices, -(step as isize), false);
    }

    #[cfg(not(feature = "genre"))]
    pub fn page_up(&mut self, query: &str, step: usize) {
        let indices = self.filtered_indices(query);
        self.move_by(&indices, -(step as isize), false);
    }

    #[cfg(feature = "genre")]
    pub fn first(&mut self, query: &str, genres: &[String], db: &mut GenreDB) {
        let indices = self.filtered_indices(query, genres, db);
        if let Some(&first) = indices.first() {
            self.selected = first;
        }
    }

    #[cfg(not(feature = "genre"))]
    pub fn first(&mut self, query: &str) {
        let indices = self.filtered_indices(query);
        if let Some(&first) = indices.first() {
            self.selected = first;
        }
    }

    #[cfg(feature = "genre")]
    pub fn last(&mut self, query: &str, genres: &[String], db: &mut GenreDB) {
        let indices = self.filtered_indices(query, genres, db);
        if let Some(&last) = indices.last() {
            self.selected = last;
        }
    }

    #[cfg(not(feature = "genre"))]
    pub fn last(&mut self, query: &str) {
        let indices = self.filtered_indices(query);
        if let Some(&last) = indices.last() {
            self.selected = last;
        }
    }

    pub fn select_and_mark_playing(&mut self, song_path: &Path) {
        if let Some(index) = self.songs.iter().position(|p| p == song_path) {
            self.playing_index = Some(index);
            self.selected = index;
        } else {
            self.songs.push(song_path.to_path_buf());
            let new_index = self.songs.len() - 1;
            self.playing_index = Some(new_index);
            self.selected = new_index;
        }
    }

    pub fn current_selected_song(&self) -> Option<PathBuf> {
        self.songs.get(self.selected).cloned()
    }

    pub fn next_track_path(&mut self) -> Option<PathBuf> {
        if self.songs.is_empty() {
            return None;
        }

        let next_index = match self.playing_index {
            Some(current) => (current + 1) % self.songs.len(),
            None => 0,
        };

        let path = self.songs[next_index].clone();
        self.playing_index = Some(next_index);
        self.selected = next_index;
        Some(path)
    }

    pub fn prev_track_path(&mut self) -> Option<PathBuf> {
        if self.songs.is_empty() {
            return None;
        }

        let prev_index = match self.playing_index {
            Some(current) => {
                if current == 0 {
                    self.songs.len() - 1
                } else {
                    current - 1
                }
            }
            None => 0,
        };

        let path = self.songs[prev_index].clone();
        self.playing_index = Some(prev_index);
        self.selected = prev_index;
        Some(path)
    }

    /// Advance `playing_index` inside a precomputed visible (filtered) set.
    /// A playing track outside the set restarts from the first visible one.
    /// Returns `None` when the visible set is empty.
    pub fn next_track_path_filtered(&mut self, indices: &[usize]) -> Option<PathBuf> {
        if self.songs.is_empty() || indices.is_empty() {
            return None;
        }
        let next_index = match self.playing_index {
            Some(cur) => match indices.iter().position(|&i| i == cur) {
                Some(pos) => indices[(pos + 1) % indices.len()],
                None => indices[0],
            },
            None => indices[0],
        };
        let path = self.songs.get(next_index)?.clone();
        self.playing_index = Some(next_index);
        self.selected = next_index;
        Some(path)
    }

    /// Step `playing_index` backwards inside a visible (filtered) set.
    pub fn prev_track_path_filtered(&mut self, indices: &[usize]) -> Option<PathBuf> {
        if self.songs.is_empty() || indices.is_empty() {
            return None;
        }
        let prev_index = match self.playing_index {
            Some(cur) => match indices.iter().position(|&i| i == cur) {
                Some(pos) => indices[(pos + indices.len() - 1) % indices.len()],
                None => indices[0],
            },
            None => indices[0],
        };
        let path = self.songs.get(prev_index)?.clone();
        self.playing_index = Some(prev_index);
        self.selected = prev_index;
        Some(path)
    }

    /// True when the playing track is the last of the visible set
    /// (used by RepeatMode::Off to stop instead of leaking into
    /// hidden tracks).
    pub fn playing_is_last_visible(&self, indices: &[usize]) -> bool {
        match self.playing_index {
            Some(cur) => indices.iter().position(|&i| i == cur).is_some_and(|pos| pos + 1 >= indices.len()),
            None => false,
        }
    }

    pub fn set_songs(&mut self, songs: Vec<PathBuf>) {
        self.songs = songs;
        self.selected = 0;
        self.playing_index = None;
        self.state = ListState::default();
    }

    /// Appends new songs to the playlist without adding duplicates.
    pub fn append_songs(&mut self, new_songs: Vec<PathBuf>) {
        for song in new_songs {
            if !self.songs.contains(&song) {
                self.songs.push(song);
            }
        }
    }

    /// Inserts one song (sorted, deduped) while keeping `playing_index`
    /// and `selected` glued to the same tracks by path identity — so a
    /// background download landing mid-playback never shifts the cursor
    /// or the now-playing highlight. Returns true when newly added.
    #[cfg_attr(not(feature = "download"), allow(dead_code))]
    pub fn insert_sorted(&mut self, path: PathBuf) -> bool {
        if self.songs.contains(&path) {
            return false;
        }
        let playing = self.playing_index.and_then(|i| self.songs.get(i).cloned());
        let selected = self.songs.get(self.selected).cloned();
        self.songs.push(path);
        self.songs.sort();
        if let Some(p) = playing {
            self.playing_index = self.songs.iter().position(|s| s == &p);
        }
        if let Some(s) = selected {
            if let Some(pos) = self.songs.iter().position(|x| x == &s) {
                self.selected = pos;
            }
        }
        true
    }

    /// Removes the currently selected song from the playlist.
    /// Adjusts `playing_index` and `selected` appropriately.
    pub fn remove_selected(&mut self) -> Option<PathBuf> {
        if self.songs.is_empty() || self.selected >= self.songs.len() {
            return None;
        }

        let removed = self.songs.remove(self.selected);

        if let Some(playing) = self.playing_index {
            if playing == self.selected {
                self.playing_index = None;
            } else if playing > self.selected {
                self.playing_index = Some(playing - 1);
            }
        }

        if !self.songs.is_empty() {
            if self.selected >= self.songs.len() {
                self.selected = self.songs.len() - 1;
            }
        } else {
            self.selected = 0;
            self.playing_index = None;
        }

        Some(removed)
    }
}

/// Finds all .mp3 files in `dir`. If `recursive` is true, scans subdirectories.
pub fn find_mp3s_in_dir(dir: &Path, recursive: bool) -> Vec<PathBuf> {
    let mut results = Vec::new();
    if !dir.is_dir() {
        return results;
    }

    if !recursive {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file() && p.extension().is_some_and(|e| e.eq_ignore_ascii_case("mp3")) {
                    results.push(p);
                }
            }
        }
    } else {
        let mut stack = vec![dir.to_path_buf()];
        while let Some(current) = stack.pop() {
            if let Ok(entries) = std::fs::read_dir(&current) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    let name = p.file_name().map(|n| n.to_string_lossy()).unwrap_or_default();
                    if name.starts_with('.') || name == "target" || name == "node_modules" {
                        continue;
                    }
                    if p.is_dir() {
                        stack.push(p);
                    } else if p.is_file() && p.extension().is_some_and(|e| e.eq_ignore_ascii_case("mp3")) {
                        results.push(p);
                    }
                }
            }
        }
    }

    results.sort();
    results
}

/// Loads and resolves valid track paths from an .m3u / .m3u8 playlist file.
/// Relative paths inside the playlist are resolved relative to the playlist's parent folder.
pub fn load_m3u(path: &Path) -> Vec<PathBuf> {
    let Ok(content) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    let parent = path.parent().unwrap_or(Path::new("."));
    content
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| {
            let p = PathBuf::from(l);
            if p.is_relative() {
                parent.join(p)
            } else {
                p
            }
        })
        .filter(|p| p.is_file())
        .collect()
}


#[cfg(test)]    mod tests {
    use super::*;

    fn make_playlist() -> Playlist {
        Playlist {
            songs: (0..50).map(|i| PathBuf::from(format!("song_{:02}.mp3", i))).collect(),
            selected: 0,
            playing_index: None,
            state: ListState::default(),
        }
    }

    #[test]
    fn test_move_by_wrap_and_clamp() {
        // Feature-agnostic cursor core (shared by both configurations).
        let mut playlist = make_playlist();
        let all: Vec<usize> = (0..50).collect();

        playlist.move_by(&all, 1, true);
        assert_eq!(playlist.selected, 1);
        playlist.move_by(&all, -1, true);
        assert_eq!(playlist.selected, 0);
        // Wrap at both ends.
        playlist.move_by(&all, -1, true);
        assert_eq!(playlist.selected, 49);
        playlist.move_by(&all, 1, true);
        assert_eq!(playlist.selected, 0);
        // Clamp without wrap (paging).
        playlist.move_by(&all, 10, false);
        assert_eq!(playlist.selected, 10);
        playlist.move_by(&all, 100, false);
        assert_eq!(playlist.selected, 49);
        playlist.move_by(&all, -100, false);
        assert_eq!(playlist.selected, 0);
        // Empty set + cursor outside the set.
        playlist.move_by(&[], 1, true);
        assert_eq!(playlist.selected, 0);
        playlist.selected = 49;
        playlist.move_by(&[5, 7], 1, true);
        assert_eq!(playlist.selected, 7);
    }

    #[cfg(feature = "genre")]
    #[test]
    fn test_navigation_and_genre_filter() {
        use crate::genre::GenreDB;

        let mut playlist = make_playlist();
        let mut db = GenreDB::new_empty_for_tests();
        let empty_genres: &[String] = &[];

        playlist.next("", empty_genres, &mut db);
        assert_eq!(playlist.selected, 1);
        playlist.previous("", empty_genres, &mut db);
        assert_eq!(playlist.selected, 0);
        playlist.previous("", empty_genres, &mut db);
        assert_eq!(playlist.selected, 49);
        playlist.next("", empty_genres, &mut db);
        assert_eq!(playlist.selected, 0);
        playlist.page_down("", 10, empty_genres, &mut db);
        assert_eq!(playlist.selected, 10);
        playlist.page_up("", 10, empty_genres, &mut db);
        assert_eq!(playlist.selected, 0);
        playlist.last("", empty_genres, &mut db);
        assert_eq!(playlist.selected, 49);
        playlist.first("", empty_genres, &mut db);
        assert_eq!(playlist.selected, 0);

        // Genre filter narrows navigation to matching songs only.
        let mut gdb = GenreDB::with_sidecar_entries_for_tests([
            ("song_05.mp3".to_string(), vec!["jazz".to_string()]),
            ("song_07.mp3".to_string(), vec!["jazz".to_string()]),
        ]);
        let jazz = vec!["jazz".to_string()];
        playlist.first("", &jazz, &mut gdb);
        assert_eq!(playlist.selected, 5);
        playlist.next("", &jazz, &mut gdb);
        assert_eq!(playlist.selected, 7);
        playlist.next("", &jazz, &mut gdb);
        assert_eq!(playlist.selected, 5);
    }

    #[cfg(not(feature = "genre"))]
    #[test]
    fn test_navigation_no_genre() {
        let mut playlist = make_playlist();

        playlist.next("");
        assert_eq!(playlist.selected, 1);
        playlist.previous("");
        assert_eq!(playlist.selected, 0);
        playlist.previous("");
        assert_eq!(playlist.selected, 49);
        playlist.next("");
        assert_eq!(playlist.selected, 0);
        playlist.page_down("", 10);
        assert_eq!(playlist.selected, 10);
        playlist.page_up("", 10);
        assert_eq!(playlist.selected, 0);
        playlist.first("");
        assert_eq!(playlist.selected, 0);
    }

    #[test]
    fn test_append_songs_deduplicates() {        let mut playlist = make_playlist();
        let initial_len = playlist.songs.len();
        // Append an existing song and a new song
        playlist.append_songs(vec![
            PathBuf::from("song_00.mp3"),
            PathBuf::from("song_new.mp3"),
        ]);
        assert_eq!(playlist.songs.len(), initial_len + 1);
        assert_eq!(playlist.songs.last(), Some(&PathBuf::from("song_new.mp3")));
    }

    #[cfg(feature = "download")]
    #[test]
    fn test_insert_sorted_preserves_cursors_by_identity() {
        let mut playlist = Playlist {
            songs: vec![PathBuf::from("a.mp3"), PathBuf::from("c.mp3"), PathBuf::from("e.mp3")],
            selected: 1, // c.mp3
            playing_index: Some(1), // c.mp3
            state: ListState::default(),
        };
        // New download sorts BEFORE the playing track.
        assert!(playlist.insert_sorted(PathBuf::from("b.mp3")));
        assert_eq!(
            playlist.songs,
            vec![
                PathBuf::from("a.mp3"),
                PathBuf::from("b.mp3"),
                PathBuf::from("c.mp3"),
                PathBuf::from("e.mp3"),
            ]
        );
        assert_eq!(playlist.selected, 2, "cursor must follow c.mp3");
        assert_eq!(playlist.playing_index, Some(2), "now-playing must follow c.mp3");
        // Duplicates are ignored without touching cursors.
        assert!(!playlist.insert_sorted(PathBuf::from("b.mp3")));
        assert_eq!(playlist.selected, 2);
        assert_eq!(playlist.playing_index, Some(2));
    }

    #[test]
    fn test_load_m3u_resolves_paths_and_ignores_comments() {
        let temp = std::env::temp_dir().join("lyra_m3u_test");
        let _ = std::fs::create_dir_all(&temp);
        let track1 = temp.join("track1.mp3");
        let track2 = temp.join("track2.mp3");
        let _ = std::fs::write(&track1, b"a");
        let _ = std::fs::write(&track2, b"b");

        let m3u_path = temp.join("playlist.m3u");
        let m3u_content = format!(
            "#EXTM3U\n# Comment\ntrack1.mp3\n{}\nnonexistent.mp3\n",
            track2.to_str().unwrap()
        );
        let _ = std::fs::write(&m3u_path, m3u_content);

        let loaded = load_m3u(&m3u_path);
        assert_eq!(loaded, vec![track1, track2]);

        let _ = std::fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_find_mp3s_in_dir() {
        let temp = std::env::temp_dir().join("lyra_find_test");
        let sub = temp.join("sub");
        let _ = std::fs::create_dir_all(&sub);
        let song1 = temp.join("a.mp3");
        let song2 = sub.join("b.mp3");
        let not_song = temp.join("readme.txt");
        let _ = std::fs::write(&song1, b"a");
        let _ = std::fs::write(&song2, b"b");
        let _ = std::fs::write(&not_song, b"c");

        // Non-recursive: only direct children
        let direct = find_mp3s_in_dir(&temp, false);
        assert_eq!(direct, vec![song1.clone()]);

        // Recursive: all children
        let all = find_mp3s_in_dir(&temp, true);
        assert_eq!(all, vec![song1, song2]);

        let _ = std::fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_remove_selected() {
        let mut playlist = Playlist {
            songs: vec![
                PathBuf::from("a.mp3"),
                PathBuf::from("b.mp3"),
                PathBuf::from("c.mp3"),
            ],
            selected: 1,
            playing_index: Some(2),
            state: ListState::default(),
        };

        // Remove middle item ("b.mp3")
        let removed = playlist.remove_selected();
        assert_eq!(removed, Some(PathBuf::from("b.mp3")));
        assert_eq!(playlist.songs.len(), 2);
        assert_eq!(playlist.selected, 1); // cursor now points to "c.mp3"
        assert_eq!(playlist.playing_index, Some(1)); // playing_index adjusted from 2 to 1

        // Remove current playing item ("c.mp3" at index 1)
        let removed2 = playlist.remove_selected();
        assert_eq!(removed2, Some(PathBuf::from("c.mp3")));
        assert_eq!(playlist.songs.len(), 1);
        assert_eq!(playlist.selected, 0); // clamped to last available index
        assert_eq!(playlist.playing_index, None); // playing item was removed

        // Remove the last remaining item ("a.mp3" at index 0)
        let removed3 = playlist.remove_selected();
        assert_eq!(removed3, Some(PathBuf::from("a.mp3")));
        assert_eq!(playlist.songs.len(), 0);
        assert_eq!(playlist.selected, 0);

        // Removing from empty playlist returns None
        assert_eq!(playlist.remove_selected(), None);
    }

    #[test]
    fn test_playlist_new_only_direct_files() {
        let temp = std::env::temp_dir().join("lyra_playlist_new_direct");
        let album = temp.join("Album A");
        let _ = std::fs::create_dir_all(&album);
        let track1 = temp.join("root.mp3");
        let track2 = album.join("sub.mp3");
        let _ = std::fs::write(&track1, b"a");
        let _ = std::fs::write(&track2, b"b");

        let playlist = Playlist::new(&temp);
        assert_eq!(playlist.songs.len(), 1);
        assert!(playlist.songs.contains(&track1));
        assert!(!playlist.songs.contains(&track2), "Subdirectory songs should not clutter root playlist on startup");

        let _ = std::fs::remove_dir_all(&temp);
    }
}

