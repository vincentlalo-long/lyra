use std::{path::PathBuf, time::Instant};

use super::events::{CoverCandidate, SearchItem};

pub struct DownloadState {
    pub query: String,
    pub is_input_active: bool,
    pub is_searching: bool,
    pub search_results: Vec<SearchItem>,
    pub selected_result: usize,
    pub result_list_state: ratatui::widgets::ListState,
    pub search_limit: usize,

    // Mode 2: Metadata Review / Edit Form
    pub show_metadata_form: bool,
    pub form_field_idx: usize, // 0: Title, 1: Artist, 2: Album, 3: Genre, 4: Save to (Directory), 5: Cover pick, 6: Lyrics, 7: Submit
    /// Caret position (in chars) inside the active text field (0..=3).
    /// Reset to end-of-text whenever the field changes or text is replaced.
    pub form_cursor: usize,
    pub form_url: String,
    pub form_title: String,
    pub form_artist: String,
    pub form_album: String,
    pub form_genre: String,
    pub existing_artists: Vec<String>,
    pub selected_artist_idx: Option<usize>,
    pub existing_albums: Vec<String>,
    pub selected_album_idx: Option<usize>,
    pub suggested_genres: Vec<String>,
    pub selected_genre_idx: Option<usize>,
    pub form_dir: PathBuf,
    pub form_cover_mode: String,
    /// Subtitle availability probe for the Lyrics row:
    /// None = probing, Some((manual, auto)).
    pub lyrics_probe: Option<(bool, bool)>,
    /// Studio provider set for cover search + final download
    /// (auto/all/web/itunes/deezer/caa/youtube). Cycled with `s` on
    /// the Cover row; `web` includes generic web results.
    pub cover_source: String,
    pub form_lyrics_mode: usize, // 0: All (Manual + Auto), 1: Creator Only (No Auto), 2: Disabled

    // Studio cover candidates for field 4 (None = Auto best, Some(i) = picked).
    pub cover_candidates: Vec<CoverCandidate>,
    pub selected_cover: Option<usize>,
    pub is_cover_loading: bool,
    pub form_custom_cover: Option<String>,

    // Cover Search & Preview Modal
    pub show_cover_picker_modal: bool,
    pub cover_picker_search: String,
    pub cover_picker_input_active: bool,
    pub cover_picker_selected: usize,
    pub cover_preview_art: Option<crate::cover::AlbumArt>,
    pub cover_preview_path: Option<String>,

    // Image file picker modal (to pick a local image file from disk)
    pub show_cover_file_picker: bool,
    pub cover_file_picker: crate::browser::FileBrowser,

    // Interactive crop-focus editor: fixed 1:1 frame moved over the
    // source image with arrows (focus only applies to center_crop).
    pub show_crop_modal: bool,
    pub crop_cursor: (f32, f32),
    pub crop_img_size: Option<(u32, u32)>,
    /// Confirmed focal point (0..1) sent as `--crop-focus`, if any.
    pub cover_focus: Option<(f32, f32)>,

    // Directory picker modal
    pub show_dir_picker: bool,
    pub dir_picker: crate::browser::FileBrowser,

    // Active download tracking
    pub is_downloading: bool,
    pub active_title: String,
    pub active_artist: String,
    pub progress_pct: f32,
    pub speed: String,
    pub eta: String,
    pub current_stage: String,
    pub last_error: Option<String>,
    pub last_completed: Option<String>,
    /// Last time a real progress event arrived (or the download started).
    /// Drives the fake crawl so the bar never looks frozen at 0%.
    last_progress_at: Instant,
}

/// Fake crawl speed (%/s) while no real progress arrives, and the cap it
/// never exceeds (real events always snap past it).
const FAKE_CRAWL_RATE_PCT_PER_SEC: f32 = 3.0;
const FAKE_CRAWL_CAP_PCT: f32 = 95.0;
/// A download counts as stalled when no real progress arrived for this long.
const STALL_THRESHOLD_SECS: u64 = 2;

