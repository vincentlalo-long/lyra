use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Instant;
use anyhow::Result;
use crate::{
    audio::AudioPlayer,
    browser::FileBrowser,
    cover::AlbumArt,
    downloader::{DownloadEvent, DownloadRequest, DownloadState, Downloader},
    lyrics::Lyrics,
    mpris::{MediaKey, MprisHandle},
    playlist::Playlist,
    queue::PlayQueue,
    scanner::Scanner,
};

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum RepeatMode {
    Playlist, // Loop entire playlist
    Track,    // Loop current single track
    Off,      // Stop when reaching end of playlist
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum ViewMode {
    Playlist,
    Queue,
    Browser,
    Download,
}

pub struct App {
    pub view_mode: ViewMode,
    pub browser: FileBrowser,
    pub playlist: Playlist,
    /// Ephemeral scratchpad: what to play next. Drains before the library.
    pub queue: PlayQueue,
    pub music_folder: PathBuf,
    pub audio: AudioPlayer,
    pub scanner: Scanner,
    pub lyrics: Option<Lyrics>,
    pub search_query: String,
    pub is_searching: bool,
    pub show_help: bool,
    pub repeat_mode: RepeatMode,
    pub cover: Option<AlbumArt>,
    pub current_playing_path: Option<PathBuf>,
    pub kitty_cover_rect: Option<ratatui::layout::Rect>,
    pub last_kitty_rendered: Option<(Option<PathBuf>, ratatui::layout::Rect)>,
    pub download: DownloadState,
    pub downloader: Downloader,
    /// MPRIS bridge feeding the desktop dynamic island / playerctl / media keys.
    /// Degrades to a silent no-op without D-Bus (see `mpris.rs`).
    pub mpris: MprisHandle,
    /// Island / media-key presses, drained every main-loop tick.
    pub media_rx: mpsc::Receiver<MediaKey>,
    /// Transient one-line feedback ("Queued #3: ..."), expires after a few seconds.
    pub toast: Option<(String, Instant)>,
}

/// How long a toast stays visible.
const TOAST_TTL_SECS: u64 = 3;

fn short_name(path: &Path) -> String {
    path.file_stem()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Unknown".to_string())
}

impl App {
    pub fn new(music_folder: &Path) -> Result<Self> {
        let (mpris, media_rx) = crate::mpris::spawn();
        Ok(Self {
            view_mode: ViewMode::Playlist,
            browser: FileBrowser::new(music_folder),
            playlist: Playlist::new(music_folder),
            queue: PlayQueue::load(),
            music_folder: music_folder.to_path_buf(),
            audio: AudioPlayer::new()?,
            scanner: Scanner::new(),
            lyrics: None,
            search_query: String::new(),
            is_searching: false,
            show_help: false,
            repeat_mode: RepeatMode::Playlist,
            cover: None,
            current_playing_path: None,
            kitty_cover_rect: None,
            last_kitty_rendered: None,
            download: DownloadState::new(music_folder),
            downloader: Downloader::new(),
            mpris,
            media_rx,
            toast: None,
        })
    }

    /// Shows a transient one-line feedback message in the header.
    pub fn set_toast(&mut self, msg: String) {
        self.toast = Some((msg, Instant::now()));
    }

    /// Current toast text, if still fresh.
    pub fn toast_text(&self) -> Option<&str> {
        self.toast.as_ref().and_then(|(msg, at)| {
            if at.elapsed().as_secs() < TOAST_TTL_SECS {
                Some(msg.as_str())
            } else {
                None
            }
        })
    }

    pub fn toggle_view(&mut self) {
        self.view_mode = match self.view_mode {
            ViewMode::Playlist => ViewMode::Queue,
            ViewMode::Queue => ViewMode::Browser,
            ViewMode::Browser => ViewMode::Download,
            ViewMode::Download => ViewMode::Playlist,
        };
    }

    pub fn toggle_repeat(&mut self) {
        self.repeat_mode = match self.repeat_mode {
            RepeatMode::Playlist => RepeatMode::Track,
            RepeatMode::Track => RepeatMode::Off,
            RepeatMode::Off => RepeatMode::Playlist,
        };
        // Mirror repeat mode as MPRIS loop status for the island.
        let (track, playlist) = match self.repeat_mode {
            RepeatMode::Track => (true, false),
            RepeatMode::Playlist => (false, true),
            RepeatMode::Off => (false, false),
        };
        self.mpris.set_loop(track, playlist);
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
            ViewMode::Playlist => self.playlist.next(&self.search_query),
            ViewMode::Queue => self.queue.move_selected_down(),
            ViewMode::Browser => self.browser.next(&self.search_query),
            ViewMode::Download => {
                if self.download.show_dir_picker {
                    self.download.dir_picker.next("");
                } else if self.download.show_metadata_form {
                    self.download.form_field_idx = (self.download.form_field_idx + 1) % 6;
                } else if !self.download.search_results.is_empty() {
                    self.download.selected_result = (self.download.selected_result + 1) % self.download.search_results.len();
                }
            }
        }
    }

    pub fn previous(&mut self) {
        match self.view_mode {
            ViewMode::Playlist => self.playlist.previous(&self.search_query),
            ViewMode::Queue => self.queue.move_selected_up(),
            ViewMode::Browser => self.browser.previous(&self.search_query),
            ViewMode::Download => {
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
        }
    }

    pub fn page_down(&mut self) {
        match self.view_mode {
            ViewMode::Playlist => self.playlist.page_down(&self.search_query, 10),
            ViewMode::Queue => {
                for _ in 0..10 {
                    self.queue.move_selected_down();
                }
            }
            ViewMode::Browser => self.browser.page_down(&self.search_query, 10),
            ViewMode::Download => {
                if self.download.show_dir_picker {
                    self.download.dir_picker.page_down("", 5);
                }
            }
        }
    }

    pub fn page_up(&mut self) {
        match self.view_mode {
            ViewMode::Playlist => self.playlist.page_up(&self.search_query, 10),
            ViewMode::Queue => {
                for _ in 0..10 {
                    self.queue.move_selected_up();
                }
            }
            ViewMode::Browser => self.browser.page_up(&self.search_query, 10),
            ViewMode::Download => {
                if self.download.show_dir_picker {
                    self.download.dir_picker.page_up("", 5);
                }
            }
        }
    }

    pub fn go_to_top(&mut self) {
        match self.view_mode {
            ViewMode::Playlist => self.playlist.first(&self.search_query),
            ViewMode::Queue => self.queue.selected = 0,
            ViewMode::Browser => self.browser.first(&self.search_query),
            ViewMode::Download => {
                if self.download.show_dir_picker {
                    self.download.dir_picker.first("");
                } else if !self.download.search_results.is_empty() {
                    self.download.selected_result = 0;
                }
            }
        }
    }

    pub fn go_to_bottom(&mut self) {
        match self.view_mode {
            ViewMode::Playlist => self.playlist.last(&self.search_query),
            ViewMode::Queue => {
                self.queue.selected = self.queue.len().saturating_sub(1);
            }
            ViewMode::Browser => self.browser.last(&self.search_query),
            ViewMode::Download => {
                if self.download.show_dir_picker {
                    self.download.dir_picker.last("");
                } else if !self.download.search_results.is_empty() {
                    self.download.selected_result = self.download.search_results.len() - 1;
                }
            }
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
            self.lyrics = Lyrics::load_for_song(song_path);
            self.cover = Some(AlbumArt::load_for_song(song_path));
            // Best-effort desktop notification (quickshell/mako/dunst/swaync).
            // Never affects playback; disable with LYRA_NO_NOTIFY=1.
            crate::notify::send_track_notification(song_path);
            // Publish track to the dynamic island via MPRIS (no-op without D-Bus).
            let track = crate::mpris::track_for_song(
                song_path,
                self.audio.duration,
                self.cover.as_ref(),
            );
            self.mpris.set_track(track);
            self.mpris.set_loop(
                self.repeat_mode == RepeatMode::Track,
                self.repeat_mode == RepeatMode::Playlist,
            );
        }
    }

    /// Applies one island / media-key / playerctl event (MPRIS input side).
    /// Play/pause/status/volume/position echo back to the island on the next
    /// `mpris_tick`, so handlers only need to drive the audio engine.
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
                if let Some(selected_song) = self.browser.enter() {
                    let folder_songs: Vec<PathBuf> = self
                        .browser
                        .entries
                        .iter()
                        .filter(|p| {
                            p.is_file()
                                && p.extension().is_some_and(|e| e.eq_ignore_ascii_case("mp3"))
                        })
                        .cloned()
                        .collect();

                    if !folder_songs.is_empty() {
                        self.playlist.set_songs(folder_songs);
                    }

                    self.play_track(&selected_song);
                }
            }
            ViewMode::Download => {
                self.handle_download_enter();
            }
        }
    }

    pub fn play_next_track(&mut self) {
        // The scratchpad drains first; the library is the fallback.
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
        if let Some(next_path) = self.playlist.next_track_path() {
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
        if let Some(prev_path) = self.playlist.prev_track_path() {
            self.play_track(&prev_path);
        }
    }

    /// Path under the cursor in file-list views (for enqueue / queue-delete).
    /// Queue view resolves to the queued item itself; Download view has none.
    fn selected_file(&self) -> Option<PathBuf> {
        match self.view_mode {
            ViewMode::Playlist => self.playlist.current_selected_song(),
            ViewMode::Queue => self.queue.items.get(self.queue.selected).cloned(),
            ViewMode::Browser => self.browser.entries.get(self.browser.selected).cloned().filter(|p| {
                p.is_file() && p.extension().is_some_and(|e| e.eq_ignore_ascii_case("mp3"))
            }),
            ViewMode::Download => None,
        }
    }

    /// `a`: append cursor song to the queue tail. Duplicates allowed
    /// (a dumb list is predictable; `d` removes them one by one).
    pub fn enqueue_selected_back(&mut self) {
        if let Some(path) = self.selected_file() {
            let name = short_name(&path);
            self.queue.enqueue_back(path);
            self.queue.persist();
            self.set_toast(format!("Queued #{}: {name}", self.queue.len()));
        }
    }

    /// `A`: play cursor song next (queue head).
    pub fn enqueue_selected_front(&mut self) {
        if let Some(path) = self.selected_file() {
            let name = short_name(&path);
            self.queue.enqueue_front(path);
            self.queue.persist();
            self.set_toast(format!("Up next: {name}"));
        }
    }

    /// `Enter` on a queue row: play it now. Predecessors move to history
    /// (Prev still reaches them), successors stay queued.
    pub fn play_queue_selected(&mut self) {
        if self.queue.is_empty() {
            return;
        }
        let idx = self.queue.selected;
        let current = self.current_playing_path.clone();
        let was_from_queue = self.queue.now_from_queue;
        if let Some(path) = self.queue.jump_to(idx, current, was_from_queue) {
            self.queue.persist();
            self.play_track(&path);
            self.queue.now_from_queue = true;
        }
    }

    /// `d`: delete from the queue, never from disk.
    /// - Queue view: remove the cursor row.
    /// - File views: if the cursor song is the currently playing queue track,
    ///   skip it (advance); otherwise unqueue its first occurrence (no-op
    ///   when absent).
    pub fn delete_queue_selected(&mut self) {
        if self.view_mode == ViewMode::Queue {
            if let Some(removed) = self.queue.items.get(self.queue.selected).cloned() {
                let name = short_name(&removed);
                self.queue.remove(self.queue.selected);
                self.queue.persist();
                self.set_toast(format!("Unqueued: {name}"));
            }
            return;
        }
        let Some(path) = self.selected_file() else {
            return;
        };
        let is_now_playing = self.current_playing_path.as_ref() == Some(&path);
        if is_now_playing && self.queue.now_from_queue {
            // Deleting what is playing = skip it.
            let name = short_name(&path);
            self.queue.now_from_queue = false;
            self.play_next_track();
            self.set_toast(format!("Skipped: {name}"));
        } else if self.queue.remove_path(&path) {
            self.queue.persist();
            self.set_toast(format!("Unqueued: {}", short_name(&path)));
        } else {
            self.set_toast("Not in queue".to_string());
        }
    }

    /// `c`: drop the whole scratchpad, back to plain library playback.
    pub fn clear_queue(&mut self) {
        let n = self.queue.len();
        self.queue.clear();
        self.queue.persist();
        self.set_toast(if n == 0 {
            "Queue already empty".to_string()
        } else {
            format!("Queue cleared ({n} removed)")
        });
    }

    /// `z`: shuffle upcoming queue items (now-playing untouched).
    pub fn shuffle_queue(&mut self) {
        let n = self.queue.len();
        if n < 2 {
            self.set_toast("Nothing to shuffle".to_string());
            return;
        }
        let mut seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9E3779B97F4A7C15);
        if seed == 0 {
            seed = 0x9E3779B97F4A7C15;
        }
        self.queue.shuffle(&mut seed);
        self.queue.persist();
        self.set_toast(format!("Queue shuffled ({n} songs)"));
    }

    /// `w`: save upcoming queue as today's portable day-list.
    /// Returns the saved file for potential UI feedback.
    pub fn save_daylist(&mut self) -> Option<PathBuf> {
        if self.queue.is_empty() {
            self.set_toast("Queue empty, nothing to save".to_string());
            return None;
        }
        let saved = self.queue.save_daylist(&self.music_folder.clone());
        self.browser.refresh();
        match &saved {
            Some(p) => self.set_toast(format!(
                "Day-list saved: {}",
                p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
            )),
            None => self.set_toast("Save failed".to_string()),
        }
        saved
    }

    /// `o`: replace the queue with the most recent day-list.
    /// Returns the number of tracks loaded.
    pub fn load_daylist(&mut self) -> usize {
        let folder = self.music_folder.clone();
        let n = self.queue.load_latest_daylist(&folder);
        self.set_toast(if n == 0 {
            "No day-list found".to_string()
        } else {
            format!("Loaded {n} songs from day-list")
        });
        n
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
                    if let Some(current) = self.playlist.playing_index {
                        if current + 1 < self.playlist.songs.len() {
                            self.play_next_track();
                        }
                    }
                }
            }
        }
    }

    pub fn start_scan(&mut self) {
        let target_dir = self.browser.current_dir.clone();
        let include_hidden = self.browser.show_hidden;
        self.scanner.start(target_dir, include_hidden);
    }

    pub fn check_scan(&mut self) {
        if let Some(found_songs) = self.scanner.tick() {
            if !found_songs.is_empty() {
                self.playlist.set_songs(found_songs);
                self.view_mode = ViewMode::Playlist;
            }
        }
    }

    pub fn start_metadata_download(&mut self) {
        if !self.download.show_metadata_form {
            return;
        }
        let cover_url = self
            .download
            .selected_cover
            .and_then(|i| self.download.cover_candidates.get(i))
            .map(|c| c.cover_url.clone())
            .filter(|u| !u.is_empty());
        let req = DownloadRequest {
            url: self.download.form_url.clone(),
            output_dir: self.download.form_dir.clone(),
            title: self.download.form_title.clone(),
            artist: self.download.form_artist.clone(),
            album: self.download.form_album.clone(),
            cover_mode: self.download.form_cover_mode.clone(),
            cover_url,
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

    /// Open the metadata form and kick off background studio-cover search
    /// (termusic songtag-style: user reviews metadata while candidates load).
    fn open_metadata_form(&mut self, url: String, title: String, artist: String) {
        self.download.form_url = url;
        self.download.form_title = title.clone();
        self.download.form_artist = artist.clone();
        self.download.form_album = format!("{title} - Single");
        self.download.show_metadata_form = true;
        self.download.form_field_idx = 0;
        self.download.cover_candidates.clear();
        self.download.selected_cover = None;
        self.download.is_cover_loading = true;
        self.downloader.fetch_covers(artist, title);
    }

    pub fn handle_download_enter(&mut self) {
        if self.download.show_dir_picker {
            if let Some(selected_dir) = self.download.dir_picker.enter() {
                self.download.form_dir = selected_dir;
                self.download.show_dir_picker = false;
            }
        } else if self.download.show_metadata_form {
            if self.download.form_field_idx == 3 {
                // Save to field: open dir picker
                self.download.dir_picker = crate::browser::FileBrowser::new(&self.download.form_dir);
                self.download.show_dir_picker = true;
            } else if self.download.form_field_idx == 4 {
                self.download.form_field_idx = 5;
            } else if self.download.form_field_idx == 5 {
                self.download.form_field_idx = 6;
            } else if self.download.form_field_idx < 6 {
                // Advance to next field on Enter
                self.download.form_field_idx += 1;
            } else {
                // Field 6: Start download!
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
                self.downloader.search(q);
            }
        } else if !self.download.search_results.is_empty() {
            let item = self.download.search_results[self.download.selected_result].clone();
            let url = item.url.clone();
            let title = item.title.clone();
            let artist = item.artist.clone();
            self.open_metadata_form(url, title, artist);
        }
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
                DownloadEvent::CoverLoading => {
                    self.download.is_cover_loading = true;
                }
                DownloadEvent::CoverResults(items) => {
                    self.download.is_cover_loading = false;
                    self.download.cover_candidates = items;
                    self.download.selected_cover = None;
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
                    if !self.playlist.songs.contains(&mp3_p) {
                        self.playlist.songs.push(mp3_p);
                        self.playlist.songs.sort();
                    }
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
