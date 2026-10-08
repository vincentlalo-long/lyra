use std::path::{Path, PathBuf};
use anyhow::Result;
use crate::{
    audio::AudioPlayer,
    browser::FileBrowser,
    cover::AlbumArt,
    downloader::{DownloadEvent, DownloadRequest, DownloadState, Downloader},
    lyrics::Lyrics,
    playlist::Playlist,
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
    Browser,
    Download,
}

pub struct App {
    pub view_mode: ViewMode,
    pub browser: FileBrowser,
    pub playlist: Playlist,
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
}

impl App {
    pub fn new(music_folder: &Path) -> Result<Self> {
        Ok(Self {
            view_mode: ViewMode::Playlist,
            browser: FileBrowser::new(music_folder),
            playlist: Playlist::new(music_folder),
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
        })
    }

    pub fn toggle_view(&mut self) {
        self.view_mode = match self.view_mode {
            ViewMode::Playlist => ViewMode::Browser,
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
        if self.audio.play(song_path).is_ok() {
            self.current_playing_path = Some(song_path.to_path_buf());
            self.playlist.select_and_mark_playing(song_path);
            self.lyrics = Lyrics::load_for_song(song_path);
            self.cover = Some(AlbumArt::load_for_song(song_path));
        }
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
        if let Some(next_path) = self.playlist.next_track_path() {
            self.play_track(&next_path);
        }
    }

    pub fn play_prev_track(&mut self) {
        if let Some(prev_path) = self.playlist.prev_track_path() {
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
        let req = DownloadRequest {
            url: self.download.form_url.clone(),
            output_dir: self.download.form_dir.clone(),
            title: self.download.form_title.clone(),
            artist: self.download.form_artist.clone(),
            album: self.download.form_album.clone(),
            cover_mode: self.download.form_cover_mode.clone(),
            no_lyrics: self.download.form_lyrics_mode == 2,
            no_auto_lyrics: self.download.form_lyrics_mode == 1,
        };
        self.download.active_title = req.title.clone();
        self.download.active_artist = req.artist.clone();
        self.download.is_downloading = true;
        self.download.progress_pct = 0.0;
        self.download.speed.clear();
        self.download.eta.clear();
        self.download.current_stage = "Connecting to YouTube...".into();
        self.download.last_error = None;
        self.download.last_completed = None;
        self.download.show_metadata_form = false;
        self.downloader.start_download(req);
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
            let item = &self.download.search_results[self.download.selected_result];
            self.download.form_url = item.url.clone();
            self.download.form_title = item.title.clone();
            self.download.form_artist = item.artist.clone();
            self.download.form_album = format!("{} - Single", item.title);
            self.download.show_metadata_form = true;
            self.download.form_field_idx = 0;
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
                    self.download.form_url = url;
                    self.download.form_title = title.clone();
                    self.download.form_artist = artist;
                    self.download.form_album = format!("{title} - Single");
                    self.download.show_metadata_form = true;
                    self.download.form_field_idx = 0;
                }
                DownloadEvent::Progress { percent, speed, eta, stage } => {
                    self.download.is_downloading = true;
                    self.download.progress_pct = percent;
                    self.download.speed = speed;
                    self.download.eta = eta;
                    self.download.current_stage = stage;
                }
                DownloadEvent::Status(stage) => {
                    self.download.current_stage = stage;
                }
                DownloadEvent::Completed { title, artist, mp3_path, .. } => {
                    self.download.is_downloading = false;
                    self.download.last_completed = Some(format!("{artist} - {title}"));
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