impl DownloadState {
    pub fn new(initial_dir: &std::path::Path) -> Self {
        Self {
            query: String::new(),
            is_input_active: false,
            is_searching: false,
            search_results: Vec::new(),
            selected_result: 0,
            result_list_state: ratatui::widgets::ListState::default(),
            search_limit: 15,

            show_metadata_form: false,
            form_field_idx: 0,
            form_cursor: 0,
            form_url: String::new(),
            form_title: String::new(),
            form_artist: String::new(),
            form_album: String::new(),
            form_genre: String::new(),
            existing_artists: Vec::new(),
            selected_artist_idx: None,
            existing_albums: Vec::new(),
            selected_album_idx: None,
            suggested_genres: Vec::new(),
            selected_genre_idx: None,
            form_dir: initial_dir.to_path_buf(),
            form_cover_mode: "auto".to_string(),
            lyrics_probe: None,
            cover_source: "auto".to_string(),
            form_lyrics_mode: 0,
            cover_candidates: Vec::new(),
            selected_cover: None,
            is_cover_loading: false,
            form_custom_cover: None,

            show_cover_picker_modal: false,
            cover_picker_search: String::new(),
            cover_picker_input_active: false,
            cover_picker_selected: 0,
            cover_preview_art: None,
            cover_preview_path: None,

            show_cover_file_picker: false,
            cover_file_picker: crate::browser::FileBrowser::new(initial_dir),

            show_crop_modal: false,
            crop_cursor: (0.5, 0.5),
            crop_img_size: None,
            cover_focus: None,

            show_dir_picker: false,
            dir_picker: crate::browser::FileBrowser::new(initial_dir),

            is_downloading: false,
            active_title: String::new(),
            active_artist: String::new(),
            progress_pct: 0.0,
            speed: String::new(),
            eta: String::new(),
            current_stage: String::new(),
            last_error: None,
            last_completed: None,
            last_progress_at: Instant::now(),
        }
    }

    /// Mutable text of a typeable form field (0: Title, 1: Artist,
    /// 2: Album, 3: Genre). `None` for picker/button rows.
    fn form_text_mut(&mut self, idx: usize) -> Option<&mut String> {
        match idx {
            0 => Some(&mut self.form_title),
            1 => Some(&mut self.form_artist),
            2 => Some(&mut self.form_album),
            3 => Some(&mut self.form_genre),
            _ => None,
        }
    }

    fn form_text(&self, idx: usize) -> Option<&str> {
        match idx {
            0 => Some(&self.form_title),
            1 => Some(&self.form_artist),
            2 => Some(&self.form_album),
            3 => Some(&self.form_genre),
            _ => None,
        }
    }

    /// Switches the active form field, snapping the caret to end-of-text.
    pub fn move_form_field(&mut self, idx: usize) {
        self.form_field_idx = idx;
        self.snap_cursor_to_end();
    }

    /// Puts the caret at end-of-text of the current field.
    /// Call after any programmatic text replacement (suggestion cycling…).
    pub fn snap_cursor_to_end(&mut self) {
        let len = self.form_text(self.form_field_idx).map(|t| t.chars().count()).unwrap_or(0);
        self.form_cursor = len;
    }

    /// Clamps a char index to a valid caret position.
    fn byte_idx(text: &str, char_idx: usize) -> usize {
        text.char_indices()
            .map(|(b, _)| b)
            .nth(char_idx)
            .unwrap_or(text.len())
    }

    pub fn cursor_left(&mut self) {
        self.form_cursor = self.form_cursor.saturating_sub(1);
    }

    pub fn cursor_right(&mut self) {
        let max = self.form_text(self.form_field_idx).map(|t| t.chars().count()).unwrap_or(0);
        if self.form_cursor < max {
            self.form_cursor += 1;
        }
    }

    pub fn cursor_home(&mut self) {
        self.form_cursor = 0;
    }

    pub fn cursor_end(&mut self) {
        self.snap_cursor_to_end();
    }

    /// Inserts a char at the caret and advances it.
    pub fn cursor_insert(&mut self, c: char) {
        let cursor = self.form_cursor;
        if let Some(text) = self.form_text_mut(self.form_field_idx) {
            let byte = Self::byte_idx(text, cursor);
            text.insert(byte, c);
        }
        self.form_cursor = cursor + 1;
    }

