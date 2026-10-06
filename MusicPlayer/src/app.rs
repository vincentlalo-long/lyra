use std::{fs, path::{Path, PathBuf}};
use anyhow::Result;
use crate::{audio::AudioPlayer, browser::FileBrowser};

#[derive(PartialEq, Eq)]
pub enum ViewMode {
    Playlist,
    Browser,
}

pub struct App {
    pub view_mode: ViewMode,
    pub browser: FileBrowser,
    pub audio: AudioPlayer,
    pub songs: Vec<PathBuf>,
    pub selected: usize,
    pub playing_index: Option<usize>,
}

impl App {
    pub fn new(music_folder: &Path) -> Result<Self> {
        let mut songs = Vec::new();

        if music_folder.exists() {
            if let Ok(entries) = fs::read_dir(music_folder) {
                for entry in entries.flatten() {
                    let file_path = entry.path();
                    if file_path.is_file() {
                        if let Some(extension) = file_path.extension() {
                            if extension.eq_ignore_ascii_case("mp3") {
                                songs.push(file_path);
                            }
                        }
                    }
                }
            }
        }

        songs.sort();

        Ok(Self {
            view_mode: ViewMode::Playlist,
            browser: FileBrowser::new(music_folder),
            audio: AudioPlayer::new()?,
            songs,
            selected: 0,
            playing_index: None,
        })
    }

    pub fn toggle_view(&mut self) {
        match self.view_mode {
            ViewMode::Playlist => self.view_mode = ViewMode::Browser,
            ViewMode::Browser => self.view_mode = ViewMode::Playlist,
        }
    }

    pub fn next(&mut self) {
        match self.view_mode {
            ViewMode::Playlist => {
                if !self.songs.is_empty() {
                    self.selected = (self.selected + 1) % self.songs.len();
                }
            }
            ViewMode::Browser => {
                self.browser.next();
            }
        }
    }

    pub fn previous(&mut self) {
        match self.view_mode {
            ViewMode::Playlist => {
                if !self.songs.is_empty() {
                    if self.selected == 0 {
                        self.selected = self.songs.len() - 1;
                    } else {
                        self.selected -= 1;
                    }
                }
            }
            ViewMode::Browser => {
                self.browser.previous();
            }
        }
    }

    pub fn play_file(&mut self, song_path: &Path) {
        if self.audio.play(song_path).is_ok() {
            if let Some(index) = self.songs.iter().position(|p| p == song_path) {
                self.playing_index = Some(index);
            } else {
                self.songs.push(song_path.to_path_buf());
                self.playing_index = Some(self.songs.len() - 1);
            }
        }
    }

    pub fn on_enter(&mut self) {
        match self.view_mode {
            ViewMode::Playlist => {
                if !self.songs.is_empty() {
                    let song_path = self.songs[self.selected].clone();
                    self.play_file(&song_path);
                }
            }
            ViewMode::Browser => {
                if let Some(song_path) = self.browser.enter() {
                    self.play_file(&song_path);
                }
            }
        }
    }
}
