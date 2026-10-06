use std::fs;
use std::path::{Path, PathBuf};

pub struct FileBrowser {
    pub current_dir: PathBuf,
    pub entries: Vec<PathBuf>,
    pub selected: usize,
    pub show_hidden: bool, // Toggle hidden files/folders (starting with .)
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
            show_hidden: false, // Default: hide dotfiles
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

        // Always add parent directory entry if available
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

                // Filter out dotfiles if show_hidden is false
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

    pub fn next(&mut self) {
        if !self.entries.is_empty() {
            self.selected = (self.selected + 1) % self.entries.len();
        }
    }

    pub fn previous(&mut self) {
        if !self.entries.is_empty() {
            if self.selected == 0 {
                self.selected = self.entries.len() - 1;
            } else {
                self.selected -= 1;
            }
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
            self.refresh();
            None
        } else {
            Some(target_path)
        }
    }
}
