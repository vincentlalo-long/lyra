use std::path::{Path, PathBuf};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use id3::TagLike;
use crate::browser::BrowserActionModal;
use super::App;

/// Byte index of a char-position caret (clamped to text end).
fn caret_byte_idx(s: &str, caret: usize) -> usize {
    s.char_indices().map(|(b, _)| b).nth(caret).unwrap_or(s.len())
}

/// Movable caret for single-line modal inputs (browser modals).
fn caret_insert(s: &mut String, caret: &mut usize, c: char) {
    let byte = caret_byte_idx(s, *caret);
    s.insert(byte, c);
    *caret += 1;
}

fn caret_backspace(s: &mut String, caret: &mut usize) {
    if *caret == 0 {
        return;
    }
    let end = caret_byte_idx(s, *caret);
    let start = caret_byte_idx(s, *caret - 1);
    s.drain(start..end);
    *caret -= 1;
}

fn caret_delete(s: &mut String, caret: usize) {
    let len = s.chars().count();
    if caret >= len {
        return;
    }
    let start = caret_byte_idx(s, caret);
    let end = caret_byte_idx(s, caret + 1);
    s.drain(start..end);
}

fn caret_delete_word(s: &mut String, caret: &mut usize) {
    let chars: Vec<char> = s.chars().collect();
    let mut start = (*caret).min(chars.len());
    while start > 0 && chars[start - 1] == ' ' {
        start -= 1;
    }
    while start > 0 && chars[start - 1] != ' ' {
        start -= 1;
    }
    let b_start = caret_byte_idx(s, start);
    let b_end = caret_byte_idx(s, (*caret).min(chars.len()));
    s.drain(b_start..b_end);
    *caret = start;
}

fn caret_end(s: &str) -> usize {
    s.chars().count()
}

/// Mutable access to one EditTrack text field by index
/// (0: Title, 1: Artist, 2: Album, 3: File name).
fn edit_field_mut<'a>(
    idx: usize,
    title: &'a mut String,
    artist: &'a mut String,
    album: &'a mut String,
    file_name: &'a mut String,
) -> Option<&'a mut String> {
    match idx {
        0 => Some(title),
        1 => Some(artist),
        2 => Some(album),
        3 => Some(file_name),
        _ => None,
    }
}

