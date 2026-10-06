use std::path::{Path, PathBuf};
use anyhow::Result;
use crate::{
    audio::AudioPlayer,
    browser::FileBrowser,
    playlist::Playlist,
    scanner::Scanner,
};

#[derive(PartialEq, Eq)]
pub enum ViewMode {
    Playlist,
    Browser,
}

pub struct App {
    pub view_mode: ViewMode,
    pub browser: FileBrowser,
    pub playlist: Playlist,
    pub audio: AudioPlayer,
    pub scanner: Scanner,
}

impl App {
    pub fn new(music_folder: &Path) -> Result<Self> {
        Ok(Self {
            view_mode: ViewMode::Playlist,
            browser: FileBrowser::new(music_folder),
            playlist: Playlist::new(music_folder),
            audio: AudioPlayer::new()?,
            scanner: Scanner::new(),
        })
    }

    pub fn toggle_view(&mut self) {
        self.view_mode = match self.view_mode {
            ViewMode::Playlist => ViewMode::Browser,
            ViewMode::Browser => ViewMode::Playlist,
        };
    }

    pub fn next(&mut self) {
        match self.view_mode {
            ViewMode::Playlist => self.playlist.next(),
            ViewMode::Browser => self.browser.next(),
        }
    }

    pub fn previous(&mut self) {
        match self.view_mode {
            ViewMode::Playlist => self.playlist.previous(),
            ViewMode::Browser => self.browser.previous(),
        }
    }

    pub fn play_track(&mut self, song_path: &Path) {
        if self.audio.play(song_path).is_ok() {
            self.playlist.select_and_mark_playing(song_path);
        }
    }

    pub fn on_enter(&mut self) {
        match self.view_mode {
            ViewMode::Playlist => {
                if let Some(song_path) = self.playlist.current_selected_song() {
                    self.play_track(&song_path);
                }
            }
            ViewMode::Browser => {
                if let Some(selected_song) = self.browser.enter() {
                    // Populate playlist with all mp3 files found in this browser folder
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
        }
    }

    pub fn play_next_track(&mut self) {
        if let Some(next_path) = self.playlist.next_track_path() {
            let _ = self.audio.play(&next_path);
        }
    }

    pub fn play_prev_track(&mut self) {
        if let Some(prev_path) = self.playlist.prev_track_path() {
            let _ = self.audio.play(&prev_path);
        }
    }

    pub fn check_auto_advance(&mut self) {
        if self.playlist.playing_index.is_some()
            && !self.audio.is_paused
            && self.audio.is_finished()
        {
            if self.playlist.songs.len() > 1 {
                self.play_next_track();
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
}
