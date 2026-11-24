//! Keys for the MoveTrack modal (album list + inline new-album input).

use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent};

use crate::app::App;
use crate::browser::BrowserActionModal;

use super::super::caret::{
    caret_backspace, caret_delete, caret_delete_word, caret_end, caret_insert,
};

/// State carried by an open MoveTrack modal.
pub(super) struct MoveTrackFields {
    pub target_file: PathBuf,
    pub candidate_albums: Vec<PathBuf>,
    pub selected_idx: usize,
    pub creating_new: bool,
    pub new_album_input: String,
}

impl App {
    pub(super) fn handle_move_track_key(
        &mut self,
        key: KeyEvent,
        has_ctrl: bool,
        fields: MoveTrackFields,
    ) {
        let MoveTrackFields {
            target_file,
            candidate_albums,
            mut selected_idx,
            creating_new,
            mut new_album_input,
        } = fields;
        if creating_new {
            if has_ctrl {
                match key.code {
                    KeyCode::Char('u') | KeyCode::Char('U') => {
                        new_album_input.clear();
                        self.browser_caret = 0;
                    }
                    KeyCode::Char('w') | KeyCode::Char('W') => {
                        caret_delete_word(&mut new_album_input, &mut self.browser_caret)
                    }
                    _ => {}
                }
                self.browser_modal = Some(BrowserActionModal::MoveTrack {
                    target_file,
                    candidate_albums,
                    selected_idx,
                    creating_new: true,
                    new_album_input,
                });
                return;
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
                        let album_name = target_dir
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| "Album".into());
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
