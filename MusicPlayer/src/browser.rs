use std::fs;
use std::path::{Path, PathBuf};
use ratatui::widgets::ListState;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BrowserActionModal {
    NewAlbum {
        input: String,
    },
    RenameAlbum {
        target_dir: PathBuf,
        input: String,
    },
    EditTrack {
        target_file: PathBuf,
        field_idx: usize, // 0: Title, 1: Artist, 2: Album
        title: String,
        artist: String,
        album: String,
    },
    MoveTrack {
        target_file: PathBuf,
        candidate_albums: Vec<PathBuf>,
        selected_idx: usize,
        creating_new: bool,
        new_album_input: String,
    },
}

pub struct FileBrowser {
    pub current_dir: PathBuf,
    pub entries: Vec<PathBuf>,
    pub selected: usize,
    pub show_hidden: bool,
    pub state: ListState,
    pub allowed_extensions: Option<Vec<String>>,
}

impl FileBrowser {
    pub fn new(start_dir: &Path) -> Self {
        let current_dir = match fs::canonicalize(start_dir) {
            Ok(absolute_path) => absolute_path,
            Err(_) => PathBuf::from("/"),
        };

        let mut browser = Self {
            current_dir,
            entries: Vec::new(),
            selected: 0,
            show_hidden: false,
            state: ListState::default(),
            allowed_extensions: Some(vec!["mp3".into(), "m3u".into(), "m3u8".into()]),
        };

        browser.refresh();
        browser
    }

    #[allow(dead_code)]
    pub fn for_images(start_dir: &Path) -> Self {
        let current_dir = match fs::canonicalize(start_dir) {
            Ok(absolute_path) => absolute_path,
            Err(_) => PathBuf::from("/"),
        };

        let mut browser = Self {
            current_dir,
            entries: Vec::new(),
            selected: 0,
            show_hidden: false,
            state: ListState::default(),
            allowed_extensions: Some(vec![
                "jpg".into(),
                "jpeg".into(),
                "png".into(),
                "webp".into(),
            ]),
        };

        browser.refresh();
        browser
    }

    pub fn toggle_hidden(&mut self) {
        self.show_hidden = !self.show_hidden;
        self.refresh();
    }

    pub fn refresh(&mut self) {
        self.entries.clear();

        if let Some(parent_dir) = self.current_dir.parent() {
            self.entries.push(parent_dir.to_path_buf());
        }

        if let Ok(read_dir) = fs::read_dir(&self.current_dir) {
            let mut list_folder = Vec::new();
            let mut list_files = Vec::new();

            for entry in read_dir.flatten() {
                let path = entry.path();
                let file_name = match path.file_name() {
                    Some(name) => name.to_string_lossy(),
                    None => continue,
                };

                if !self.show_hidden && file_name.starts_with('.') {
                    continue;
                }

                if path.is_dir() {
                    list_folder.push(path);
                } else if path.is_file() {
                    if let Some(extension) = path.extension() {
                        let ext_lower = extension.to_string_lossy().to_lowercase();
                        if let Some(allowed) = &self.allowed_extensions {
                            if allowed.iter().any(|a| a == &ext_lower) {
                                list_files.push(path);
                            }
                        } else {
                            list_files.push(path);
                        }
                    }
                }
            }

            list_folder.sort();
            list_files.sort();

            self.entries.extend(list_folder);
            self.entries.extend(list_files);
        }

        if self.selected >= self.entries.len() && !self.entries.is_empty() {
            self.selected = self.entries.len() - 1;
        }
    }

    pub fn filtered_indices(&self, query: &str) -> Vec<usize> {
        if query.is_empty() {
            (0..self.entries.len()).collect()
        } else {
            let lower_query = query.to_lowercase();
            self.entries
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

    pub fn go_parent(&mut self) -> bool {
        if let Some(parent) = self.current_dir.parent().map(|p| p.to_path_buf()) {
            let prev_dir = self.current_dir.clone();
            self.current_dir = parent;
            self.refresh();
            if let Some(pos) = self.entries.iter().position(|p| p == &prev_dir) {
                self.selected = pos;
            } else {
                self.selected = 0;
            }
            self.state = ListState::default();
            true
        } else {
            false
        }
    }

    pub fn enter(&mut self) -> Option<PathBuf> {
        if self.entries.is_empty() {
            return None;
        }

        let target_path = self.entries[self.selected].clone();

        if target_path.is_dir() {
            let prev_dir = self.current_dir.clone();
            let is_going_up = self.current_dir.parent() == Some(&target_path);
            self.current_dir = target_path;
            self.refresh();
            if is_going_up {
                if let Some(pos) = self.entries.iter().position(|p| p == &prev_dir) {
                    self.selected = pos;
                } else {
                    self.selected = 0;
                }
            } else {
                self.selected = 0;
            }
            self.state = ListState::default();
            None
        } else {
            Some(target_path)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_browser_for_images_filters_extensions() {
        let temp_dir = std::env::temp_dir().join("lyra_browser_test");
        let _ = fs::create_dir_all(&temp_dir);
        let _ = fs::write(temp_dir.join("song.mp3"), b"fake mp3");
        let _ = fs::write(temp_dir.join("photo.jpg"), b"fake jpg");
        let _ = fs::write(temp_dir.join("art.png"), b"fake png");
        let _ = fs::write(temp_dir.join("readme.txt"), b"fake txt");

        let browser = FileBrowser::for_images(&temp_dir);
        let file_names: Vec<String> = browser
            .entries
            .iter()
            .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
            .collect();

        assert!(file_names.contains(&"photo.jpg".to_string()));
        assert!(file_names.contains(&"art.png".to_string()));
        assert!(!file_names.contains(&"song.mp3".to_string()));
        assert!(!file_names.contains(&"readme.txt".to_string()));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_file_browser_go_parent() {
        let base_dir = std::env::temp_dir().join("lyra_parent_test");
        let sub_dir = base_dir.join("subfolder");
        let _ = fs::create_dir_all(&sub_dir);
        let _ = fs::write(sub_dir.join("track.mp3"), b"mp3");

        let mut browser = FileBrowser::new(&sub_dir);
        assert_eq!(browser.current_dir, fs::canonicalize(&sub_dir).unwrap());

        let went_up = browser.go_parent();
        assert!(went_up);
        assert_eq!(browser.current_dir, fs::canonicalize(&base_dir).unwrap());

        let _ = fs::remove_dir_all(&base_dir);
    }
}
