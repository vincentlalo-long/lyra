use std::path::Path;
use crate::{cover::AlbumArt, lyrics::Lyrics};
#[cfg(feature = "mpris")]
use crate::mpris::MediaKey;

use super::{App, RepeatMode, ViewMode, short_name};

impl App {
    pub fn toggle_repeat(&mut self) {
        self.repeat_mode = match self.repeat_mode {
            RepeatMode::Playlist => RepeatMode::Track,
            RepeatMode::Track => RepeatMode::Off,
            RepeatMode::Off => RepeatMode::Playlist,
        };
        // Mirror repeat mode as MPRIS loop status for the island.
        #[cfg(feature = "mpris")]
        if self.plugin_enabled("mpris") {
            self.sync_mpris_loop();
        }
    }

    /// Syncs MPRIS loop status. Queue-loop has no MPRIS equivalent and maps
    /// to Playlist (looping a list); it wins over the `r` repeat mode.
    #[cfg(feature = "mpris")]
    pub fn sync_mpris_loop(&self) {
        if self.queue.loop_enabled {
            self.mpris.set_loop(false, true);
        } else {
            let (track, playlist) = match self.repeat_mode {
                RepeatMode::Track => (true, false),
                RepeatMode::Playlist => (false, true),
                RepeatMode::Off => (false, false),
            };
            self.mpris.set_loop(track, playlist);
        }
    }

    /// `L`: loop the whole queue round (`RepeatMode` untouched).
    /// Enabling snapshots current upcoming items as one round; edits join
    /// future rounds; disabling lets the remainder drain once.
    pub fn toggle_queue_loop(&mut self) {
        let on = !self.queue.loop_enabled;
        self.queue.set_loop(on);
        self.queue.persist();
        #[cfg(feature = "mpris")]
        if self.plugin_enabled("mpris") {
            self.sync_mpris_loop();
        }
        self.set_toast(if on {
            "Queue loop: ON".to_string()
        } else {
            "Queue loop: OFF".to_string()
        });
    }

    pub fn replay_current_track(&mut self) {
        if let Some(current_idx) = self.playlist.playing_index {
            if let Some(song_path) = self.playlist.songs.get(current_idx).cloned() {
                self.play_track(&song_path);
            }
        }
    }

    pub fn next(&mut self) {
        match self.view_mode {
            ViewMode::Playlist => {
                #[cfg(feature = "genre")]
                self.playlist.next(&self.search_query, &self.genre_filters, &mut self.genre_db);
                #[cfg(not(feature = "genre"))]
                self.playlist.next(&self.search_query);
            }
            ViewMode::Queue => self.queue.move_selected_down(),
            ViewMode::Browser => self.browser.next(&self.search_query),
            #[cfg(feature = "download")]
            ViewMode::Extensions => {
                if self.download.show_dir_picker {
                    self.download.dir_picker.next("");
                } else if self.download.show_metadata_form {
                    self.download.form_field_idx = (self.download.form_field_idx + 1) % 6;
                } else if !self.download.search_results.is_empty() {
                    self.download.selected_result = (self.download.selected_result + 1) % self.download.search_results.len();
                }
            }
            #[cfg(not(feature = "download"))]
            ViewMode::Extensions => {}
            ViewMode::Plugins => {
                let max = self.total_plugin_items().saturating_sub(1);
                self.plugin_selected = (self.plugin_selected + 1).min(max);
            }
        }
    }

    pub fn previous(&mut self) {
        match self.view_mode {
            ViewMode::Playlist => {
                #[cfg(feature = "genre")]
                self.playlist.previous(&self.search_query, &self.genre_filters, &mut self.genre_db);
                #[cfg(not(feature = "genre"))]
                self.playlist.previous(&self.search_query);
            }
            ViewMode::Queue => self.queue.move_selected_up(),
            ViewMode::Browser => self.browser.previous(&self.search_query),
            #[cfg(feature = "download")]
            ViewMode::Extensions => {
                if self.download.show_dir_picker {
                    self.download.dir_picker.previous("");
                } else if self.download.show_metadata_form {
                    self.download.form_field_idx = if self.download.form_field_idx == 0 { 5 } else { self.download.form_field_idx - 1 };
                } else if !self.download.search_results.is_empty() {
                    self.download.selected_result = if self.download.selected_result == 0 {
                        self.download.search_results.len() - 1
                    } else {
                        self.download.selected_result - 1
                    };
                }
            }
            #[cfg(not(feature = "download"))]
            ViewMode::Extensions => {}
            ViewMode::Plugins => {
                self.plugin_selected = self.plugin_selected.saturating_sub(1);
            }
        }
    }

