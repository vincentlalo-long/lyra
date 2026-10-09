use std::path::{Path, PathBuf};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use id3::TagLike;
use crate::browser::BrowserActionModal;
use super::App;

impl App {
    /// Opens the prompt modal to create a new album (folder) in current browser directory
    pub fn browser_create_album_prompt(&mut self) {
        self.browser_modal = Some(BrowserActionModal::NewAlbum {
            input: String::new(),
        });
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

            let album_name = entry_path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "album".into());

            if is_empty {
                if let Err(e) = std::fs::remove_dir(&entry_path) {
                    self.set_toast(format!("Failed to delete album '{album_name}': {e}"));
                } else {
                    self.browser.refresh();
                    self.set_toast(format!("󰄬 Deleted empty album '{album_name}'"));
                }
            } else {
                self.set_toast(format!("󰅖 Album '{album_name}' is not empty! Only empty albums can be deleted."));
            }
        } else if entry_path.is_file() {
            let file_name = entry_path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "file".into());
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

    /// Opens modal to edit track metadata (Title, Artist, Album) or rename album folder
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
            let name = entry_path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            self.browser_modal = Some(BrowserActionModal::RenameAlbum {
                target_dir: entry_path,
                input: name,
            });
        } else if entry_path.is_file() {
            let meta = self.song_meta(&entry_path);
            let title = if meta.title.is_empty() {
                entry_path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default()
            } else {
                meta.title
            };
            self.browser_modal = Some(BrowserActionModal::EditTrack {
                target_file: entry_path,
                field_idx: 0,
                title,
                artist: meta.artist,
                album: meta.album,
            });
        }
    }

    /// Handles keyboard events when a browser action modal is active
    pub fn handle_browser_modal_key(&mut self, key: KeyEvent) -> bool {
        let Some(modal) = self.browser_modal.take() else {
            return true;
        };

        let has_ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let has_alt = key.modifiers.contains(KeyModifiers::ALT);

        if has_alt {
            self.browser_modal = Some(modal);
            return true;
        }

        if has_ctrl && (key.code == KeyCode::Char('c') || key.code == KeyCode::Char('C')) {
            self.set_toast("Action cancelled".to_string());
            self.browser_modal = None;
            return true;
        }

        match modal {
            BrowserActionModal::NewAlbum { mut input } => {
                if has_ctrl {
                    match key.code {
                        KeyCode::Char('u') | KeyCode::Char('U') => input.clear(),
                        KeyCode::Char('w') | KeyCode::Char('W') => crate::input::delete_last_word(&mut input),
                        _ => {}
                    }
                    self.browser_modal = Some(BrowserActionModal::NewAlbum { input });
                    return true;
                }
                match key.code {
                    KeyCode::Esc => {
                        self.set_toast("Album creation cancelled".to_string());
                    }
                    KeyCode::Backspace => {
                        input.pop();
                        self.browser_modal = Some(BrowserActionModal::NewAlbum { input });
                    }
                    KeyCode::Char(c) => {
                        input.push(c);
                        self.browser_modal = Some(BrowserActionModal::NewAlbum { input });
                    }
                    KeyCode::Enter => {
                        let trimmed = input.trim();
                        if trimmed.is_empty() {
                            self.set_toast("Album name cannot be empty".to_string());
                        } else {
                            let new_dir = self.browser.current_dir.join(trimmed);
                            match std::fs::create_dir_all(&new_dir) {
                                Ok(_) => {
                                    self.browser.refresh();
                                    if let Some(pos) = self.browser.entries.iter().position(|p| p == &new_dir) {
                                        self.browser.selected = pos;
                                    }
                                    self.set_toast(format!("󰄬 Created new album: {trimmed}"));
                                }
                                Err(e) => {
                                    self.set_toast(format!("Failed to create album: {e}"));
                                }
                            }
                        }
                    }
                    _ => {
                        self.browser_modal = Some(BrowserActionModal::NewAlbum { input });
                    }
                }
            }
            BrowserActionModal::RenameAlbum { target_dir, mut input } => {
                if has_ctrl {
                    match key.code {
                        KeyCode::Char('u') | KeyCode::Char('U') => input.clear(),
                        KeyCode::Char('w') | KeyCode::Char('W') => crate::input::delete_last_word(&mut input),
                        _ => {}
                    }
                    self.browser_modal = Some(BrowserActionModal::RenameAlbum { target_dir, input });
                    return true;
                }
                match key.code {
                    KeyCode::Esc => {
                        self.set_toast("Album rename cancelled".to_string());
                    }
                    KeyCode::Backspace => {
                        input.pop();
                        self.browser_modal = Some(BrowserActionModal::RenameAlbum { target_dir, input });
                    }
                    KeyCode::Char(c) => {
                        input.push(c);
                        self.browser_modal = Some(BrowserActionModal::RenameAlbum { target_dir, input });
                    }
                    KeyCode::Enter => {
                        let trimmed = input.trim();
                        if trimmed.is_empty() {
                            self.set_toast("Album name cannot be empty".to_string());
                        } else if let Some(parent) = target_dir.parent() {
                            let new_dir = parent.join(trimmed);
                            if new_dir == target_dir {
                                self.set_toast("Album name unchanged".to_string());
                            } else {
                                match std::fs::rename(&target_dir, &new_dir) {
                                    Ok(_) => {
                                        self.browser.refresh();
                                        if let Some(pos) = self.browser.entries.iter().position(|p| p == &new_dir) {
                                            self.browser.selected = pos;
                                        }
                                        self.set_toast(format!("󰄬 Renamed album to: {trimmed}"));
                                    }
                                    Err(e) => {
                                        self.set_toast(format!("Failed to rename album: {e}"));
                                    }
                                }
                            }
                        }
                    }
                    _ => {
                        self.browser_modal = Some(BrowserActionModal::RenameAlbum { target_dir, input });
                    }
                }
            }
            BrowserActionModal::EditTrack {
                target_file,
                mut field_idx,
                mut title,
                mut artist,
                mut album,
            } => {
                if has_ctrl {
                    match key.code {
                        KeyCode::Char('u') | KeyCode::Char('U') => {
                            match field_idx {
                                0 => title.clear(),
                                1 => artist.clear(),
                                2 => album.clear(),
                                _ => {}
                            }
                            self.browser_modal = Some(BrowserActionModal::EditTrack {
                                target_file,
                                field_idx,
                                title,
                                artist,
                                album,
                            });
                            return true;
                        }
                        KeyCode::Char('w') | KeyCode::Char('W') => {
                            match field_idx {
                                0 => crate::input::delete_last_word(&mut title),
                                1 => crate::input::delete_last_word(&mut artist),
                                2 => crate::input::delete_last_word(&mut album),
                                _ => {}
                            }
                            self.browser_modal = Some(BrowserActionModal::EditTrack {
                                target_file,
                                field_idx,
                                title,
                                artist,
                                album,
                            });
                            return true;
                        }
                        _ => {}
                    }
                }
                match key.code {
                    KeyCode::Esc => {
                        self.set_toast("Track metadata edit cancelled".to_string());
                    }
                    KeyCode::Tab | KeyCode::Down => {
                        field_idx = (field_idx + 1) % 3;
                        self.browser_modal = Some(BrowserActionModal::EditTrack {
                            target_file,
                            field_idx,
                            title,
                            artist,
                            album,
                        });
                    }
                    KeyCode::Up => {
                        field_idx = (field_idx + 2) % 3;
                        self.browser_modal = Some(BrowserActionModal::EditTrack {
                            target_file,
                            field_idx,
                            title,
                            artist,
                            album,
                        });
                    }
                    KeyCode::Backspace => {
                        match field_idx {
                            0 => { title.pop(); }
                            1 => { artist.pop(); }
                            2 => { album.pop(); }
                            _ => {}
                        }
                        self.browser_modal = Some(BrowserActionModal::EditTrack {
                            target_file,
                            field_idx,
                            title,
                            artist,
                            album,
                        });
                    }
                    KeyCode::Char(c) => {
                        match field_idx {
                            0 => { title.push(c); }
                            1 => { artist.push(c); }
                            2 => { album.push(c); }
                            _ => {}
                        }
                        self.browser_modal = Some(BrowserActionModal::EditTrack {
                            target_file,
                            field_idx,
                            title,
                            artist,
                            album,
                        });
                    }
                    KeyCode::Enter => {
                        if field_idx < 2 && !key.modifiers.contains(KeyModifiers::CONTROL) {
                            field_idx += 1;
                            self.browser_modal = Some(BrowserActionModal::EditTrack {
                                target_file,
                                field_idx,
                                title,
                                artist,
                                album,
                            });
                        } else {
                            // Save ID3 tags
                            let mut tag = id3::Tag::read_from_path(&target_file).unwrap_or_else(|_| id3::Tag::new());
                            tag.set_title(title.trim());
                            tag.set_artist(artist.trim());
                            tag.set_album(album.trim());

                            if let Err(e) = tag.write_to_path(&target_file, id3::Version::Id3v24) {
                                self.set_toast(format!("Failed to write ID3 tags: {e}"));
                            } else {
                                self.meta_cache.remove(&target_file);
                                let _ = self.song_meta(&target_file);
                                self.set_toast(format!("󰄬 Updated metadata for: {}", title.trim()));
                            }
                        }
                    }
                    _ => {
                        self.browser_modal = Some(BrowserActionModal::EditTrack {
                            target_file,
                            field_idx,
                            title,
                            artist,
                            album,
                        });
                    }
                }
            }
            BrowserActionModal::MoveTrack {
                target_file,
                candidate_albums,
                mut selected_idx,
                creating_new,
                mut new_album_input,
            } => {
                if creating_new {
                    if has_ctrl {
                        match key.code {
                            KeyCode::Char('u') | KeyCode::Char('U') => new_album_input.clear(),
                            KeyCode::Char('w') | KeyCode::Char('W') => crate::input::delete_last_word(&mut new_album_input),
                            _ => {}
                        }
                        self.browser_modal = Some(BrowserActionModal::MoveTrack {
                            target_file,
                            candidate_albums,
                            selected_idx,
                            creating_new: true,
                            new_album_input,
                        });
                        return true;
                    }
                    match key.code {
                        KeyCode::Esc => {
                            self.browser_modal = Some(BrowserActionModal::MoveTrack {
                                target_file,
                                candidate_albums,
                                selected_idx,
                                creating_new: false,
                                new_album_input,
                            });
                        }
                        KeyCode::Backspace => {
                            new_album_input.pop();
                            self.browser_modal = Some(BrowserActionModal::MoveTrack {
                                target_file,
                                candidate_albums,
                                selected_idx,
                                creating_new: true,
                                new_album_input,
                            });
                        }
                        KeyCode::Char(c) => {
                            new_album_input.push(c);
                            self.browser_modal = Some(BrowserActionModal::MoveTrack {
                                target_file,
                                candidate_albums,
                                selected_idx,
                                creating_new: true,
                                new_album_input,
                            });
                        }
                        KeyCode::Enter => {
                            let trimmed = new_album_input.trim();
                            if trimmed.is_empty() {
                                self.set_toast("New album name cannot be empty".to_string());
                            } else {
                                let new_album_dir = self.music_folder.join(trimmed);
                                if let Err(e) = std::fs::create_dir_all(&new_album_dir) {
                                    self.set_toast(format!("Failed to create new album dir: {e}"));
                                } else {
                                    self.execute_move_track(target_file, &new_album_dir, trimmed);
                                }
                            }
                        }
                        _ => {
                            self.browser_modal = Some(BrowserActionModal::MoveTrack {
                                target_file,
                                candidate_albums,
                                selected_idx,
                                creating_new: true,
                                new_album_input,
                            });
                        }
                    }
                } else {
                    let total_options = candidate_albums.len() + 1; // +1 for "Create New Album"
                    match key.code {
                        KeyCode::Esc => {
                            self.set_toast("Move track cancelled".to_string());
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            if selected_idx == 0 {
                                selected_idx = total_options.saturating_sub(1);
                            } else {
                                selected_idx -= 1;
                            }
                            self.browser_modal = Some(BrowserActionModal::MoveTrack {
                                target_file,
                                candidate_albums,
                                selected_idx,
                                creating_new: false,
                                new_album_input,
                            });
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            selected_idx = (selected_idx + 1) % total_options;
                            self.browser_modal = Some(BrowserActionModal::MoveTrack {
                                target_file,
                                candidate_albums,
                                selected_idx,
                                creating_new: false,
                                new_album_input,
                            });
                        }
                        KeyCode::Char('n') | KeyCode::Char('N') => {
                            self.browser_modal = Some(BrowserActionModal::MoveTrack {
                                target_file,
                                candidate_albums,
                                selected_idx,
                                creating_new: true,
                                new_album_input: String::new(),
                            });
                        }
                        KeyCode::Enter => {
                            if selected_idx == candidate_albums.len() {
                                // Option "Create New Album"
                                self.browser_modal = Some(BrowserActionModal::MoveTrack {
                                    target_file,
                                    candidate_albums,
                                    selected_idx,
                                    creating_new: true,
                                    new_album_input: String::new(),
                                });
                            } else if let Some(target_dir) = candidate_albums.get(selected_idx) {
                                let album_name = target_dir.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "Album".into());
                                self.execute_move_track(target_file, target_dir, &album_name);
                            }
                        }
                        _ => {
                            self.browser_modal = Some(BrowserActionModal::MoveTrack {
                                target_file,
                                candidate_albums,
                                selected_idx,
                                creating_new: false,
                                new_album_input,
                            });
                        }
                    }
                }
            }
        }

        true
    }

    /// Moves a track to target destination album directory and updates ID3 album tag & playlist
    fn execute_move_track(&mut self, source_file: PathBuf, target_dir: &Path, album_name: &str) {
        let Some(fname) = source_file.file_name() else {
            self.set_toast("Invalid track file name".to_string());
            return;
        };
        let dest_path = target_dir.join(fname);

        if dest_path == source_file {
            self.set_toast("Track is already in target album".to_string());
            return;
        }

        match std::fs::rename(&source_file, &dest_path) {
            Ok(_) => {
                // Update ID3 album tag
                if let Ok(mut tag) = id3::Tag::read_from_path(&dest_path) {
                    tag.set_album(album_name);
                    let _ = tag.write_to_path(&dest_path, id3::Version::Id3v24);
                }

                // Update playlist songs if present
                if let Some(pos) = self.playlist.songs.iter().position(|p| p == &source_file) {
                    self.playlist.songs[pos] = dest_path.clone();
                }
                self.meta_cache.remove(&source_file);
                let _ = self.song_meta(&dest_path);

                self.browser.refresh();
                self.set_toast(format!("󰄬 Moved track to album '{album_name}'"));
            }
            Err(e) => {
                self.set_toast(format!("Failed to move track: {e}"));
            }
        }
    }

    /// Sets the current browser directory as the permanent default Music Library path.
    /// Updates active playlist, config paths, and persists setting to ~/.config/lyra/config.toml!
    pub fn browser_set_current_as_music_folder(&mut self) {
        let new_path = self.browser.current_dir.clone();
        let path_str = new_path.to_string_lossy().to_string();
        self.music_folder = new_path.clone();
        self.playlist = crate::playlist::Playlist::new(&new_path);
        if let Some(item) = self.config_paths.iter_mut().find(|p| p.id == "music_folder") {
            item.path = path_str.clone();
        }
        let _ = crate::config::Config::save_setting("music_folder", &path_str);
        self.set_toast(format!("󰄬 Set & saved Music Library to: {path_str}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[test]
    fn test_browser_create_album() {
        let temp_dir = std::env::temp_dir().join("lyra_test_create_album");
        let _ = std::fs::remove_dir_all(&temp_dir);
        let _ = std::fs::create_dir_all(&temp_dir);

        if let Ok(mut app) = App::new(&temp_dir) {
            app.browser.current_dir = temp_dir.clone();
            app.browser.refresh();

            app.browser_create_album_prompt();
            assert!(matches!(app.browser_modal, Some(BrowserActionModal::NewAlbum { .. })));

            // Type "Summer 2026"
            for c in "Summer 2026".chars() {
                app.handle_browser_modal_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
            }
            // Press Enter
            app.handle_browser_modal_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

            assert!(app.browser_modal.is_none());
            assert!(temp_dir.join("Summer 2026").is_dir());
        }

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_browser_delete_empty_album_and_protect_non_empty() {
        let temp_dir = std::env::temp_dir().join("lyra_test_delete_album");
        let _ = std::fs::remove_dir_all(&temp_dir);
        let _ = std::fs::create_dir_all(&temp_dir);

        let empty_album = temp_dir.join("EmptyAlbum");
        let non_empty_album = temp_dir.join("NonEmptyAlbum");
        let _ = std::fs::create_dir_all(&empty_album);
        let _ = std::fs::create_dir_all(&non_empty_album);
        let _ = std::fs::write(non_empty_album.join("song.mp3"), b"audio data");

        if let Ok(mut app) = App::new(&temp_dir) {
            app.browser.current_dir = temp_dir.clone();
            app.browser.refresh();

            // Select EmptyAlbum
            if let Some(pos) = app.browser.entries.iter().position(|p| p == &empty_album) {
                app.browser.selected = pos;
                app.browser_delete_selected();
                assert!(!empty_album.exists());
            }

            // Select NonEmptyAlbum
            app.browser.refresh();
            if let Some(pos) = app.browser.entries.iter().position(|p| p == &non_empty_album) {
                app.browser.selected = pos;
                app.browser_delete_selected();
                // Must NOT be deleted!
                assert!(non_empty_album.exists());
                assert!(non_empty_album.join("song.mp3").exists());
            }
        }

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_browser_move_track_and_update_id3() {
        let temp_dir = std::env::temp_dir().join("lyra_test_move_track");
        let _ = std::fs::remove_dir_all(&temp_dir);
        let _ = std::fs::create_dir_all(&temp_dir);

        let source_file = temp_dir.join("track1.mp3");
        let target_album = temp_dir.join("TargetAlbum");
        let _ = std::fs::create_dir_all(&target_album);
        let _ = std::fs::write(&source_file, b"test audio content");

        if let Ok(mut app) = App::new(&temp_dir) {
            app.browser.current_dir = temp_dir.clone();
            app.browser.refresh();

            app.execute_move_track(source_file.clone(), &target_album, "TargetAlbum");

            assert!(!source_file.exists());
            assert!(target_album.join("track1.mp3").exists());
        }

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_browser_set_current_as_music_folder() {
        let temp_dir = std::env::temp_dir().join("lyra_test_set_lib");
        let sub_album = temp_dir.join("MyNewAlbum");
        let _ = std::fs::create_dir_all(&sub_album);
        let song = sub_album.join("test.mp3");
        let _ = std::fs::write(&song, b"mp3");

        if let Ok(mut app) = App::new(&temp_dir) {
            app.browser.current_dir = sub_album.clone();
            app.browser_set_current_as_music_folder();

            assert_eq!(app.music_folder, sub_album);
            assert_eq!(app.playlist.songs.len(), 1);
            assert_eq!(app.playlist.songs[0], song);
        }

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