    /// Backspace: deletes the char before the caret.
    pub fn cursor_backspace(&mut self) {
        let cursor = self.form_cursor;
        if cursor == 0 {
            return;
        }
        if let Some(text) = self.form_text_mut(self.form_field_idx) {
            let start = Self::byte_idx(text, cursor - 1);
            let end = Self::byte_idx(text, cursor);
            text.drain(start..end);
        }
        self.form_cursor = cursor - 1;
    }

    /// Delete: deletes the char at the caret.
    pub fn cursor_delete(&mut self) {
        let cursor = self.form_cursor;
        let len = self.form_text(self.form_field_idx).map(|t| t.chars().count()).unwrap_or(0);
        if cursor >= len {
            return;
        }
        if let Some(text) = self.form_text_mut(self.form_field_idx) {
            let start = Self::byte_idx(text, cursor);
            let end = Self::byte_idx(text, cursor + 1);
            text.drain(start..end);
        }
    }

    /// Ctrl+W: deletes the word before the caret (then trailing spaces).
    pub fn cursor_delete_word(&mut self) {
        let cursor = self.form_cursor;
        if cursor == 0 {
            return;
        }
        let mut new_cursor = 0;
        if let Some(text) = self.form_text_mut(self.form_field_idx) {
            let chars: Vec<char> = text.chars().collect();
            let mut start = cursor.min(chars.len());
            while start > 0 && chars[start - 1] == ' ' {
                start -= 1;
            }
            while start > 0 && chars[start - 1] != ' ' {
                start -= 1;
            }
            let b_start = Self::byte_idx(text, start);
            let b_end = Self::byte_idx(text, cursor.min(chars.len()));
            text.drain(b_start..b_end);
            new_cursor = start;
        }
        self.form_cursor = new_cursor;
    }

    /// Source image path for the crop editor: the picked local file,
    /// else the selected/downloaded candidate's cached file, if any.
    pub fn crop_source_path(&self) -> Option<String> {
        if let Some(custom) = &self.form_custom_cover {
            if std::path::Path::new(custom).is_file() {
                return Some(custom.clone());
            }
        }
        if let Some(i) = self.selected_cover {
            if let Some(cand) = self.cover_candidates.get(i) {
                if let Some(cached) = &cand.cached_path {
                    if std::path::Path::new(cached).is_file() {
                        return Some(cached.clone());
                    }
                }
            }
        }
        None
    }

    /// Opens the crop-focus editor for the current cover source.
    /// Returns false (with nothing opened) when no image is available.
    pub fn open_crop_modal(&mut self) -> bool {
        let Some(path) = self.crop_source_path() else {
            return false;
        };
        let Ok((w, h)) = image::image_dimensions(&path) else {
            return false;
        };
        if w == 0 || h == 0 {
            return false;
        }
        self.crop_img_size = Some((w, h));
        self.crop_cursor = self.cover_focus.unwrap_or((0.5, 0.5));
        self.show_crop_modal = true;
        true
    }

