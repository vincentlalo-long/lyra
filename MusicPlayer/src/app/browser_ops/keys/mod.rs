//! Keyboard dispatch for browser action modals.
//!
//! The outer function owns modal take/restore and global shortcuts (Alt holds,
//! Ctrl+C cancels); each modal's editing rules live in its own file.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::App;
use crate::browser::BrowserActionModal;

mod edit_track;
mod move_track;
mod new_album;
mod rename_album;
mod set_cover;

impl App {
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
            BrowserActionModal::NewAlbum { input } => {
                self.handle_new_album_key(key, has_ctrl, input);
            }
            BrowserActionModal::RenameAlbum { target_dir, input } => {
                self.handle_rename_album_key(key, has_ctrl, target_dir, input);
            }
            BrowserActionModal::EditTrack {
                target_file,
                field_idx,
                title,
                artist,
                album,
                file_name,
            } => {
                self.handle_edit_track_key(
                    key,
                    has_ctrl,
                    edit_track::EditTrackFields {
                        target_file,
                        field_idx,
                        title,
                        artist,
                        album,
                        file_name,
                    },
                );
            }
            BrowserActionModal::MoveTrack {
                target_file,
                candidate_albums,
                selected_idx,
                creating_new,
                new_album_input,
            } => {
                self.handle_move_track_key(
                    key,
                    has_ctrl,
                    move_track::MoveTrackFields {
                        target_file,
                        candidate_albums,
                        selected_idx,
                        creating_new,
                        new_album_input,
                    },
                );
            }
            BrowserActionModal::SetCover {
                target_file,
                picker,
            } => {
                self.handle_set_cover_key(key, target_file, picker);
            }
        }

        true
    }
}