    pub fn page_down(&mut self) {
        match self.view_mode {
            ViewMode::Playlist => {
                #[cfg(feature = "genre")]
                self.playlist.page_down(&self.search_query, 10, &self.genre_filters, &mut self.genre_db);
                #[cfg(not(feature = "genre"))]
                self.playlist.page_down(&self.search_query, 10);
            }
            ViewMode::Queue => {
                for _ in 0..10 {
                    self.queue.move_selected_down();
                }
            }
            ViewMode::Browser => self.browser.page_down(&self.search_query, 10),
            #[cfg(feature = "download")]
            ViewMode::Extensions => {
                if self.download.show_dir_picker {
                    self.download.dir_picker.page_down("", 5);
                }
            }
            #[cfg(not(feature = "download"))]
            ViewMode::Extensions => {}
            ViewMode::Plugins => {
                let max = self.total_plugin_items().saturating_sub(1);
                self.plugin_selected = (self.plugin_selected + 4).min(max);
            }
        }
    }

    pub fn page_up(&mut self) {
        match self.view_mode {
            ViewMode::Playlist => {
                #[cfg(feature = "genre")]
                self.playlist.page_up(&self.search_query, 10, &self.genre_filters, &mut self.genre_db);
                #[cfg(not(feature = "genre"))]
                self.playlist.page_up(&self.search_query, 10);
            }
            ViewMode::Queue => {
                for _ in 0..10 {
                    self.queue.move_selected_up();
                }
            }
            ViewMode::Browser => self.browser.page_up(&self.search_query, 10),
            #[cfg(feature = "download")]
            ViewMode::Extensions => {
                if self.download.show_dir_picker {
                    self.download.dir_picker.page_up("", 5);
                }
            }
            #[cfg(not(feature = "download"))]
            ViewMode::Extensions => {}
            ViewMode::Plugins => {
                self.plugin_selected = self.plugin_selected.saturating_sub(4);
            }
        }
    }

    pub fn go_to_top(&mut self) {
        match self.view_mode {
            ViewMode::Playlist => {
                #[cfg(feature = "genre")]
                self.playlist.first(&self.search_query, &self.genre_filters, &mut self.genre_db);
                #[cfg(not(feature = "genre"))]
                self.playlist.first(&self.search_query);
            }
            ViewMode::Queue => self.queue.selected = 0,
            ViewMode::Browser => self.browser.first(&self.search_query),
            #[cfg(feature = "download")]
            ViewMode::Extensions => {
                if self.download.show_dir_picker {
                    self.download.dir_picker.first("");
                } else if !self.download.search_results.is_empty() {
                    self.download.selected_result = 0;
                }
            }
            #[cfg(not(feature = "download"))]
            ViewMode::Extensions => {}
            ViewMode::Plugins => self.plugin_selected = 0,
        }
    }

    pub fn go_to_bottom(&mut self) {
        match self.view_mode {
            ViewMode::Playlist => {
                #[cfg(feature = "genre")]
                self.playlist.last(&self.search_query, &self.genre_filters, &mut self.genre_db);
                #[cfg(not(feature = "genre"))]
                self.playlist.last(&self.search_query);
            }
            ViewMode::Queue => {
                self.queue.selected = self.queue.len().saturating_sub(1);
            }
            ViewMode::Browser => self.browser.last(&self.search_query),
            #[cfg(feature = "download")]
            ViewMode::Extensions => {
                if self.download.show_dir_picker {
                    self.download.dir_picker.last("");
                } else if !self.download.search_results.is_empty() {
                    self.download.selected_result = self.download.search_results.len() - 1;
                }
            }
            #[cfg(not(feature = "download"))]
            ViewMode::Extensions => {}
            ViewMode::Plugins => self.plugin_selected = self.total_plugin_items().saturating_sub(1),
        }
    }