    /// Refresh decoded preview image based on currently selected candidate or custom local cover
    pub fn update_cover_preview(&mut self) {        let path_to_load = if self.cover_picker_selected == 0 {
            if let Some(custom) = &self.form_custom_cover {
                if std::path::Path::new(custom).exists() {
                    Some(custom.clone())
                } else {
                    None
                }
            } else {
                None
            }
        } else if !self.cover_candidates.is_empty() {
            let cand_idx = (self.cover_picker_selected - 1).min(self.cover_candidates.len() - 1);
            if let Some(cand) = self.cover_candidates.get(cand_idx) {
                if let Some(cached) = &cand.cached_path {
                    if std::path::Path::new(cached).exists() {
                        Some(cached.clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        if path_to_load != self.cover_preview_path {
            self.cover_preview_path = path_to_load.clone();
            self.cover_preview_art = match path_to_load {
                Some(ref p) => match image::open(p) {
                    Ok(img) => Some(crate::cover::AlbumArt::from_dynamic_image(img)),
                    Err(_) => None,
                },
                None => None,
            };
        }
    }

    /// Records a real progress event from the backend (resets the stall clock).
    pub fn set_progress(&mut self, percent: f32, speed: String, eta: String, stage: String) {
        self.progress_pct = percent;
        self.speed = speed;
        self.eta = eta;
        self.current_stage = stage;
        self.last_progress_at = Instant::now();
    }

    /// Resets progress tracking when a new download starts.
    pub fn reset_progress(&mut self) {
        self.progress_pct = 0.0;
        self.speed.clear();
        self.eta.clear();
        self.last_progress_at = Instant::now();
    }

    /// True while a download runs but no real progress arrived recently
    /// (lyrics / iTunes / tagging stages emit no percentages).
    pub fn is_stalled(&self) -> bool {
        self.is_downloading && self.last_progress_at.elapsed().as_secs() >= STALL_THRESHOLD_SECS
    }

    /// Percentage shown on the gauge: the real value, or a slow fake crawl
    /// toward the cap while stalled so the bar never freezes at 0%.
    /// Real events always win via `max` (monotonic within one download).
    pub fn display_pct(&self) -> u16 {
        if !self.is_downloading {
            return (self.progress_pct as u16).min(100);
        }
        let elapsed = self.last_progress_at.elapsed().as_secs_f32();
        let fake = (self.progress_pct + elapsed * FAKE_CRAWL_RATE_PCT_PER_SEC).min(FAKE_CRAWL_CAP_PCT);
        (self.progress_pct.max(fake) as u16).min(100)
    }

    /// Expands search limit by +10 results up to a maximum of 50.
    pub fn expand_search_limit(&mut self) -> usize {
        self.search_limit = (self.search_limit + 10).min(50);
        self.search_limit
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration as StdDuration;

    #[test]
    fn test_display_pct_crawls_while_stalled_and_caps() {
        let mut state = DownloadState::new(std::path::Path::new("/tmp"));
        assert!(!state.is_stalled());
        assert_eq!(state.display_pct(), 0);

        state.is_downloading = true;
        state.reset_progress();
        assert!(!state.is_stalled());
        assert_eq!(state.display_pct(), 0);

        // 10s without real events: fake crawl at 3%/s -> 30%, flagged stalled.
        state.last_progress_at = Instant::now() - StdDuration::from_secs(10);
        assert!(state.is_stalled());
        assert_eq!(state.display_pct(), 30);

        // Long stalls never pose as near-done: capped at 95%.
        state.last_progress_at = Instant::now() - StdDuration::from_secs(1000);
        assert_eq!(state.display_pct(), 95);

        // A fresh real event resets the clock and always wins.
        state.set_progress(40.0, String::new(), String::new(), "Downloading...".into());
        assert!(!state.is_stalled());
        assert_eq!(state.display_pct(), 40);

        // Real values above the fake cap pass through untouched.
        state.set_progress(97.0, String::new(), String::new(), "Downloading...".into());
        assert_eq!(state.display_pct(), 97);
    }

    #[test]
    fn test_update_cover_preview_loads_image() {
        let mut state = DownloadState::new(std::path::Path::new("/tmp"));
        let temp_img = std::env::temp_dir().join("test_cover_preview.png");
        let img = image::RgbImage::new(10, 10);
        let _ = img.save(&temp_img);

        state.form_custom_cover = Some(temp_img.to_string_lossy().to_string());
        state.update_cover_preview();

        assert!(state.cover_preview_art.is_some());
        assert_eq!(state.cover_preview_path, Some(temp_img.to_string_lossy().to_string()));

        let _ = std::fs::remove_file(&temp_img);
    }

    #[test]
    fn test_expand_search_limit() {
        let mut state = DownloadState::new(std::path::Path::new("/tmp"));
        assert_eq!(state.search_limit, 15);
        assert_eq!(state.expand_search_limit(), 25);
        assert_eq!(state.expand_search_limit(), 35);
        assert_eq!(state.expand_search_limit(), 45);
        assert_eq!(state.expand_search_limit(), 50); // caps at 50
        assert_eq!(state.expand_search_limit(), 50);
    }
}
