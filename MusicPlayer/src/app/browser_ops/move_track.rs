//! Filesystem move of one track into a target album.

use std::path::{Path, PathBuf};

use id3::TagLike;

use crate::app::App;

impl App {
    /// Moves a track to target destination album directory and updates ID3 album tag & playlist
    pub(super) fn execute_move_track(
        &mut self,
        source_file: PathBuf,
        target_dir: &Path,
        album_name: &str,
    ) {
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
}
