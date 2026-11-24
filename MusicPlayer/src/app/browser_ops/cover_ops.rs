//! Embedded cover-art replacement for a single track.

use std::path::Path;

use id3::TagLike;

use crate::app::App;

impl App {
    /// Embeds `image_path` as the track's front cover (ID3 APIC) and saves
    /// a `cover.jpg` next to the track for file-based fallbacks.
    pub(super) fn embed_track_cover(&mut self, target_file: &Path, image_path: &Path) {
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
            let _ = std::fs::write(
                parent.join("cover.jpg"),
                std::fs::read(image_path).unwrap_or_default(),
            );
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
}
