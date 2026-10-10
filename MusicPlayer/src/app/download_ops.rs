use std::path::PathBuf;
use crate::download::{DownloadEvent, DownloadRequest};

use super::App;

impl App {
    pub fn start_metadata_download(&mut self) {
        if !self.download.show_metadata_form {
            return;
        }
        let custom_cover = if let Some(local_path) = self.download.form_custom_cover.as_ref().filter(|s| !s.is_empty()) {
            Some(local_path.clone())
        } else {
            self.download
                .selected_cover
                .and_then(|i| self.download.cover_candidates.get(i))
                .map(|c| c.cover_url.clone())
                .filter(|u| !u.is_empty())
        };
        let req = DownloadRequest {
            url: self.download.form_url.clone(),
            output_dir: self.download.form_dir.clone(),
            title: self.download.form_title.clone(),
            artist: self.download.form_artist.clone(),
            album: self.download.form_album.clone(),
            genre: if self.download.form_genre.trim().is_empty() { None } else { Some(self.download.form_genre.trim().to_string()) },
            cover_mode: self.download.form_cover_mode.clone(),
            cover_source: self.download.cover_source.clone(),
            cover_focus: self.download.cover_focus,
            cover_url: custom_cover.clone(),
            custom_cover,
            no_lyrics: self.download.form_lyrics_mode == 2,
            no_auto_lyrics: self.download.form_lyrics_mode == 1,
        };
        self.download.active_title = req.title.clone();
        self.download.active_artist = req.artist.clone();
        self.download.is_downloading = true;
        self.download.reset_progress();
        self.download.speed.clear();
        self.download.eta.clear();
        self.download.current_stage = "Connecting to YouTube...".into();
        self.download.last_error = None;
        self.download.last_completed = None;
        self.download.show_metadata_form = false;
        self.downloader.start_download(req);
    }

    pub fn collect_existing_artists(&self) -> Vec<String> {
        let mut set = std::collections::BTreeSet::new();

        // 1. Current playlist songs
        for p in &self.playlist.songs {
            let meta = if let Some(cached) = self.meta_cache.get(p) {
                cached.clone()
            } else {
                crate::meta::song_meta(p)
            };
            let art = meta.artist.trim();
            if !art.is_empty() && !art.eq_ignore_ascii_case("unknown") {
                set.insert(art.to_string());
            }
        }

        // 2. All audio files in the music library directory (recursive scan)
        let lib_songs = crate::playlist::find_mp3s_in_dir(&self.music_folder, true);
        for p in &lib_songs {
            let meta = if let Some(cached) = self.meta_cache.get(p) {
                cached.clone()
            } else {
                crate::meta::song_meta(p)
            };
            let art = meta.artist.trim();
            if !art.is_empty() && !art.eq_ignore_ascii_case("unknown") {
                set.insert(art.to_string());
            }
        }

        // 3. Audio files in current browser directory if different from music_folder
        if self.browser.current_dir != self.music_folder {
            let browser_songs = crate::playlist::find_mp3s_in_dir(&self.browser.current_dir, true);
            for p in &browser_songs {
                let meta = if let Some(cached) = self.meta_cache.get(p) {
                    cached.clone()
                } else {
                    crate::meta::song_meta(p)
                };
                let art = meta.artist.trim();
                if !art.is_empty() && !art.eq_ignore_ascii_case("unknown") {
                    set.insert(art.to_string());
                }
            }
        }

        set.into_iter().collect()
    }

    pub fn collect_existing_albums(&self) -> Vec<String> {
        let mut set = std::collections::BTreeSet::new();

        // 1. Scanned songs in playlist
        for p in &self.playlist.songs {
            let meta = if let Some(cached) = self.meta_cache.get(p) {
                cached.clone()
            } else {
                crate::meta::song_meta(p)
            };
            let alb = meta.album.trim();
            if !alb.is_empty() && !alb.ends_with(" - Single") && !alb.eq_ignore_ascii_case("unknown") {
                set.insert(alb.to_string());
            }
        }

        // 2. Subdirectories in music_folder only if they actually contain audio files
        let ignore_dirs = [
            "download", "downloads", "output", "temp", "tmp", "dist",
            "target", "src", "bin", "tests", "plugins", "node_modules",
        ];
        if let Ok(entries) = std::fs::read_dir(&self.music_folder) {
            for entry in entries.flatten() {
                if let Ok(ft) = entry.file_type() {
                    if ft.is_dir() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        if !name.starts_with('.') && !ignore_dirs.iter().any(|ig| name.eq_ignore_ascii_case(ig)) {
                            if let Ok(sub) = std::fs::read_dir(entry.path()) {
                                let has_audio = sub.flatten().any(|e| {
                                    e.path()
                                        .extension()
                                        .and_then(|ext| ext.to_str())
                                        .map(|ext| matches!(ext.to_ascii_lowercase().as_str(), "mp3" | "flac" | "m4a" | "wav" | "ogg"))
                                        .unwrap_or(false)
                                });
                                if has_audio {
                                    set.insert(name);
                                }
                            }
                        }
                    }
                }
            }
        }

