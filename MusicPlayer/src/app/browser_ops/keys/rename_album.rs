//! Keys for the RenameAlbum modal.

use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent};

use crate::app::App;
use crate::browser::BrowserActionModal;

use super::super::caret::{
    caret_backspace, caret_delete, caret_delete_word, caret_end, caret_insert,
};

impl App {
    pub(super) fn handle_rename_album_key(
        &mut self,
        key: KeyEvent,
        has_ctrl: bool,
        target_dir: PathBuf,
        mut input: String,
    ) {
        if has_ctrl {
            match key.code {
                KeyCode::Char('u') | KeyCode::Char('U') => {
                    input.clear();
                    self.browser_caret = 0;
                }
                KeyCode::Char('w') | KeyCode::Char('W') => {
                    caret_delete_word(&mut input, &mut self.browser_caret)
                }
                _ => {}
            }
            self.browser_modal = Some(BrowserActionModal::RenameAlbum { target_dir, input });
            return;
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
                                if let Some(pos) =
                                    self.browser.entries.iter().position(|p| p == &new_dir)
                                {
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
}
