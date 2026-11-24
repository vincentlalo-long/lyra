//! Keys for the NewAlbum modal.

use crossterm::event::{KeyCode, KeyEvent};

use crate::app::App;
use crate::browser::BrowserActionModal;

use super::super::caret::{
    caret_backspace, caret_delete, caret_delete_word, caret_end, caret_insert,
};

impl App {
    pub(super) fn handle_new_album_key(
        &mut self,
        key: KeyEvent,
        has_ctrl: bool,
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
            self.browser_modal = Some(BrowserActionModal::NewAlbum { input });
            return;
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
                            if let Some(pos) =
                                self.browser.entries.iter().position(|p| p == &new_dir)
                            {
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
}