    pub fn play_track(&mut self, song_path: &Path) {
        // A direct play (Enter on library/browser) is NOT a queue play.
        // Queue paths below re-assert the flag right after this call.
        // Without this reset, provenance leaks: pressing Prev after an
        // Enter would drag a library song into the queue items.
        self.queue.now_from_queue = false;
        if self.audio.play(song_path).is_ok() {
            self.current_playing_path = Some(song_path.to_path_buf());
            self.playlist.select_and_mark_playing(song_path);
            self.lyrics = if self.plugin_enabled("lyrics") {
                Lyrics::load_for_song(song_path)
            } else {
                None
            };
            self.cover = if self.plugin_enabled("artwork") {
                Some(AlbumArt::load_for_song(song_path))
            } else {
                None
            };
            // Best-effort desktop notification (quickshell/mako/dunst/swaync).
            // Never affects playback; disable with LYRA_NO_NOTIFY=1.
            #[cfg(feature = "notify")]
            if self.notifications_enabled {
                crate::notify::send_track_notification(song_path);
            }
            // Publish track to the dynamic island via MPRIS (no-op without D-Bus).
            #[cfg(feature = "mpris")]
            if self.plugin_enabled("mpris") {
                let track = crate::mpris::track_for_song(
                    song_path,
                    self.audio.duration,
                    self.cover.as_ref(),
                );
                self.mpris.set_track(track);
            }
            #[cfg(feature = "mpris")]
            if self.plugin_enabled("mpris") {
                self.sync_mpris_loop();
            }
        }
    }

    /// Applies one island / media-key / playerctl event (MPRIS input side).
    /// Play/pause/status/volume/position echo back to the island on the next
    /// `mpris_tick`, so handlers only need to drive the audio engine.
    #[cfg(feature = "mpris")]
    pub fn handle_media_key(&mut self, key: MediaKey) {
        match key {
            MediaKey::PlayPause => self.audio.toggle_pause(),
            MediaKey::Play => {
                if self.audio.is_paused {
                    self.audio.toggle_pause();
                }
            }
            MediaKey::Pause | MediaKey::Stop => {
                if !self.audio.is_paused {
                    self.audio.toggle_pause();
                }
            }
            MediaKey::Next => self.play_next_track(),
            MediaKey::Previous => self.play_prev_track(),
            MediaKey::SeekByMicros(delta) => {
                let base = self.audio.position();
                if delta >= 0 {
                    self.audio.seek_to(
                        base + crate::mpris::micros_to_duration(delta),
                    );
                } else {
                    self.audio.seek_to(
                        base.saturating_sub(crate::mpris::micros_to_duration(-delta)),
                    );
                }
            }
            MediaKey::SetPositionMicros(pos) => {
                self.audio.seek_to(crate::mpris::micros_to_duration(pos));
            }
            MediaKey::SetVolume(vol) => self.audio.set_volume_absolute(vol),
        }
    }

    /// Pushes playing state + volume + position to the island (MPRIS).
    /// Only sends on change (position throttled to ~4 Hz). Call every tick.
    #[cfg(feature = "mpris")]
    pub fn mpris_tick(&self) {
        let playing = !self.audio.is_paused && self.current_playing_path.is_some();
        self.mpris
            .tick(playing, self.audio.position(), self.audio.volume);
    }

    pub fn on_enter(&mut self) {
        if self.is_searching {
            self.is_searching = false;
            self.search_query.clear();
        }

        match self.view_mode {
            ViewMode::Playlist => {
                if let Some(song_path) = self.playlist.current_selected_song() {
                    self.play_track(&song_path);
                }
            }
            ViewMode::Queue => self.play_queue_selected(),
            ViewMode::Browser => {
                // Queue-replace flow (`e`): Enter picks the replacement.
                if self.pending_replace.is_some() {
                    self.confirm_replace();
                } else if let Some(selected_entry) = self.browser.enter() {
                    let ext = selected_entry
                        .extension()
                        .map(|e| e.to_string_lossy().to_lowercase())
                        .unwrap_or_default();
                    if ext == "m3u" || ext == "m3u8" {
                        let songs = crate::playlist::load_m3u(&selected_entry);
                        let count = songs.len();
                        if !songs.is_empty() {
                            self.playlist.append_songs(songs);
                            let name = selected_entry
                                .file_name()
                                .map(|n| n.to_string_lossy().into_owned())
                                .unwrap_or_default();
                            self.set_toast(format!("Added playlist '{name}' ({count} songs) -> Switch to [1] Playlist or [2] Queue to play"));
                        } else {
                            self.set_toast("Playlist is empty or tracks missing".to_string());
                        }
                    } else if ext == "mp3" {
                        let name = short_name(&selected_entry);
                        self.playlist.append_songs(vec![selected_entry]);
                        self.set_toast(format!("Added to Playlist: {name} (Switch to [1] Playlist or [2] Queue to play)"));
                    }
                }
            }
            #[cfg(feature = "download")]
            ViewMode::Extensions => {
                self.handle_download_enter();
            }
            #[cfg(not(feature = "download"))]
            ViewMode::Extensions => {}
            ViewMode::Plugins => {
                self.toggle_selected_plugin();
            }
        }
    }

