//! Keys for the SetCover image picker modal.

use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent};

use crate::app::App;
use crate::browser::{BrowserActionModal, FileBrowser};

impl App {
    pub(super) fn handle_set_cover_key(
        &mut self,
        key: KeyEvent,
        target_file: PathBuf,
        mut picker: FileBrowser,
    ) {
        match key.code {
            KeyCode::Esc => {
                self.set_toast("Cover change cancelled".to_string());
            }
            KeyCode::Up | KeyCode::Char('k') => {
                picker.previous("");
                self.browser_modal = Some(BrowserActionModal::SetCover {
                    target_file,
                    picker,
                });
            }
            KeyCode::Down | KeyCode::Char('j') => {
                picker.next("");
                self.browser_modal = Some(BrowserActionModal::SetCover {
                    target_file,
                    picker,
                });
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                let chosen = picker.entries.get(picker.selected).cloned();
                match chosen {
                    Some(p) if p.is_dir() => {
                        picker.enter();
                        self.browser_modal = Some(BrowserActionModal::SetCover {
                            target_file,
                            picker,
                        });
                    }
                    Some(img) => {
                        self.embed_track_cover(&target_file, &img);
                    }
                    None => {
                        self.browser_modal = Some(BrowserActionModal::SetCover {
                            target_file,
                            picker,
                        });
                    }
                }
            }
            KeyCode::Backspace => {
                picker.go_parent();
                self.browser_modal = Some(BrowserActionModal::SetCover {
                    target_file,
                    picker,
                });
            }
            _ => {
                self.browser_modal = Some(BrowserActionModal::SetCover {
                    target_file,
                    picker,
                });
            }
        }
    }
}
