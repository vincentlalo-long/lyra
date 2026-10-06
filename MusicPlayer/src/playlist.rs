use std::{fs, path::{Path, PathBuf}};

pub struct Playlist {
    pub songs: Vec<PathBuf>,
    pub selected: usize,
    pub playing_index: Option<usize>,
}

impl Playlist {
    pub fn new(music_folder: &Path) -> Self {
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

        Self {
            songs,
            selected: 0,
            playing_index: None,
        }
    }

    pub fn next(&mut self) {
        if !self.songs.is_empty() {
            self.selected = (self.selected + 1) % self.songs.len();
        }
    }

    pub fn previous(&mut self) {
        if !self.songs.is_empty() {
            if self.selected == 0 {
                self.selected = self.songs.len() - 1;
            } else {
                self.selected -= 1;
            }
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

    pub fn set_songs(&mut self, songs: Vec<PathBuf>) {
        self.songs = songs;
        self.selected = 0;
        self.playing_index = None;
    }
}