        set.into_iter().collect()
    }

    pub fn collect_suggested_genres(&mut self) -> Vec<String> {
        let mut list = Vec::new();

        #[cfg(feature = "genre")]
        {
            let known = self.genre_db.all_known_genres(&self.playlist.songs);
            for (g, _) in known {
                if !list.iter().any(|existing: &String| existing.eq_ignore_ascii_case(&g)) {
                    list.push(g);
                }
            }
        }

        let defaults = [
            "Pop", "Anime", "Rock", "J-Pop", "V-Pop", "Ballad", "Lo-fi",
            "Electronic", "R&B", "Hip-Hop", "Acoustic", "Jazz",
        ];
        for d in defaults {
            if !list.iter().any(|existing: &String| existing.eq_ignore_ascii_case(d)) {
                list.push(d.to_string());
            }
        }

        list
    }

    /// Open the metadata form and kick off background studio-cover search
    /// (termusic songtag-style: user reviews metadata while candidates load).
    fn open_metadata_form(&mut self, url: String, title: String, artist: String) {
        self.download.existing_artists = self.collect_existing_artists();
        self.download.selected_artist_idx = None;
        self.download.existing_albums = self.collect_existing_albums();
        self.download.selected_album_idx = None;
        self.download.suggested_genres = self.collect_suggested_genres();
        self.download.selected_genre_idx = None;

        self.download.form_url = url;
        self.download.form_title = title.clone();
        self.download.form_artist = artist.clone();
        self.download.form_album = format!("{title} - Single");
        self.download.form_genre.clear();
        self.download.show_metadata_form = true;
        self.download.form_field_idx = 0;
        self.download.snap_cursor_to_end();
        self.download.form_custom_cover = None;
        self.download.show_cover_picker_modal = false;
        self.download.show_cover_file_picker = false;
        self.download.cover_picker_search = format!("{artist} {title}");
        self.download.cover_picker_selected = 0;
        self.download.cover_preview_art = None;
        self.download.cover_preview_path = None;
        self.download.cover_candidates.clear();
        self.download.selected_cover = None;
        self.download.is_cover_loading = true;
        self.download.lyrics_probe = None;
        let src = self.download.cover_source.clone();
        self.downloader.fetch_covers_query(artist, title, None, Some(src));
        // Background subtitle probe for the Lyrics-row badge (silent on failure).
        self.downloader.probe_lyrics(self.download.form_url.clone());
    }

    pub fn handle_download_enter(&mut self) {
        if !self.plugin_enabled("download") {
            self.set_toast("Downloader plugin is disabled (enable it in [5] Plugins)".to_string());
            return;
        }
        if self.download.show_dir_picker {
            if let Some(selected_dir) = self.download.dir_picker.enter() {
                let path_str = selected_dir.to_string_lossy().to_string();
                self.download.form_dir = selected_dir;
                self.download.show_dir_picker = false;
                if let Some(item) = self.config_paths.iter_mut().find(|p| p.id == "download_dir") {
                    item.path = path_str.clone();
                }
                let _ = crate::config::Config::save_setting("download_dir", &path_str);
            }
        } else if self.download.show_metadata_form {
            if self.download.form_field_idx == 4 {
                // Save to field: open dir picker
                self.download.dir_picker = crate::browser::FileBrowser::new(&self.download.form_dir);
                self.download.show_dir_picker = true;
            } else if self.download.form_field_idx == 5 {
                // Cover field: open cover modal
                self.download.cover_picker_selected = match self.download.selected_cover {
                    Some(i) => i + 1,
                    None => if self.download.form_custom_cover.is_some() { 0 } else { 1 },
                };
                self.download.cover_picker_input_active = true;
                self.download.update_cover_preview();
                self.download.show_cover_picker_modal = true;
            } else if self.download.form_field_idx == 6 {
                self.download.form_field_idx = 7;
            } else if self.download.form_field_idx < 7 {
                // Advance to next field on Enter
                self.download.form_field_idx += 1;
            } else {
                // Field 7: Start download!
                self.start_metadata_download();
            }
        } else if self.download.is_input_active {
            let q = self.download.query.trim().to_string();
            if q.is_empty() {
                return;
            }
            if q.starts_with("http://") || q.starts_with("https://") || q.contains("youtu.be") {
                self.download.is_searching = true;
                self.download.is_input_active = false;
                self.downloader.fetch_info(q);
            } else {
                self.download.is_searching = true;
                self.download.is_input_active = false;
                let limit = self.download.search_limit;
                self.downloader.search_with_limit(q, limit);
            }
        } else if !self.download.search_results.is_empty() {
            let item = self.download.search_results[self.download.selected_result].clone();
            let url = item.url.clone();
            let title = item.title.clone();
            let artist = item.artist.clone();
            self.open_metadata_form(url, title, artist);
        }
    }

    /// Expands search results by +10 items (up to 50) and refetches from YouTube
    pub fn expand_download_search(&mut self) {
        let q = self.download.query.trim().to_string();
        if q.is_empty() || q.starts_with("http://") || q.starts_with("https://") || q.contains("youtu.be") {
            return;
        }
        let new_limit = self.download.expand_search_limit();
        self.download.is_searching = true;
        self.downloader.search_with_limit(q.clone(), new_limit);
        self.set_toast(format!("Expanding results for '{q}' to {new_limit}..."));
    }

    /// Cycles the result-count preset (5/10/15/25/50) and refetches.
    pub fn cycle_search_limit(&mut self) {
        const PRESETS: [usize; 5] = [5, 10, 15, 25, 50];
        let cur = self.download.search_limit;
        let next = PRESETS.iter().find(|&&p| p > cur).copied().unwrap_or(PRESETS[0]);
        self.download.search_limit = next;
        let q = self.download.query.trim().to_string();
        if q.is_empty() || q.starts_with("http://") || q.starts_with("https://") || q.contains("youtu.be") {
            self.set_toast(format!("Search result limit: {next}"));
            return;
        }
        self.download.is_searching = true;
        self.downloader.search_with_limit(q.clone(), next);
        self.set_toast(format!("Search result limit: {next} (refetching '{q}')"));
    }

    pub fn check_download_events(&mut self) {
        while let Ok(event) = self.downloader.receiver.try_recv() {
            match event {
                DownloadEvent::Searching => {
                    self.download.is_searching = true;
                    self.download.last_error = None;
                }
                DownloadEvent::SearchResults(items) => {
                    self.download.is_searching = false;
                    self.download.search_results = items;
                    self.download.selected_result = 0;
                    self.download.is_input_active = false;
                }
                DownloadEvent::InfoLoaded { url, title, artist } => {
                    self.download.is_searching = false;
                    self.open_metadata_form(url, title, artist);
                }
                DownloadEvent::LyricsProbe { has_manual, has_auto } => {
                    // Only a badge hint: manual subs ~ .lrc likely,
                    // auto-only ~ auto captions, neither ~ skip lyrics.
                    self.download.lyrics_probe = Some((has_manual, has_auto));
                }
                DownloadEvent::CoverLoading => {
                    self.download.is_cover_loading = true;
                }
                DownloadEvent::CoverResults(items) => {
                    self.download.is_cover_loading = false;
                    self.download.cover_candidates = items;
                    if self.download.form_custom_cover.is_none() && !self.download.cover_candidates.is_empty() {
                        self.download.selected_cover = Some(0);
                        self.download.cover_picker_selected = 1;
                    }
                    self.download.update_cover_preview();
                }
                DownloadEvent::Progress { percent, speed, eta, stage } => {
                    self.download.is_downloading = true;
                    self.download.set_progress(percent, speed, eta, stage);
                }
                DownloadEvent::Status(stage) => {
                    self.download.current_stage = stage;
                }
                DownloadEvent::Completed { title, artist, mp3_path, cover_source, .. } => {
                    self.download.is_downloading = false;
                    let src = cover_source.unwrap_or_default();
                    let suffix = if src.is_empty() || src == "none" {
                        String::new()
                    } else {
                        format!(" [{src}]")
                    };
                    self.download.last_completed = Some(format!("{artist} - {title}{suffix}"));
                    self.download.last_error = None;
                    self.browser.refresh();
                    let mp3_p = PathBuf::from(mp3_path);
                    // Sorted insert that preserves playing/selected cursors
                    // by path identity (no display/cursor jump mid-playback).
                    self.playlist.insert_sorted(mp3_p);
                }
                DownloadEvent::Error(err) => {
                    self.download.is_searching = false;
                    self.download.is_downloading = false;
                    self.download.last_error = Some(err);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collect_existing_artists_discovers_subfolder_tracks() {
        let temp_dir = std::env::temp_dir().join(format!("lyra_artist_test_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let sub_dir = temp_dir.join("Anime Album");
        std::fs::create_dir_all(&sub_dir).unwrap();

        // Create dummy audio files with artist - title filename pattern
        let song1 = sub_dir.join("Konomi Suzuki - This game.mp3");
        let song2 = sub_dir.join("Aya Hirano - God knows.mp3");
        std::fs::write(&song1, b"dummy mp3 1").unwrap();
        std::fs::write(&song2, b"dummy mp3 2").unwrap();

        if let Ok(app) = App::new(&temp_dir) {
            let artists = app.collect_existing_artists();
            assert!(artists.contains(&"Konomi Suzuki".to_string()), "Must find Konomi Suzuki from subdirectory");
            assert!(artists.contains(&"Aya Hirano".to_string()), "Must find Aya Hirano from subdirectory");
        }

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
