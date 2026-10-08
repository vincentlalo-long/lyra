use std::fs;
use std::path::{Path, PathBuf};
use ratatui::widgets::ListState;

pub struct FileBrowser {
    pub current_dir: PathBuf,
    pub entries: Vec<PathBuf>,
    pub selected: usize,
    pub show_hidden: bool,
    pub state: ListState,
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
            let mut list_mp3 = Vec::new();

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
                        if extension.eq_ignore_ascii_case("mp3") {
                            list_mp3.push(path);
                        }
                    }
                }
            }

            list_folder.sort();
            list_mp3.sort();

            self.entries.extend(list_folder);
            self.entries.extend(list_mp3);
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

    pub fn enter(&mut self) -> Option<PathBuf> {
        if self.entries.is_empty() {
            return None;
        }

        let target_path = self.entries[self.selected].clone();

        if target_path.is_dir() {
            self.current_dir = target_path;
            self.selected = 0;
            self.state = ListState::default();
            self.refresh();
            None
        } else {
            Some(target_path)
        }
    }
}
