//! Modal entry points and the operations they trigger: create/delete album,
//! move/edit/cover prompts, and in-memory path remapping after a rename.

use std::path::{Path, PathBuf};

use crate::browser::BrowserActionModal;

use super::caret::caret_end;
use crate::app::App;

impl App {
    /// Opens the prompt modal to create a new album (folder) in current browser directory
    pub fn browser_create_album_prompt(&mut self) {
        self.browser_modal = Some(BrowserActionModal::NewAlbum {
            input: String::new(),
        });
        self.browser_caret = 0;
    }

    /// Deletes the currently selected item in browser.
    /// If it's an album/folder: ONLY deletes if the folder is empty (safe data protection).
    /// If it's a file: deletes the file and removes it from active playlist if present.
    pub fn browser_delete_selected(&mut self) {
        let filtered = self.browser.filtered_indices(&self.search_query);
        let Some(&entry_idx) = filtered.get(self.browser.selected) else {
            return;
        };
        let Some(entry_path) = self.browser.entries.get(entry_idx).cloned() else {
            return;
        };

        if self.browser.current_dir.parent() == Some(entry_path.as_path()) {
            self.set_toast("Cannot delete parent directory ('..')".to_string());
            return;
        }

        if entry_path.is_dir() {
            // Check if folder is completely empty
            let is_empty = match std::fs::read_dir(&entry_path) {
                Ok(mut read) => read.next().is_none(),
                Err(_) => false,
            };

            let album_name = entry_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "album".into());

            if is_empty {
                if let Err(e) = std::fs::remove_dir(&entry_path) {
                    self.set_toast(format!("Failed to delete album '{album_name}': {e}"));
                } else {
                    self.browser.refresh();
                    self.set_toast(format!("󰄬 Deleted empty album '{album_name}'"));
                }
            } else {
                self.set_toast(format!(
                    "󰅖 Album '{album_name}' is not empty! Only empty albums can be deleted."
                ));
            }
        } else if entry_path.is_file() {
            let file_name = entry_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "file".into());
            if let Err(e) = std::fs::remove_file(&entry_path) {
                self.set_toast(format!("Failed to delete '{file_name}': {e}"));
            } else {
                // Remove from playlist if present
                if let Some(pos) = self.playlist.songs.iter().position(|p| p == &entry_path) {
                    self.playlist.songs.remove(pos);
                }
                self.meta_cache.remove(&entry_path);
                self.browser.refresh();
                self.set_toast(format!("󰄬 Deleted track '{file_name}'"));
            }
        }
    }

    /// Collect candidate album directories (subdirectories with audio files or in music_folder)
    pub fn collect_candidate_albums(&self, current_file: &Path) -> Vec<PathBuf> {
        let mut dirs = std::collections::BTreeSet::new();
        let cur_parent = current_file.parent();

        // 1. Scan music_folder subdirectories
        if let Ok(entries) = std::fs::read_dir(&self.music_folder) {
            for entry in entries.flatten() {
                if let Ok(ft) = entry.file_type() {
                    if ft.is_dir() {
                        let p = entry.path();
                        if Some(p.as_path()) != cur_parent {
                            dirs.insert(p);
                        }
                    }
                }
            }
        }

        // 2. Scan current browser directory subdirectories
        if let Ok(entries) = std::fs::read_dir(&self.browser.current_dir) {
            for entry in entries.flatten() {
                if let Ok(ft) = entry.file_type() {
                    if ft.is_dir() {
                        let p = entry.path();
                        if Some(p.as_path()) != cur_parent {
                            dirs.insert(p);
                        }
                    }
                }
            }
        }

        dirs.into_iter().collect()
    }

    /// Opens modal to move selected track to another album
    pub fn browser_move_track_prompt(&mut self) {
        let filtered = self.browser.filtered_indices(&self.search_query);
        let Some(&entry_idx) = filtered.get(self.browser.selected) else {
            return;
        };
        let Some(entry_path) = self.browser.entries.get(entry_idx).cloned() else {
            return;
        };

        if entry_path.is_dir() {
            self.set_toast("Select a music track to move it to an album".to_string());
            return;
        }

        let candidates = self.collect_candidate_albums(&entry_path);
        self.browser_modal = Some(BrowserActionModal::MoveTrack {
            target_file: entry_path,
            candidate_albums: candidates,
            selected_idx: 0,
            creating_new: false,
            new_album_input: String::new(),
        });
    }

    /// Opens modal to edit track metadata (Title, Artist, Album, File name)
    /// or rename album folder
    pub fn browser_edit_prompt(&mut self) {
        let filtered = self.browser.filtered_indices(&self.search_query);
        let Some(&entry_idx) = filtered.get(self.browser.selected) else {
            return;
        };
        let Some(entry_path) = self.browser.entries.get(entry_idx).cloned() else {
            return;
        };

        if self.browser.current_dir.parent() == Some(entry_path.as_path()) {
            self.set_toast("Cannot edit parent directory ('..')".to_string());
            return;
        }

        if entry_path.is_dir() {
            let name = entry_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            self.browser_caret = caret_end(&name);
            self.browser_modal = Some(BrowserActionModal::RenameAlbum {
                target_dir: entry_path,
                input: name,
            });
        } else if entry_path.is_file() {
            let meta = self.song_meta(&entry_path);
            let title = if meta.title.is_empty() {
                entry_path
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default()
            } else {
                meta.title
            };
            let file_name = entry_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            self.browser_caret = caret_end(&title);
            self.browser_modal = Some(BrowserActionModal::EditTrack {
                target_file: entry_path,
                field_idx: 0,
                title,
                artist: meta.artist,
                album: meta.album,
                file_name,
            });
        }
    }

    /// Remaps every in-memory reference of one track file after a
    /// rename/move: playlist, queue, now-playing, metadata cache, genres.
    pub(super) fn remap_track_path(&mut self, old: &Path, new: &Path) {
        for song in self.playlist.songs.iter_mut() {
            if song == old {
                *song = new.to_path_buf();
            }
        }
        for item in self.queue.items.iter_mut() {
            if item == old {
                *item = new.to_path_buf();
            }
        }
        for item in self.queue.history.iter_mut() {
            if item == old {
                *item = new.to_path_buf();
            }
        }
        if self.current_playing_path.as_deref() == Some(old) {
            self.current_playing_path = Some(new.to_path_buf());
        }
        self.meta_cache.remove(old);
        self.meta_cache.remove(new);
        #[cfg(feature = "genre")]
        self.genre_db.remap_paths(old, new);
    }

    /// Remaps every in-memory reference under a renamed album directory.
    /// Playing/selected cursors are preserved by path identity.
    pub(super) fn remap_album_dir(&mut self, old_dir: &Path, new_dir: &Path) {
        let playing_path = self.current_playing_path.clone();
        let selected_path = self.playlist.songs.get(self.playlist.selected).cloned();
        let map_prefix = |p: &Path| p.strip_prefix(old_dir).ok().map(|rest| new_dir.join(rest));
        for song in self.playlist.songs.iter_mut() {
            if let Some(mapped) = map_prefix(song) {
                *song = mapped;
            }
        }
        for item in self.queue.items.iter_mut() {
            if let Some(mapped) = map_prefix(item) {
                *item = mapped;
            }
        }
        for item in self.queue.history.iter_mut() {
            if let Some(mapped) = map_prefix(item) {
                *item = mapped;
            }
        }
        if let Some(cur) = playing_path {
            if let Some(mapped) = map_prefix(&cur) {
                self.current_playing_path = Some(mapped);
                self.cover = None; // reloaded lazily on next play_track
            }
        }
        // Restore cursors by identity (a plain refresh would lose them).
        if let Some(sel) = selected_path {
            if let Some(mapped) = map_prefix(&sel) {
                if let Some(pos) = self.playlist.songs.iter().position(|p| p == &mapped) {
                    self.playlist.selected = pos;
                }
            }
        }
        if let Some(cur) = self.current_playing_path.clone() {
            if let Some(pos) = self.playlist.songs.iter().position(|p| p == &cur) {
                self.playlist.playing_index = Some(pos);
            } else {
                self.playlist.playing_index = None;
            }
        }
        self.meta_cache
            .retain(|k, _| k.strip_prefix(old_dir).is_err());
        #[cfg(feature = "genre")]
        self.genre_db.remap_paths(old_dir, new_dir);
        // Follow the rename when it touches the browsed / library root.
        if self.browser.current_dir == old_dir {
            self.browser.current_dir = new_dir.to_path_buf();
        }
        if self.music_folder == old_dir {
            self.music_folder = new_dir.to_path_buf();
            let path_str = new_dir.to_string_lossy().to_string();
            if let Some(item) = self
                .config_paths
                .iter_mut()
                .find(|p| p.id == "music_folder")
            {
                item.path = path_str.clone();
            }
            let _ = crate::config::Config::save_setting("music_folder", &path_str);
        }
    }

    /// Opens the cover-art picker for the selected track (`B` in Browser).
    pub fn browser_set_cover_prompt(&mut self) {
        let filtered = self.browser.filtered_indices(&self.search_query);
        let Some(&entry_idx) = filtered.get(self.browser.selected) else {
            return;
        };
        let Some(entry_path) = self.browser.entries.get(entry_idx).cloned() else {
            return;
        };
        if !entry_path.is_file() {
            self.set_toast("Select a music track to change its cover art".to_string());
            return;
        }
        let start = entry_path
            .parent()
            .unwrap_or(&self.browser.current_dir)
            .to_path_buf();
        self.browser_modal = Some(BrowserActionModal::SetCover {
            target_file: entry_path,
            picker: crate::browser::FileBrowser::for_images(&start),
        });
    }

    /// Sets the current browser directory as the permanent default Music Library path.
    /// Updates active playlist, config paths, and persists setting to ~/.config/lyra/config.toml!
    pub fn browser_set_current_as_music_folder(&mut self) {
        let new_path = self.browser.current_dir.clone();
        let path_str = new_path.to_string_lossy().to_string();
        self.music_folder = new_path.clone();
        self.playlist = crate::playlist::Playlist::new(&new_path);
        if let Some(item) = self
            .config_paths
            .iter_mut()
            .find(|p| p.id == "music_folder")
        {
            item.path = path_str.clone();
        }
        let _ = crate::config::Config::save_setting("music_folder", &path_str);
        self.set_toast(format!("󰄬 Set & saved Music Library to: {path_str}"));
    }
}
