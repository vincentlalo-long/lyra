//! Keys for the EditTrack modal.

use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use id3::TagLike;

use crate::app::App;
use crate::browser::BrowserActionModal;

use super::super::caret::{
    caret_backspace, caret_delete, caret_delete_word, caret_insert, edit_field, edit_field_mut,
};

/// Editable state carried by an open EditTrack modal.
pub(super) struct EditTrackFields {
    pub target_file: PathBuf,
    pub field_idx: usize,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub file_name: String,
}

impl App {
    pub(super) fn handle_edit_track_key(
        &mut self,
        key: KeyEvent,
        has_ctrl: bool,
        fields: EditTrackFields,
    ) {
        let EditTrackFields {
            target_file,
            mut field_idx,
            mut title,
            mut artist,
            mut album,
            mut file_name,
        } = fields;
        if has_ctrl {
            match key.code {
                KeyCode::Char('u') | KeyCode::Char('U') => {
                    if let Some(text) = edit_field_mut(
                        field_idx,
                        &mut title,
                        &mut artist,
                        &mut album,
                        &mut file_name,
                    ) {
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
                    return;
                }
                KeyCode::Char('w') | KeyCode::Char('W') => {
                    if let Some(text) = edit_field_mut(
                        field_idx,
                        &mut title,
                        &mut artist,
                        &mut album,
                        &mut file_name,
                    ) {
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
                    return;
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
                if let Some(text) = edit_field_mut(
                    field_idx,
                    &mut title,
                    &mut artist,
                    &mut album,
                    &mut file_name,
                ) {
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
                if let Some(text) = edit_field_mut(
                    field_idx,
                    &mut title,
                    &mut artist,
                    &mut album,
                    &mut file_name,
                ) {
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
                if let Some(text) = edit_field_mut(
                    field_idx,
                    &mut title,
                    &mut artist,
                    &mut album,
                    &mut file_name,
                ) {
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
                    let mut tag =
                        id3::Tag::read_from_path(&target_file).unwrap_or_else(|_| id3::Tag::new());
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
                            let wanted_name = if std::path::Path::new(wanted).extension().is_some()
                            {
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
                                        self.set_toast(format!(
                                            "Tags saved, but rename failed: {e}"
                                        ));
                                        self.meta_cache.remove(&target_file);
                                        let _ = self.song_meta(&target_file);
                                        self.browser.refresh();
                                        return;
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
}
