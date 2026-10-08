use crate::genre::{GenrePicker, GenreTaggerState};

use super::{App, ViewMode, short_name};

impl App {
    /// Visible playlist indices: `/` search ANDed with the multiple genre filters.
    pub fn visible_playlist_indices(&mut self) -> Vec<usize> {
        let query = self.search_query.clone();
        let genres = self.genre_filters.clone();
        self.playlist
            .filtered_indices(&query, &genres, &mut self.genre_db)
    }

    /// `f`: open the genre picker (reloads sidecar so external edits show up).
    pub fn open_genre_picker(&mut self) {
        self.genre_db.reload_sidecar();
        let genres = self.genre_db.all_known_genres(&self.playlist.songs);
        if genres.is_empty() {
            self.set_toast("No genres yet. Press 't' on any song to add tags, or run: lyra fill-genres".to_string());
            return;
        }
        self.genre_picker = Some(GenrePicker::new(genres));
    }

    /// `Enter` in the picker: enqueue every song matching any of the picked genres.
    pub fn enqueue_picked_genre(&mut self) {
        let Some(picker) = self.genre_picker.take() else {
            return;
        };
        let genres = picker.picked_genres();
        if genres.is_empty() {
            return;
        }
        let hits = self.genre_db.songs_matching_genres(&genres, &self.playlist.songs);
        let n = hits.len();
        for h in hits {
            self.queue.enqueue_back(h);
        }
        self.queue.persist();
        let g_str = genres.join(", ");
        self.set_toast(if n == 0 {
            format!("No songs found for [{g_str}]")
        } else {
            format!("Queued {n} songs matching [{g_str}]")
        });
    }

    /// `f` in the picker: filter the Playlist view to the
    /// picked genres (multiple choice OR match). Esc clears the filter.
    pub fn filter_by_picked_genre(&mut self) {
        let Some(picker) = self.genre_picker.take() else {
            return;
        };
        let genres = picker.picked_genres();
        if genres.is_empty() {
            return;
        }
        let g_str = genres.join(", ");

        // If current playlist doesn't contain matching songs, import matching songs from sidecar
        let has_local_matches = self.playlist.songs.iter().any(|p| genres.iter().any(|g| self.genre_db.matches(p, g)));
        if !has_local_matches {
            let matched_songs = self.genre_db.songs_matching_genres(&genres, &[]);
            if !matched_songs.is_empty() {
                self.playlist.append_songs(matched_songs);
            }
        }

        self.genre_filters = genres;
        // Keep the cursor on something visible after filtering.
        let indices = self.visible_playlist_indices();
        if let Some(&first) = indices.first() {
            self.playlist.selected = first;
        }
        self.set_toast(format!("Genre filter: [{g_str}] (Esc clears)"));
    }

    /// Clears the genre filter. Returns true when one was active.
    pub fn clear_genre_filter(&mut self) -> bool {
        if !self.genre_filters.is_empty() {
            self.genre_filters.clear();
            self.set_toast("Genre filter cleared".to_string());
            true
        } else {
            false
        }
    }

    /// `t`: Opens the interactive Genre Tagger / Editor for the selected song
    pub fn open_genre_tagger(&mut self) {
        let target_path = match self.view_mode {
            ViewMode::Playlist => self.playlist.current_selected_song(),
            ViewMode::Browser => {
                if let Some(entry) = self.browser.entries.get(self.browser.selected).cloned() {
                    if entry.is_file() && entry.extension().is_some_and(|e| e.eq_ignore_ascii_case("mp3")) {
                        Some(entry)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            _ => None,
        };

        let Some(song_path) = target_path else {
            self.set_toast("Select an MP3 song first to tag genre (press 't')".to_string());
            return;
        };

        let song_title = short_name(&song_path);
        let existing = self.genre_db.genres_for(&song_path);
        let input = existing.join(", ");

        // Build suggestions:
        // 1. Any genre previously recorded in sidecar or library
        let mut suggestions = Vec::new();
        let mut seen = std::collections::HashSet::new();

        self.genre_db.reload_sidecar();
        for g in self.genre_db.recorded_genres() {
            let trimmed = g.trim();
            if !trimmed.is_empty() && seen.insert(trimmed.to_lowercase()) {
                suggestions.push(trimmed.to_string());
            }
        }

        for (g, _) in self.genre_db.all_genres(&self.playlist.songs) {
            let trimmed = g.trim();
            if !trimmed.is_empty() && seen.insert(trimmed.to_lowercase()) {
                suggestions.push(trimmed.to_string());
            }
        }

        // 2. Curated simple, standard tags (basic genres, retaining Anime, Lo-Fi, Chill...)
        let defaults = [
            "Anime", "Lo-Fi", "Chill", "Pop", "Ballad", "Acoustic", "Indie", "R&B", "EDM"
        ];
        for d in defaults {
            if seen.insert(d.to_lowercase()) {
                suggestions.push(d.to_string());
            }
        }

        self.genre_tagger = Some(GenreTaggerState {
            song_path,
            song_title,
            input,
            suggestions,
            selected_suggestion: 0,
            focus_suggestions: false,
        });
    }

    /// Confirms and saves tags in the genre tagger modal
    pub fn save_genre_tagger(&mut self) {
        if let Some(tagger) = self.genre_tagger.take() {
            let tags: Vec<String> = tagger
                .input
                .split([',', ';', '|'])
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            let name = tagger.song_title;
            let tag_summary = if tags.is_empty() {
                "Cleared all genres".to_string()
            } else {
                tags.join(", ")
            };

            let _ = self.genre_db.set_genres_for_song(&tagger.song_path, tags);
            self.set_toast(format!("Updated genres for '{name}': [{tag_summary}]"));
        }
    }

    /// Appends the highlighted suggestion chip to the tagger input box
    pub fn append_suggestion_to_genre_tagger(&mut self) {
        if let Some(tagger) = self.genre_tagger.as_mut() {
            if let Some(sug) = tagger.suggestions.get(tagger.selected_suggestion) {
                let trimmed = tagger.input.trim_end();
                if trimmed.is_empty() {
                    tagger.input = sug.clone();
                } else if trimmed.ends_with(',') || trimmed.ends_with(';') {
                    tagger.input = format!("{trimmed} {sug}");
                } else {
                    tagger.input = format!("{trimmed}, {sug}");
                }
                tagger.focus_suggestions = false;
            }
        }
    }
}