    /// In Browser: navigate to parent directory ('..' / Backspace / h / Left).
    pub fn browser_go_parent(&mut self) {
        if self.browser.go_parent() {
            let dir_name = self.browser.current_dir.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "/".into());
            self.set_toast(format!("󰉋 Up to: {dir_name}/"));
        }
    }

    /// In Browser: imports selected folder, playlist file, or track into Playlist without wiping.
    pub fn import_browser_folder_into_playlist(&mut self) {
        if let Some(entry) = self.browser.entries.get(self.browser.selected).cloned() {
            if entry.extension().is_some_and(|e| e.eq_ignore_ascii_case("m3u") || e.eq_ignore_ascii_case("m3u8")) {
                let songs = crate::playlist::load_m3u(&entry);
                let count = songs.len();
                if count > 0 {
                    let name = short_name(&entry);
                    self.playlist.append_songs(songs);
                    self.set_toast(format!("Imported playlist '{name}' ({count} tracks)"));
                } else {
                    self.set_toast("Playlist file is empty or tracks missing".to_string());
                }
                return;
            } else if entry.is_file() && entry.extension().is_some_and(|e| e.eq_ignore_ascii_case("mp3")) {
                let name = short_name(&entry);
                self.playlist.append_songs(vec![entry]);
                self.set_toast(format!("Imported track '{name}' into Playlist"));
                return;
            } else if entry.is_dir() && Some(entry.as_path()) != self.browser.current_dir.parent() {
                let songs = crate::playlist::find_mp3s_in_dir(&entry, false);
                let folder_name = short_name(&entry);
                if songs.is_empty() {
                    self.set_toast(format!("No MP3 files found in {folder_name}/"));
                } else {
                    let count = songs.len();
                    self.playlist.append_songs(songs);
                    self.set_toast(format!("Imported album '{folder_name}' ({count} tracks) into Playlist"));
                }
                return;
            }
        }

        // Fallback: import all MP3s in the current browser directory
        let songs = crate::playlist::find_mp3s_in_dir(&self.browser.current_dir, false);
        let folder_name = short_name(&self.browser.current_dir);
        if songs.is_empty() {
            self.set_toast(format!("No MP3 files in {folder_name}/ to import"));
        } else {
            let count = songs.len();
            self.playlist.append_songs(songs);
            self.set_toast(format!("Imported '{folder_name}' ({count} tracks) into Playlist"));
        }
    }

    /// In Browser: plays all songs in the selected directory (or playlist file) immediately.
    pub fn play_browser_folder(&mut self) {
        if self.view_mode != ViewMode::Browser {
            return;
        }

        let (songs, folder_name) = if let Some(entry) = self.browser.entries.get(self.browser.selected).cloned() {
            if entry.extension().is_some_and(|e| e.eq_ignore_ascii_case("m3u") || e.eq_ignore_ascii_case("m3u8")) {
                let s = crate::playlist::load_m3u(&entry);
                let name = short_name(&entry);
                (s, name)
            } else if entry.is_dir() && Some(entry.as_path()) != self.browser.current_dir.parent() {
                let s = crate::playlist::find_mp3s_in_dir(&entry, false);
                let name = short_name(&entry);
                (s, name)
            } else {
                let s = crate::playlist::find_mp3s_in_dir(&self.browser.current_dir, false);
                let name = short_name(&self.browser.current_dir);
                (s, name)
            }
        } else {
            let s = crate::playlist::find_mp3s_in_dir(&self.browser.current_dir, false);
            let name = short_name(&self.browser.current_dir);
            (s, name)
        };

        if songs.is_empty() {
            self.set_toast(format!("No MP3 files found in {folder_name}"));
            return;
        }

        let count = songs.len();
        self.playlist.set_songs(songs);
        self.set_toast(format!("Loaded {count} songs from {folder_name} into Playlist (Switch to [1] Playlist to play)"));
    }

    /// True when a `/` search query or genre filter currently narrows
    /// the playlist (a disabled genre plugin never counts as filtering).
    fn playlist_filter_active(&self) -> bool {
        if !self.search_query.is_empty() {
            return true;
        }
        #[cfg(feature = "genre")]
        {
            self.plugin_enabled("genre") && !self.genre_filters.is_empty()
        }
        #[cfg(not(feature = "genre"))]
        {
            false
        }
    }

    /// Visible playlist indices regardless of compile features.
    fn playlist_visible_indices_any(&mut self) -> Vec<usize> {
        #[cfg(feature = "genre")]
        {
            self.visible_playlist_indices()
        }
        #[cfg(not(feature = "genre"))]
        {
            let q = self.search_query.clone();
            self.playlist.filtered_indices(&q)
        }
    }

    pub fn play_next_track(&mut self) {        // The scratchpad drains first; the library is the fallback.
        if !self.queue.is_empty() {
            let current = self.current_playing_path.clone();
            let was_from_queue = self.queue.now_from_queue;
            if let Some(next_path) = self.queue.pop_next(current, was_from_queue) {
                self.queue.persist();
                self.play_track(&next_path);
                // play_track() clears provenance; this one really is queued.
                self.queue.now_from_queue = true;
                return;
            }
        }
        self.queue.now_from_queue = false;
        // An active filter narrows audio advance too — not just the cursor.
        if self.playlist_filter_active() {
            let indices = self.playlist_visible_indices_any();
            if let Some(next_path) = self.playlist.next_track_path_filtered(&indices) {
                self.play_track(&next_path);
            }
        } else if let Some(next_path) = self.playlist.next_track_path() {
            self.play_track(&next_path);
        }
    }

    pub fn play_prev_track(&mut self) {
        // Walk back through queue playback when the current track came from it.
        if self.queue.now_from_queue && !self.queue.history.is_empty() {
            let current = self.current_playing_path.clone();
            if let Some(prev_path) = self.queue.pop_prev(current) {
                self.queue.persist();
                self.play_track(&prev_path);
                self.queue.now_from_queue = true;
                return;
            }
        }
        self.queue.now_from_queue = false;
        if self.playlist_filter_active() {
            let indices = self.playlist_visible_indices_any();
            if let Some(prev_path) = self.playlist.prev_track_path_filtered(&indices) {
                self.play_track(&prev_path);
            }
        } else if let Some(prev_path) = self.playlist.prev_track_path() {
            self.play_track(&prev_path);
        }
    }

    pub fn check_auto_advance(&mut self) {
        if self.playlist.playing_index.is_some()
            && !self.audio.is_paused
            && self.audio.is_finished()
        {
            if self.playlist.songs.is_empty() {
                return;
            }

            // If playlist only has 1 track: always repeat it
            if self.playlist.songs.len() == 1 {
                let song_path = self.playlist.songs[0].clone();
                self.play_track(&song_path);
                return;
            }

            match self.repeat_mode {
                RepeatMode::Track => {
                    self.replay_current_track();
                }
                RepeatMode::Playlist => {
                    self.play_next_track();
                }
                RepeatMode::Off => {
                    // Stop at the end of the *visible* list so playback
                    // never leaks into filtered-out tracks.
                    if self.playlist_filter_active() {
                        let indices = self.playlist_visible_indices_any();
                        if !self.playlist.playing_is_last_visible(&indices) {
                            self.play_next_track();
                        }
                    } else if let Some(current) = self.playlist.playing_index {
                        if current + 1 < self.playlist.songs.len() {
                            self.play_next_track();
                        }
                    }
                }
            }
        }
    }

}