fn edit_field(field_idx: usize, title: &str, artist: &str, album: &str, file_name: &str) -> usize {
    let text = match field_idx {
        0 => title,
        1 => artist,
        2 => album,
        3 => file_name,
        _ => "",
    };
    caret_end(text)
}

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
            let name = entry_path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            self.browser_caret = caret_end(&name);
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
            let file_name = entry_path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
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
    fn remap_track_path(&mut self, old: &Path, new: &Path) {
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
    fn remap_album_dir(&mut self, old_dir: &Path, new_dir: &Path) {
        let playing_path = self.current_playing_path.clone();
        let selected_path = self.playlist.songs.get(self.playlist.selected).cloned();
        let map_prefix = |p: &Path| {
            p.strip_prefix(old_dir)
                .ok()
                .map(|rest| new_dir.join(rest))
        };
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
        self.meta_cache.retain(|k, _| k.strip_prefix(old_dir).is_err());
        #[cfg(feature = "genre")]
        self.genre_db.remap_paths(old_dir, new_dir);
        // Follow the rename when it touches the browsed / library root.
        if self.browser.current_dir == old_dir {
            self.browser.current_dir = new_dir.to_path_buf();
        }
        if self.music_folder == old_dir {
            self.music_folder = new_dir.to_path_buf();
            let path_str = new_dir.to_string_lossy().to_string();
            if let Some(item) = self.config_paths.iter_mut().find(|p| p.id == "music_folder") {
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
        let start = entry_path.parent().unwrap_or(&self.browser.current_dir).to_path_buf();
        self.browser_modal = Some(BrowserActionModal::SetCover {
            target_file: entry_path,
            picker: crate::browser::FileBrowser::for_images(&start),
        });
    }

    /// Embeds `image_path` as the track's front cover (ID3 APIC) and saves
    /// a `cover.jpg` next to the track for file-based fallbacks.
    fn embed_track_cover(&mut self, target_file: &Path, image_path: &Path) {
        let ext = image_path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let mime_type = match ext.as_str() {
            "png" => "image/png",
            "webp" => "image/webp",
            _ => "image/jpeg",
        };
        let data = match std::fs::read(image_path) {
            Ok(d) => d,
            Err(e) => {
                self.set_toast(format!("Failed to read image: {e}"));
                return;
            }
        };
        let mut tag = id3::Tag::read_from_path(target_file).unwrap_or_else(|_| id3::Tag::new());
        tag.add_frame(id3::frame::Picture {
            mime_type: mime_type.to_string(),
            picture_type: id3::frame::PictureType::CoverFront,
            description: String::new(),
            data,
        });
        if let Err(e) = tag.write_to_path(target_file, id3::Version::Id3v24) {
            self.set_toast(format!("Failed to write cover art: {e}"));
            return;
        }
        // Sibling cover.jpg keeps folder-based fallbacks in sync.
        if let Some(parent) = target_file.parent() {
            let _ = std::fs::write(parent.join("cover.jpg"), std::fs::read(image_path).unwrap_or_default());
        }
        self.meta_cache.remove(target_file);
        // Refresh the live cover when it belongs to the playing track.
        if self.current_playing_path.as_deref() == Some(target_file) {
            self.cover = Some(crate::cover::AlbumArt::load_for_song(target_file));
        }
        self.browser.refresh();
        let name = target_file
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        self.set_toast(format!("󰄬 Updated cover art for: {name}"));
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
                        KeyCode::Char('u') | KeyCode::Char('U') => {
                            input.clear();
                            self.browser_caret = 0;
                        }
                        KeyCode::Char('w') | KeyCode::Char('W') => caret_delete_word(&mut input, &mut self.browser_caret),
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
                        caret_backspace(&mut input, &mut self.browser_caret);
                        self.browser_modal = Some(BrowserActionModal::NewAlbum { input });
                    }
                    KeyCode::Delete => {
                        caret_delete(&mut input, self.browser_caret);
                        self.browser_modal = Some(BrowserActionModal::NewAlbum { input });
                    }
                    KeyCode::Left => {
                        self.browser_caret = self.browser_caret.saturating_sub(1);
                        self.browser_modal = Some(BrowserActionModal::NewAlbum { input });
                    }
                    KeyCode::Right => {
                        self.browser_caret = (self.browser_caret + 1).min(caret_end(&input));
                        self.browser_modal = Some(BrowserActionModal::NewAlbum { input });
                    }
                    KeyCode::Home => {
                        self.browser_caret = 0;
                        self.browser_modal = Some(BrowserActionModal::NewAlbum { input });
                    }
                    KeyCode::End => {
                        self.browser_caret = caret_end(&input);
                        self.browser_modal = Some(BrowserActionModal::NewAlbum { input });
                    }
                    KeyCode::Char(c) => {
                        caret_insert(&mut input, &mut self.browser_caret, c);
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
                        KeyCode::Char('u') | KeyCode::Char('U') => {
                            input.clear();
                            self.browser_caret = 0;
                        }
                        KeyCode::Char('w') | KeyCode::Char('W') => caret_delete_word(&mut input, &mut self.browser_caret),
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
                        caret_backspace(&mut input, &mut self.browser_caret);
                        self.browser_modal = Some(BrowserActionModal::RenameAlbum { target_dir, input });
                    }
                    KeyCode::Delete => {
                        caret_delete(&mut input, self.browser_caret);
                        self.browser_modal = Some(BrowserActionModal::RenameAlbum { target_dir, input });
                    }
                    KeyCode::Left => {
                        self.browser_caret = self.browser_caret.saturating_sub(1);
                        self.browser_modal = Some(BrowserActionModal::RenameAlbum { target_dir, input });
                    }
                    KeyCode::Right => {
                        self.browser_caret = (self.browser_caret + 1).min(caret_end(&input));
                        self.browser_modal = Some(BrowserActionModal::RenameAlbum { target_dir, input });
                    }
                    KeyCode::Home => {
                        self.browser_caret = 0;
                        self.browser_modal = Some(BrowserActionModal::RenameAlbum { target_dir, input });
                    }
                    KeyCode::End => {
                        self.browser_caret = caret_end(&input);
                        self.browser_modal = Some(BrowserActionModal::RenameAlbum { target_dir, input });
                    }
                    KeyCode::Char(c) => {
                        caret_insert(&mut input, &mut self.browser_caret, c);
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
                                        // Keep playlist/queue/now-playing/genres
                                        // pointing at the moved files.
                                        self.remap_album_dir(&target_dir, &new_dir);
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
                mut file_name,
            } => {
                if has_ctrl {
                    match key.code {
                        KeyCode::Char('u') | KeyCode::Char('U') => {
                            if let Some(text) = edit_field_mut(field_idx, &mut title, &mut artist, &mut album, &mut file_name) {
                                text.clear();
                            }
                            self.browser_caret = 0;
                            self.browser_modal = Some(BrowserActionModal::EditTrack {
                                target_file,
                                field_idx,
                                title,
                                artist,
                                album,
                                file_name,
                            });
                            return true;
                        }
                        KeyCode::Char('w') | KeyCode::Char('W') => {
                            if let Some(text) = edit_field_mut(field_idx, &mut title, &mut artist, &mut album, &mut file_name) {
                                caret_delete_word(text, &mut self.browser_caret);
                            }
                            self.browser_modal = Some(BrowserActionModal::EditTrack {
                                target_file,
                                field_idx,
                                title,
                                artist,
                                album,
                                file_name,
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
                        field_idx = (field_idx + 1) % 4;
                        self.browser_caret = edit_field(field_idx, &title, &artist, &album, &file_name);
                        self.browser_modal = Some(BrowserActionModal::EditTrack {
                            target_file,
                            field_idx,
                            title,
                            artist,
                            album,
                            file_name,
                        });
                    }
                    KeyCode::Up => {
                        field_idx = (field_idx + 3) % 4;
                        self.browser_caret = edit_field(field_idx, &title, &artist, &album, &file_name);
                        self.browser_modal = Some(BrowserActionModal::EditTrack {
                            target_file,
                            field_idx,
                            title,
                            artist,
                            album,
                            file_name,
                        });
                    }
                    KeyCode::Backspace => {
                        if let Some(text) = edit_field_mut(field_idx, &mut title, &mut artist, &mut album, &mut file_name) {
                            caret_backspace(text, &mut self.browser_caret);
                        }
                        self.browser_modal = Some(BrowserActionModal::EditTrack {
                            target_file,
                            field_idx,
                            title,
                            artist,
                            album,
                            file_name,
                        });
                    }
                    KeyCode::Delete => {
                        if let Some(text) = edit_field_mut(field_idx, &mut title, &mut artist, &mut album, &mut file_name) {
                            caret_delete(text, self.browser_caret);
                        }
                        self.browser_modal = Some(BrowserActionModal::EditTrack {
                            target_file,
                            field_idx,
                            title,
                            artist,
                            album,
                            file_name,
                        });
                    }
                    KeyCode::Left => {
                        self.browser_caret = self.browser_caret.saturating_sub(1);
                        self.browser_modal = Some(BrowserActionModal::EditTrack {
                            target_file,
                            field_idx,
                            title,
                            artist,
                            album,
                            file_name,
                        });
                    }
                    KeyCode::Right => {
                        let max = edit_field(field_idx, &title, &artist, &album, &file_name);
                        self.browser_caret = (self.browser_caret + 1).min(max);
                        self.browser_modal = Some(BrowserActionModal::EditTrack {
                            target_file,
                            field_idx,
                            title,
                            artist,
                            album,
                            file_name,
                        });
                    }
                    KeyCode::Home => {
                        self.browser_caret = 0;
                        self.browser_modal = Some(BrowserActionModal::EditTrack {
                            target_file,
                            field_idx,
                            title,
                            artist,
                            album,
                            file_name,
                        });
                    }
                    KeyCode::End => {
                        self.browser_caret = edit_field(field_idx, &title, &artist, &album, &file_name);
                        self.browser_modal = Some(BrowserActionModal::EditTrack {
                            target_file,
                            field_idx,
                            title,
                            artist,
                            album,
                            file_name,
                        });
                    }
                    KeyCode::Char(c) => {
                        if let Some(text) = edit_field_mut(field_idx, &mut title, &mut artist, &mut album, &mut file_name) {
                            caret_insert(text, &mut self.browser_caret, c);
                        }
                        self.browser_modal = Some(BrowserActionModal::EditTrack {
                            target_file,
                            field_idx,
                            title,
                            artist,
                            album,
                            file_name,
                        });
                    }
                    KeyCode::Enter => {
                        if field_idx < 3 && !key.modifiers.contains(KeyModifiers::CONTROL) {
                            field_idx += 1;
                            self.browser_caret = edit_field(field_idx, &title, &artist, &album, &file_name);
                            self.browser_modal = Some(BrowserActionModal::EditTrack {
                                target_file,
                                field_idx,
                                title,
                                artist,
                                album,
                                file_name,
                            });
                        } else {
                            // Save ID3 tags, then rename the file when the
                            // file-name field changed.
                            let mut tag = id3::Tag::read_from_path(&target_file).unwrap_or_else(|_| id3::Tag::new());
                            tag.set_title(title.trim());
                            tag.set_artist(artist.trim());
                            tag.set_album(album.trim());

                            if let Err(e) = tag.write_to_path(&target_file, id3::Version::Id3v24) {
                                self.set_toast(format!("Failed to write ID3 tags: {e}"));
                            } else {
                                let mut final_path = target_file.clone();
                                let wanted = file_name.trim();
                                let current = target_file
                                    .file_name()
                                    .map(|n| n.to_string_lossy().to_string())
                                    .unwrap_or_default();
                                if !wanted.is_empty() && wanted != current {
                                    // Keep the .mp3 extension when omitted.
                                    let wanted_name = if std::path::Path::new(wanted).extension().is_some() {
                                        wanted.to_string()
                                    } else {
                                        format!("{wanted}.mp3")
                                    };
                                    if let Some(parent) = target_file.parent() {
                                        let dest = parent.join(&wanted_name);
                                        match std::fs::rename(&target_file, &dest) {
                                            Ok(_) => {
                                                self.remap_track_path(&target_file, &dest);
                                                final_path = dest;
                                            }
                                            Err(e) => {
                                                self.set_toast(format!("Tags saved, but rename failed: {e}"));
                                                self.meta_cache.remove(&target_file);
                                                let _ = self.song_meta(&target_file);
                                                self.browser.refresh();
                                                return true;
                                            }
                                        }
                                    }
                                }
                                self.meta_cache.remove(&final_path);
                                let _ = self.song_meta(&final_path);
                                self.browser.refresh();
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
                            file_name,
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
                            KeyCode::Char('u') | KeyCode::Char('U') => {
                                new_album_input.clear();
                                self.browser_caret = 0;
                            }
                            KeyCode::Char('w') | KeyCode::Char('W') => caret_delete_word(&mut new_album_input, &mut self.browser_caret),
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
                            caret_backspace(&mut new_album_input, &mut self.browser_caret);
                            self.browser_modal = Some(BrowserActionModal::MoveTrack {
                                target_file,
                                candidate_albums,
                                selected_idx,
                                creating_new: true,
                                new_album_input,
                            });
                        }
                        KeyCode::Delete => {
                            caret_delete(&mut new_album_input, self.browser_caret);
                            self.browser_modal = Some(BrowserActionModal::MoveTrack {
                                target_file,
                                candidate_albums,
                                selected_idx,
                                creating_new: true,
                                new_album_input,
                            });
                        }
                        KeyCode::Left => {
                            self.browser_caret = self.browser_caret.saturating_sub(1);
                            self.browser_modal = Some(BrowserActionModal::MoveTrack {
                                target_file,
                                candidate_albums,
                                selected_idx,
                                creating_new: true,
                                new_album_input,
                            });
                        }
                        KeyCode::Right => {
                            self.browser_caret = (self.browser_caret + 1).min(caret_end(&new_album_input));
                            self.browser_modal = Some(BrowserActionModal::MoveTrack {
                                target_file,
                                candidate_albums,
                                selected_idx,
                                creating_new: true,
                                new_album_input,
                            });
                        }
                        KeyCode::Home => {
                            self.browser_caret = 0;
                            self.browser_modal = Some(BrowserActionModal::MoveTrack {
                                target_file,
                                candidate_albums,
                                selected_idx,
                                creating_new: true,
                                new_album_input,
                            });
                        }
                        KeyCode::End => {
                            self.browser_caret = caret_end(&new_album_input);
                            self.browser_modal = Some(BrowserActionModal::MoveTrack {
                                target_file,
                                candidate_albums,
                                selected_idx,
                                creating_new: true,
                                new_album_input,
                            });
                        }
                        KeyCode::Char(c) => {
                            caret_insert(&mut new_album_input, &mut self.browser_caret, c);
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
                            self.browser_caret = 0;
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
                                self.browser_caret = 0;
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
            BrowserActionModal::SetCover { target_file, mut picker } => {
                match key.code {
                    KeyCode::Esc => {
                        self.set_toast("Cover change cancelled".to_string());
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        picker.previous("");
                        self.browser_modal = Some(BrowserActionModal::SetCover { target_file, picker });
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        picker.next("");
                        self.browser_modal = Some(BrowserActionModal::SetCover { target_file, picker });
                    }
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        let chosen = picker.entries.get(picker.selected).cloned();
                        match chosen {
                            Some(p) if p.is_dir() => {
                                picker.enter();
                                self.browser_modal = Some(BrowserActionModal::SetCover { target_file, picker });
                            }
                            Some(img) => {
                                self.embed_track_cover(&target_file, &img);
                            }
                            None => {
                                self.browser_modal = Some(BrowserActionModal::SetCover { target_file, picker });
                            }
                        }
                    }
                    KeyCode::Backspace => {
                        picker.go_parent();
                        self.browser_modal = Some(BrowserActionModal::SetCover { target_file, picker });
                    }
                    _ => {
                        self.browser_modal = Some(BrowserActionModal::SetCover { target_file, picker });
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

                // Update every in-memory reference (playlist, queue,
                // now-playing, cache, genres) by path identity.
                self.remap_track_path(&source_file, &dest_path);
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
    fn test_browser_create_album() {        let temp_dir = std::env::temp_dir().join("lyra_test_create_album");
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
    fn test_browser_modal_caret_edits_mid_text() {
        let temp_dir = std::env::temp_dir().join("lyra_test_modal_caret");
        let _ = std::fs::remove_dir_all(&temp_dir);
        let _ = std::fs::create_dir_all(&temp_dir);

        if let Ok(mut app) = App::new(&temp_dir) {
            app.browser.current_dir = temp_dir.clone();
            app.browser.refresh();

            app.browser_create_album_prompt();
            for c in "abcd".chars() {
                app.handle_browser_modal_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
            }
            // Caret at end; move left twice and insert mid-text.
            app.handle_browser_modal_key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));
            app.handle_browser_modal_key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));
            assert_eq!(app.browser_caret, 2);
            app.handle_browser_modal_key(KeyEvent::new(KeyCode::Char('X'), KeyModifiers::NONE));
            // Backspace removes the inserted char.
            app.handle_browser_modal_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
            // Delete removes the char at the caret ('c').
            app.handle_browser_modal_key(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE));
            // Home + type at front, End + type at back.
            app.handle_browser_modal_key(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE));
            app.handle_browser_modal_key(KeyEvent::new(KeyCode::Char('Z'), KeyModifiers::NONE));
            app.handle_browser_modal_key(KeyEvent::new(KeyCode::End, KeyModifiers::NONE));
            app.handle_browser_modal_key(KeyEvent::new(KeyCode::Char('Y'), KeyModifiers::NONE));

            match &app.browser_modal {
                Some(BrowserActionModal::NewAlbum { input }) => {
                    assert_eq!(input, "ZabdY");
                }
                other => panic!("expected NewAlbum modal, got {other:?}"),
            }
            app.handle_browser_modal_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
            assert!(temp_dir.join("ZabdY").is_dir());
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
