use std::{
    fs,
    path::{Path, PathBuf},
};
use ratatui::widgets::ListState;

pub struct Playlist {
    pub songs: Vec<PathBuf>,
    pub selected: usize,
    pub playing_index: Option<usize>,
    pub state: ListState,
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
            state: ListState::default(),
        }
    }

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

    pub fn next(&mut self, query: &str) {
        let indices = self.filtered_indices(query);
        if !indices.is_empty() {
            let current_pos = indices.iter().position(|&i| i == self.selected).unwrap_or(0);
            let next_pos = (current_pos + 1) % indices.len();
            self.selected = indices[next_pos];
        }
    }

    pub fn previous(&mut self, query: &str) {
        let indices = self.filtered_indices(query);
        if !indices.is_empty() {
            let current_pos = indices.iter().position(|&i| i == self.selected).unwrap_or(0);
            let prev_pos = if current_pos == 0 {
                indices.len() - 1
            } else {
                current_pos - 1
            };
            self.selected = indices[prev_pos];
        }
    }

    pub fn page_down(&mut self, query: &str, step: usize) {
        let indices = self.filtered_indices(query);
        if !indices.is_empty() {
            let current_pos = indices.iter().position(|&i| i == self.selected).unwrap_or(0);
            let next_pos = (current_pos + step).min(indices.len() - 1);
            self.selected = indices[next_pos];
        }
    }

    pub fn page_up(&mut self, query: &str, step: usize) {
        let indices = self.filtered_indices(query);
        if !indices.is_empty() {
            let current_pos = indices.iter().position(|&i| i == self.selected).unwrap_or(0);
            let prev_pos = current_pos.saturating_sub(step);
            self.selected = indices[prev_pos];
        }
    }

    pub fn first(&mut self, query: &str) {
        let indices = self.filtered_indices(query);
        if let Some(&first) = indices.first() {
            self.selected = first;
        }
    }

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

    pub fn set_songs(&mut self, songs: Vec<PathBuf>) {
        self.songs = songs;
        self.selected = 0;
        self.playing_index = None;
        self.state = ListState::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_playlist_navigation_and_paging() {
        let mut playlist = Playlist {
            songs: (0..50).map(|i| PathBuf::from(format!("song_{:02}.mp3", i))).collect(),
            selected: 0,
            playing_index: None,
            state: ListState::default(),
        };

        assert_eq!(playlist.selected, 0);

        // Next and previous
        playlist.next("");
        assert_eq!(playlist.selected, 1);
        playlist.previous("");
        assert_eq!(playlist.selected, 0);

        // Previous from 0 wraps to last
        playlist.previous("");
        assert_eq!(playlist.selected, 49);

        // Next from last wraps to 0
        playlist.next("");
        assert_eq!(playlist.selected, 0);

        // Page down (jump 10)
        playlist.page_down("", 10);
        assert_eq!(playlist.selected, 10);
        playlist.page_down("", 10);
        assert_eq!(playlist.selected, 20);

        // Page up
        playlist.page_up("", 10);
        assert_eq!(playlist.selected, 10);

        // First and last
        playlist.last("");
        assert_eq!(playlist.selected, 49);
        playlist.first("");
        assert_eq!(playlist.selected, 0);
    }
}
