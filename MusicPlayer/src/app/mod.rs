use std::path::{Path, PathBuf};
#[cfg(feature = "mpris")]
use std::sync::mpsc;
use std::sync::mpsc as plugin_mpsc;
use std::time::Instant;
use std::collections::HashMap;
use anyhow::Result;
use crate::{
    audio::AudioPlayer,
    browser::FileBrowser,
    config::Config,
    cover::AlbumArt,
    lyrics::Lyrics,
    playlist::Playlist,
    queue::PlayQueue,
    scanner::Scanner,
};
// Genre tags (sidecar + ID3 TCON). Absent without the feature.
#[cfg(feature = "genre")]
use crate::genre::{GenreDB, GenrePicker};
// MPRIS bridge (dynamic island / playerctl). Absent without the feature.
#[cfg(feature = "mpris")]
use crate::mpris::{MediaKey, MprisHandle};
// Download plugin state (absent without the `download` feature).
#[cfg(feature = "download")]
use crate::download::{DownloadState, Downloader};


#[cfg(feature = "genre")]
pub mod genre_ops;
pub mod playback;
pub mod plugin_ops;
pub mod queue_ops;
#[cfg(feature = "download")]
pub mod download_ops;
pub mod browser_ops;

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum RepeatMode {
    Playlist, // Loop entire playlist
    Track,    // Loop current single track
    Off,      // Stop when reaching end of playlist
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum ViewMode {
    Playlist,
    Queue,
    Browser,
    /// Installed interactive extensions workspace (e.g. YouTube Downloader, OCR Lyrics, Radio).
    Extensions,
    /// Plugin manager, settings & extension store.
    Plugins,
}

/// Playlist import/export modal state.
#[derive(Clone, Debug)]
pub struct PlaylistIoModal {
    pub mode: PlaylistIoMode,
    pub input: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlaylistIoMode {
    Import,
    Export,
}

pub struct App {
    pub view_mode: ViewMode,
    pub browser: FileBrowser,
    pub playlist: Playlist,
    /// Ephemeral scratchpad: what to play next. Drains before the library.
    pub queue: PlayQueue,
    pub music_folder: PathBuf,
    pub audio: AudioPlayer,
    pub scanner: Scanner,
    pub lyrics: Option<Lyrics>,
    pub search_query: String,
    pub is_searching: bool,
    pub show_help: bool,
    pub help_scroll: u16,
    pub plugin_selected: usize,
    pub plugins: Vec<crate::plugin::ManagedPlugin>,
    pub config_paths: Vec<crate::plugin::ConfigPathItem>,
    pub downloadable_plugins: Vec<crate::plugin::DownloadablePlugin>,
    pub editing_path_index: Option<usize>,
    pub editing_path_input: String,
    pub show_plugin_store: bool,
    pub store_selected: usize,
    pub notifications_enabled: bool,
    pub repeat_mode: RepeatMode,
    pub cover: Option<AlbumArt>,
    pub current_playing_path: Option<PathBuf>,
    pub kitty_cover_rect: Option<ratatui::layout::Rect>,
    pub last_kitty_rendered: Option<(Option<PathBuf>, ratatui::layout::Rect)>,
    #[cfg(feature = "download")]
    pub download: DownloadState,
    #[cfg(feature = "download")]
    pub downloader: Downloader,
    /// MPRIS bridge feeding the desktop dynamic island / playerctl / media keys.
    /// Degrades to a silent no-op without D-Bus (see `mpris.rs`).
    #[cfg(feature = "mpris")]
    pub mpris: MprisHandle,
    /// Island / media-key presses, drained every main-loop tick.
    #[cfg(feature = "mpris")]
    pub media_rx: mpsc::Receiver<MediaKey>,
    /// Transient one-line feedback ("Queued #3: ..."), expires after a few seconds.
    pub toast: Option<(String, Instant)>,
    /// Queue-replace flow (`e`): index in `queue.items` awaiting a
    /// replacement pick from the Browser. `None` when inactive.
    pub pending_replace: Option<usize>,
    /// Genre tags (sidecar + ID3 TCON). See `genre.rs`.
    #[cfg(feature = "genre")]
    pub genre_db: GenreDB,
    /// Active multiple-choice genre filters on Playlist view (empty = no filter).
    /// Combines with `/` search (AND). Cleared with Esc.
    #[cfg(feature = "genre")]
    pub genre_filters: Vec<String>,
    /// Genre picker modal (`f`). `None` when closed.
    #[cfg(feature = "genre")]
    pub genre_picker: Option<GenrePicker>,
    /// Interactive Genre tagger / editor modal (`t`). `None` when closed.
    #[cfg(feature = "genre")]
    pub genre_tagger: Option<crate::genre::GenreTaggerState>,
    /// Lyrics plugin toggle (defaults to false: studio layout without lyrics).
    pub show_lyrics: bool,
    /// Per-file metadata cache (ID3 + duration probe). Filled lazily by
    /// [`App::song_meta`]; files never change mid-session so no invalidation.
    meta_cache: HashMap<PathBuf, crate::meta::SongMeta>,
    /// Tracks which playlist row is focused and when it gained focus,
    /// used to drive the smooth marquee title scroll loop.
    pub selected_scroll_tracker: (usize, Instant),
    /// Tracks which queue row is focused and when it gained focus,
    /// used to drive the smooth marquee title scroll loop for queue items.
    pub queue_scroll_tracker: (usize, Instant),
    /// Interactive browser actions: new album, rename album, edit track ID3, move track.
    pub browser_modal: Option<crate::browser::BrowserActionModal>,
    /// Playlist import/export modal (M3U/PLS).
    pub playlist_io_modal: Option<PlaylistIoModal>,
    /// Caret (char index) inside the active browser-modal text input.
    /// Reset to end-of-text whenever a modal opens or its field changes.
    pub browser_caret: usize,
    /// Screen rect of the draggable seekbar (set every frame by
    /// `ui::controls`). `None` before the first render.
    pub seekbar_rect: Option<ratatui::layout::Rect>,
    /// True while the user is dragging the seekbar with the mouse.
    /// While set, `Drag` events keep re-seeking (debounced in audio).
    pub seeking_drag: bool,
    /// Active semantic theme (roles). Changed via `set_theme()`.
    pub theme: crate::theme::Theme,
    /// Active theme preset name (`mocha` default).
    pub theme_name: String,
    /// Sleep timer: `Some(instant)` when active, `None` when disabled.
    /// Playback is paused when the timer expires.
    pub sleep_timer: Option<Instant>,
    /// Sleep timer duration in minutes.
    pub sleep_timer_minutes: u64,
    /// Auto-embed cover art into ID3 after download.
    pub cover_embed: bool,
    /// Mouse support enabled (from config). Can be toggled at runtime.
    pub mouse_enabled: bool,
    /// Per-row rects for the visible playlist (used by mouse click detection).
    pub playlist_row_rects: Vec<(u16, ratatui::layout::Rect, usize)>,
    /// Per-row rects for the visible browser.
    pub browser_row_rects: Vec<(u16, ratatui::layout::Rect, usize)>,
    /// Per-row rects for the visible queue.
    pub queue_row_rects: Vec<(u16, ratatui::layout::Rect, usize)>,
    /// Rect of the volume bar (used by mouse click/drag).
    pub volume_bar_rect: Option<ratatui::layout::Rect>,
    /// Last click info for double-click detection: (instant, row_index, view_mode).
    pub last_click: Option<(std::time::Instant, usize, ViewMode)>,
    /// Background community-plugin installer: sending side (cloned per job).
    pub plugin_dl_tx: plugin_mpsc::Sender<crate::plugin_dl::InstallEvent>,
    /// Receiving side, drained every main-loop tick (`check_plugin_downloads`).
    pub plugin_dl_rx: plugin_mpsc::Receiver<crate::plugin_dl::InstallEvent>,
    /// Id of the plugin currently downloading, if any (one at a time).
    pub plugin_dl_active: Option<String>,
    /// Override for the community-plugin data root (tests / dev).
    /// `None` = resolve via `DownloadablePlugin::data_root()`.
    pub plugin_data_root: Option<PathBuf>,
}

/// How long a toast stays visible.
const TOAST_TTL_SECS: u64 = 3;
/// Cap for the in-memory song metadata cache. Without a cap a 10k-song
/// library pins 10k `PathBuf + 3xString + Duration` for the whole session.
/// When full we evict everything and start fresh (cheap: re-probed lazily).
const MAX_META_CACHE: usize = 1500;

pub fn short_name(path: &Path) -> String {
    path.file_stem()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Unknown".to_string())
}

impl App {
    pub fn new(music_folder: &Path) -> Result<Self> {
        let config = crate::config::Config::load();
        let (download_dir, raw_download_dir) = config.resolve_download_dir(music_folder);
        let mut config_paths = crate::plugin::ConfigPathItem::default_paths(music_folder);
        if let Some(item) = config_paths.iter_mut().find(|p| p.id == "download_dir") {
            item.path = raw_download_dir;
        }

        #[cfg(feature = "mpris")]
        let (mpris, media_rx) = crate::mpris::spawn();
        let (plugin_dl_tx, plugin_dl_rx) =
            plugin_mpsc::channel::<crate::plugin_dl::InstallEvent>();
        let mut app = Self {
            view_mode: ViewMode::Playlist,
            browser: FileBrowser::new(music_folder),
            playlist: Playlist::new(music_folder),
            queue: PlayQueue::load(),
            music_folder: music_folder.to_path_buf(),
            audio: AudioPlayer::new()?,
            scanner: Scanner::new(),
            lyrics: None,
            search_query: String::new(),
            is_searching: false,
            show_help: false,
            help_scroll: 0,
            plugin_selected: 0,
            plugins: {
                crate::plugin::ManagedPlugin::load_and_merge_plugins()
            },
            config_paths,
            downloadable_plugins: {
                let mut dps = crate::plugin::DownloadablePlugin::available_community_plugins();
                let saved = crate::plugin::ManagedPlugin::load_and_merge_plugins();
                for dp in &mut dps {
                    if saved.iter().any(|p| p.id == dp.id) {
                        dp.is_installed = true;
                    }
                }
                // Reconcile with disk: mock-era flags without files are cleared,
                // manually-placed plugin dirs are picked up.
                crate::plugin::DownloadablePlugin::refresh_installed_from_disk(&mut dps);
                dps
            },
            editing_path_index: None,
            editing_path_input: String::new(),
            show_plugin_store: false,
            store_selected: 0,
            notifications_enabled: true,
            repeat_mode: RepeatMode::Playlist,
            cover: None,
            current_playing_path: None,
            kitty_cover_rect: None,
            last_kitty_rendered: None,
            #[cfg(feature = "download")]
            download: DownloadState::new(&download_dir),
            #[cfg(feature = "download")]
            downloader: Downloader::new(),
            #[cfg(feature = "mpris")]
            mpris,
            #[cfg(feature = "mpris")]
            media_rx,
            toast: None,
            pending_replace: None,
            #[cfg(feature = "genre")]
            genre_db: GenreDB::load(),
            #[cfg(feature = "genre")]
            genre_filters: Vec::new(),
            #[cfg(feature = "genre")]
            genre_picker: None,
            #[cfg(feature = "genre")]
            genre_tagger: None,
            show_lyrics: false,
            meta_cache: HashMap::new(),
            selected_scroll_tracker: (0, Instant::now()),
            queue_scroll_tracker: (0, Instant::now()),
            browser_modal: None,
            playlist_io_modal: None,
            browser_caret: 0,
            seekbar_rect: None,
            seeking_drag: false,
            theme: crate::theme::Theme::mocha(),
            theme_name: "mocha".to_string(),
            sleep_timer: None,
            sleep_timer_minutes: 0,
            cover_embed: true,
            mouse_enabled: true,
            playlist_row_rects: Vec::new(),
            browser_row_rects: Vec::new(),
            queue_row_rects: Vec::new(),
            volume_bar_rect: None,
            last_click: None,
            plugin_dl_tx,
            plugin_dl_rx,
            plugin_dl_active: None,
            plugin_data_root: None,
        };
        // Theme preset: LYRA_THEME env wins, then config file `theme`.
        let env_theme = std::env::var("LYRA_THEME").ok();
        let theme_choice = env_theme.clone().or(config.theme.clone());
        if let Some(name) = theme_choice {
            app.set_theme(&name);
            if let (Some(env), Some(config_val)) = (&env_theme, &config.theme) {
                if env != config_val {
                    eprintln!(
                        "Warning: LYRA_THEME='{env}' overrides saved theme='{config_val}' in config.toml."
                    );
                }
            }
        }
        // Validate config values and warn about anything that looks wrong.
        for warning in config.validate() {
            eprintln!("Warning: {warning}");
        }
        // Derive runtime flags from saved plugin states (a plugin disabled
        // in a previous session stays off — no recompile needed).
        // Lyrics layout defaults ON when its plugin ships enabled.
        let lyrics_on = app.plugin_enabled("lyrics");
        app.sync_plugin_runtime_flags(lyrics_on);
        // Persisted YouTube search result limit (Plugins settings row / E).
        #[cfg(feature = "download")]
        if let Some(n) = config.search_limit {
            if (1..=50).contains(&n) {
                app.download.search_limit = n;
            }
        }
        app.cover_embed = config.cover_embed;
        app.mouse_enabled = config.mouse;
        Ok(app)
    }

    /// Toggle lyrics display mode (Studio Mode <-> Karaoke Mode).
    /// Refused while the lyrics plugin is disabled.
    pub fn toggle_lyrics(&mut self) {
        if !self.plugin_enabled("lyrics") {
            self.set_toast("Lyrics plugin is disabled (enable it in [5] Plugins)".to_string());
            return;
        }
        self.show_lyrics = !self.show_lyrics;
        self.set_toast(if self.show_lyrics {
            "Lyrics: ON (Karaoke)".to_string()
        } else {
            "Lyrics: OFF (Studio)".to_string()
        });
    }

    /// Name of the upcoming track (preferring queue, falling back to library).
    /// Respects an active playlist filter, like audio advance does.
    pub fn get_up_next_name(&mut self) -> Option<String> {
        if let Some(next_queue) = self.queue.items.front() {
            return Some(format!("{} (queue)", short_name(next_queue)));
        }
        #[cfg(feature = "genre")]
        let indices = self.visible_playlist_indices();
        #[cfg(not(feature = "genre"))]
        let indices = {
            let q = self.search_query.clone();
            self.playlist.filtered_indices(&q)
        };
        let filter_active = !self.search_query.is_empty() || {
            #[cfg(feature = "genre")]
            {
                self.plugin_enabled("genre") && !self.genre_filters.is_empty()
            }
            #[cfg(not(feature = "genre"))]
            {
                false
            }
        };
        if filter_active {
            if indices.is_empty() {
                return None;
            }
            let pos = match self.playlist.playing_index {
                Some(cur) => indices
                    .iter()
                    .position(|&i| i == cur)
                    .map(|p| (p + 1) % indices.len()),
                None => Some(0),
            };
            return pos
                .and_then(|p| indices.get(p))
                .and_then(|&i| self.playlist.songs.get(i))
                .map(|next_song| format!("{} (library)", short_name(next_song)));
        }
        if let Some(playing_idx) = self.playlist.playing_index {
            if !self.playlist.songs.is_empty() {
                let next_idx = (playing_idx + 1) % self.playlist.songs.len();
                if let Some(next_song) = self.playlist.songs.get(next_idx) {
                    return Some(format!("{} (library)", short_name(next_song)));
                }
            }
        }
        None
    }

    /// Returns cached metadata (ID3 tags + duration) for `path`, probing it
    /// lazily on first access and storing it in memory for the rest of the session.
    /// Capped at [`MAX_META_CACHE`] entries to avoid pinning huge libraries in RAM.
    pub fn song_meta(&mut self, path: &Path) -> crate::meta::SongMeta {
        if self.meta_cache.len() >= MAX_META_CACHE && !self.meta_cache.contains_key(path) {
            // Simple eviction: drop half to amortize clearing cost.
            // HashMap order is arbitrary but this bounds RAM without LRU bookkeeping.
            let to_drop: Vec<PathBuf> = self.meta_cache.keys().take(MAX_META_CACHE / 2).cloned().collect();
            for k in to_drop {
                self.meta_cache.remove(&k);
            }
        }
        self.meta_cache
            .entry(path.to_path_buf())
            .or_insert_with(|| crate::meta::song_meta(path))
            .clone()
    }

    /// Explicitly drop one cached entry (call after rename/delete/move).
    #[allow(dead_code)]
    pub fn invalidate_meta(&mut self, path: &Path) {
        self.meta_cache.remove(path);
    }

    /// Switch the semantic theme by preset name. Unknown names fall back
    /// to Mocha (see `Theme::from_name`). Persists nothing by itself —
    /// callers save `theme` into the config file when appropriate.
    pub fn set_theme(&mut self, name: &str) {
        let t = crate::theme::Theme::from_name(name);
        self.theme_name = t.name.to_string();
        self.theme = t;
    }

    /// Cycle mocha -> gruvbox -> tokyo-night -> dracula -> nord -> latte.
    /// Used by the `X` keybinding and the theme plugin preview.
    pub fn cycle_theme(&mut self) {
        let next = match self.theme_name.as_str() {
            "mocha" => "gruvbox",
            "gruvbox" => "tokyo-night",
            "tokyo-night" => "dracula",
            "dracula" => "nord",
            "nord" => "latte",
            _ => "mocha",
        };
        self.set_theme(next);
        self.set_toast(format!("Theme: {next}"));
        let _ = Config::save_setting("theme", next);
    }

    /// Cycle sleep timer: off -> 15 -> 30 -> 60 -> 90 -> 120 -> off.
    /// When active, playback is paused after `sleep_timer_minutes` elapse.
    pub fn cycle_sleep_timer(&mut self) {
        let next_minutes = match self.sleep_timer_minutes {
            0 => 15,
            15 => 30,
            30 => 60,
            60 => 90,
            90 => 120,
            _ => 0,
        };
        self.sleep_timer_minutes = next_minutes;
        if next_minutes > 0 {
            self.sleep_timer = Some(Instant::now());
            self.set_toast(format!("Sleep timer: {next_minutes} min"));
        } else {
            self.sleep_timer = None;
            self.set_toast("Sleep timer: off".to_string());
        }
    }

    /// Reload theme preset from config.toml at runtime (e.g. after the
    /// theme-packs plugin writes a new theme without restarting lyra).
    /// No-op if the saved theme matches the current one or is invalid.
    pub fn reload_theme_from_config(&mut self) {
        let config = crate::config::Config::load();
        let Some(saved) = config.theme else { return };
        if saved == self.theme_name {
            return;
        }
        let t = crate::theme::Theme::from_name(&saved);
        let theme_name = t.name.to_string();
        if theme_name == self.theme_name {
            return;
        }
        self.theme_name = theme_name.clone();
        self.theme = t;
        self.set_toast(format!("Theme reloaded: {theme_name}"));
    }

    /// Shows a transient one-line feedback message in the header.
    pub fn set_toast(&mut self, msg: String) {
        self.toast = Some((msg, Instant::now()));
    }

    /// Current toast text, if still fresh.
    pub fn toast_text(&self) -> Option<&str> {
        self.toast.as_ref().and_then(|(msg, at)| {
            if at.elapsed().as_secs() < TOAST_TTL_SECS {
                Some(msg.as_str())
            } else {
                None
            }
        })
    }

    pub fn toggle_view(&mut self) {
        self.view_mode = match self.view_mode {
            ViewMode::Playlist => ViewMode::Queue,
            ViewMode::Queue => ViewMode::Browser,
            ViewMode::Browser => ViewMode::Extensions,
            ViewMode::Extensions => ViewMode::Plugins,
            ViewMode::Plugins => ViewMode::Playlist,
        };
    }


    /// Removes the selected song from the active playlist session.
    pub fn remove_playlist_selected(&mut self) {
        if let Some(removed) = self.playlist.remove_selected() {
            let name = short_name(&removed);
            self.set_toast(format!("Removed from playlist: {name}"));
        }
    }
}

impl App {
    pub fn start_scan(&mut self) {
        if self.scanner.is_scanning || self.scanner.pending_result.is_some() {
            return;
        }

        let target_dir = self.browser.current_dir.clone();

        // Safety check 1: Root directory "/"
        if target_dir == Path::new("/") {
            self.set_toast("⚠️ Cannot scan root '/'. Navigate to a music folder first.".to_string());
            return;
        }

        // Safety check 2: Entire $HOME directly
        if let Ok(home) = std::env::var("HOME") {
            if target_dir == PathBuf::from(&home) && target_dir != self.music_folder {
                self.set_toast("⚠️ Cannot scan entire $HOME. Navigate into a subfolder first.".to_string());
                return;
            }
        }

        let include_hidden = self.browser.show_hidden;
        self.scanner.start(target_dir, include_hidden);
    }

    pub fn check_scan(&mut self) {
        if let Some(found_songs) = self.scanner.tick() {
            if found_songs.is_empty() {
                self.set_toast("Scan complete: 0 MP3 files found".to_string());
            } else {
                let count = found_songs.len();
                self.scanner.pending_result = Some(found_songs);
                self.set_toast(format!("Scan complete: {count} songs found. Choose action:"));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_song_meta_caching() {
        let temp_dir = std::env::temp_dir();
        if let Ok(mut app) = App::new(&temp_dir) {
            let dummy_path = temp_dir.join("Artist - Title.mp3");
            let meta1 = app.song_meta(&dummy_path);
            assert_eq!(meta1.artist, "Artist");
            assert_eq!(meta1.title, "Title");
            assert!(app.meta_cache.contains_key(&dummy_path));

            let meta2 = app.song_meta(&dummy_path);
            assert_eq!(meta2.artist, meta1.artist);
            assert_eq!(meta2.title, meta1.title);
        }
    }

    #[test]
    fn test_up_next_name() {
        let temp_dir = std::env::temp_dir();
        if let Ok(mut app) = App::new(&temp_dir) {
            app.queue.clear();
            assert_eq!(app.get_up_next_name(), None);
            let s1 = temp_dir.join("Song 1.mp3");
            let s2 = temp_dir.join("Song 2.mp3");
            app.playlist.set_songs(vec![s1.clone(), s2.clone()]);
            app.playlist.playing_index = Some(0);
            assert_eq!(app.get_up_next_name(), Some("Song 2 (library)".to_string()));

            app.queue.items.push_back(temp_dir.join("Queued Song.mp3"));
            assert_eq!(app.get_up_next_name(), Some("Queued Song (queue)".to_string()));
        }
    }

    #[test]
    fn test_remove_playlist_selected() {
        let temp_dir = std::env::temp_dir();
        if let Ok(mut app) = App::new(&temp_dir) {
            let s1 = temp_dir.join("Song1.mp3");
            let s2 = temp_dir.join("Song2.mp3");
            app.playlist.set_songs(vec![s1.clone(), s2.clone()]);
            app.playlist.selected = 0;

            app.remove_playlist_selected();
            assert_eq!(app.playlist.songs.len(), 1);
            assert_eq!(app.playlist.songs[0], s2);
            assert!(app.toast_text().is_some_and(|t| t.contains("Removed from playlist")));
        }
    }

    #[test]
    fn test_toggle_view() {
        let temp_dir = std::env::temp_dir();
        if let Ok(mut app) = App::new(&temp_dir) {
            assert_eq!(app.view_mode, ViewMode::Playlist);
            app.toggle_view();
            assert_eq!(app.view_mode, ViewMode::Queue);
            app.toggle_view();
            assert_eq!(app.view_mode, ViewMode::Browser);
            app.toggle_view();
            assert_eq!(app.view_mode, ViewMode::Extensions);
            app.toggle_view();
            assert_eq!(app.view_mode, ViewMode::Plugins);
            app.toggle_view();
            assert_eq!(app.view_mode, ViewMode::Playlist);
        }
    }

    #[test]
    fn test_plugins_navigation_and_bounds() {
        let temp_dir = std::env::temp_dir();
        if let Ok(mut app) = App::new(&temp_dir) {
            app.view_mode = ViewMode::Plugins;
            assert_eq!(app.plugin_selected, 0);

            let total = app.total_plugin_items();
            assert!(total > 0);

            // Navigate down through all items
            for _ in 0..total + 5 {
                app.next();
            }
            assert_eq!(app.plugin_selected, total - 1);

            // Test go to top
            app.go_to_top();
            assert_eq!(app.plugin_selected, 0);

            // Test go to bottom
            app.go_to_bottom();
            assert_eq!(app.plugin_selected, total - 1);

            // Test previous
            app.previous();
            assert_eq!(app.plugin_selected, total - 2);

            // Toggle item 0 (first plugin)
            app.plugin_selected = 0;
            let prev_status = app.plugins[0].enabled;
            app.toggle_selected_plugin();
            assert_eq!(app.plugins[0].enabled, !prev_status);

            // Remove plugin
            app.remove_selected_plugin();
            assert!(app.plugins[0].is_removed);
            assert!(!app.plugins[0].enabled);

            // Restore plugin
            app.restore_selected_plugin();
            assert!(!app.plugins[0].is_removed);
            assert!(app.plugins[0].enabled);

            // Edit path item
            let n_plugins = app.plugins.len();
            app.plugin_selected = n_plugins; // First path
            app.start_editing_selected_path();
            assert_eq!(app.editing_path_index, Some(0));
            app.editing_path_input = "/custom/music/path".to_string();
            app.confirm_editing_path();
            assert_eq!(app.config_paths[0].path, "/custom/music/path");
            assert_eq!(app.editing_path_index, None);

            // Plugin store option (last item)
            app.plugin_selected = total - 1;
            app.toggle_selected_plugin();
            assert!(app.show_plugin_store);
            assert_eq!(app.store_selected, 0);
        }
    }

    #[test]
    fn test_browser_go_parent_and_import() {
        let temp_base = std::env::temp_dir().join("lyra_test_b_import");
        let album_dir = temp_base.join("AlbumA");
        let _ = std::fs::create_dir_all(&album_dir);
        let song1 = album_dir.join("Song1.mp3");
        let _ = std::fs::write(&song1, b"mp3");

        if let Ok(mut app) = App::new(&album_dir) {
            app.view_mode = ViewMode::Browser;
            app.playlist.set_songs(vec![]);
            assert_eq!(app.playlist.songs.len(), 0);

            // Import folder
            app.import_browser_folder_into_playlist();
            assert_eq!(app.playlist.songs.len(), 1);

            // Test browser_go_parent
            app.browser_go_parent();
            assert_eq!(app.browser.current_dir, std::fs::canonicalize(&temp_base).unwrap());
            assert!(app.toast_text().is_some_and(|t| t.contains("Up to:")));
        }

        let _ = std::fs::remove_dir_all(&temp_base);
    }

    #[test]
    fn test_custom_plugin_uninstall_and_no_auto_restore() {
        if let Ok(mut app) = App::new(std::path::Path::new(".")) {
            // Open store and install first plugin. "Coming Soon" entries
            // are not installable, so stage a fake installable one backed
            // by a real file:// single-file source into a temp data root.
            let plug_tmp = std::env::temp_dir().join(format!("lyra_store_{}", std::process::id()));
            let _ = std::fs::create_dir_all(&plug_tmp);
            std::fs::write(plug_tmp.join("fake_plugin.py"), "def run(payload):\n    return {'status':'ok'}\n").unwrap();
            app.plugin_data_root = Some(plug_tmp.join("data"));
            app.downloadable_plugins.insert(
                0,
                crate::plugin::DownloadablePlugin {
                    id: "test-fake-plugin".into(),
                    name: "Fake Test Plugin".into(),
                    version: "v0.1.0".into(),
                    author: "tests".into(),
                    description: "Staged by unit test".into(),
                    is_installed: false,
                    url: format!("file://{}", plug_tmp.join("fake_plugin.py").display()),
                    sha256: String::new(),
                    min_version: String::new(),
                },
            );
            app.show_plugin_store = true;
            app.store_selected = 0;
            let installed_name = app.downloadable_plugins[0].name.clone();
            app.install_store_plugin();
            // Real installs run on a background thread: pump events until done.
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
            while !app.downloadable_plugins[0].is_installed
                && std::time::Instant::now() < deadline
            {
                app.check_plugin_downloads();
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            app.check_plugin_downloads();

            assert!(app.downloadable_plugins[0].is_installed);
            let custom_idx = app.plugins.iter().position(|p| p.name == installed_name);
            assert!(custom_idx.is_some());
            let custom_idx = custom_idx.unwrap();
            assert!(!app.plugins[custom_idx].is_builtin);

            // Select this newly installed custom plugin
            app.plugin_selected = custom_idx;
            // Remove it (permanently delete)
            app.remove_selected_plugin();

            // Check that it's completely uninstalled from plugins and reset in store
            assert!(!app.downloadable_plugins[0].is_installed);
            assert!(app.plugins.iter().all(|p| p.name != installed_name));

            // Test that right arrow does NOT restore a removed plugin
            // Select a builtin plugin and remove it
            app.plugin_selected = 0;
            app.remove_selected_plugin();
            assert!(app.plugins[0].is_removed);

            // Call adjust_selected_plugin(true) (Right arrow)
            app.adjust_selected_plugin(true);
            // Must stay removed!
            assert!(app.plugins[0].is_removed);

            // Call toggle_selected_plugin (Space / Enter)
            app.toggle_selected_plugin();
            // Must stay removed and give toast
            assert!(app.plugins[0].is_removed);
            assert!(app.toast_text().is_some_and(|t| t.contains("Press 'r' to restore")));

            // Only 'r' restores it
            app.restore_selected_plugin();
            assert!(!app.plugins[0].is_removed);

            // Uninstall wiped the downloaded files too.
            assert!(!plug_tmp.join("data").join("test-fake-plugin").exists());
            let _ = std::fs::remove_dir_all(&plug_tmp);
        }
    }

    #[test]
    fn test_coming_soon_store_item_not_installable() {        if let Ok(mut app) = App::new(std::path::Path::new(".")) {
            app.downloadable_plugins.insert(
                0,
                crate::plugin::DownloadablePlugin {
                    id: "soon-plugin".into(),
                    name: "Soon Plugin".into(),
                    version: "Coming Soon".into(),
                    author: "tests".into(),
                    description: "Not yet available".into(),
                    is_installed: false,
                    url: String::new(),
                    sha256: String::new(),
                    min_version: String::new(),
                },
            );
            app.show_plugin_store = true;
            app.store_selected = 0;
            let n_before = app.plugins.len();
            app.install_store_plugin();
            assert!(!app.downloadable_plugins[0].is_installed);
            assert_eq!(app.plugins.len(), n_before);
            assert!(app.toast_text().is_some_and(|t| t.contains("coming soon")));
        }
    }

    #[test]
    fn test_plugin_disable_takes_effect_without_recompile() {
        if let Ok(mut app) = App::new(std::path::Path::new(".")) {
            assert!(app.plugin_enabled("notify"));
            // Disable notify like a user would in the Plugins view.
            let idx = app.plugins.iter().position(|p| p.id == "notify").unwrap();
            app.plugin_selected = idx;
            app.toggle_selected_plugin();
            assert!(!app.plugin_enabled("notify"));
            assert!(!app.notifications_enabled);
            // Lyrics layout follows its plugin.
            let lidx = app.plugins.iter().position(|p| p.id == "lyrics").unwrap();
            app.plugin_selected = lidx;
            if app.plugin_enabled("lyrics") {
                app.toggle_selected_plugin();
            }
            assert!(!app.plugin_enabled("lyrics"));
            assert!(!app.show_lyrics);
            app.toggle_lyrics();
            assert!(!app.show_lyrics, "manual toggle must not revive a disabled plugin");
        }
    }

    #[cfg(feature = "download")]
    #[test]
    fn test_search_limit_settings_row() {
        if let Ok(mut app) = App::new(std::path::Path::new(".")) {
            let row = app.search_limit_row_idx();
            assert!(app.is_search_limit_row() == (app.plugin_selected == row));
            app.plugin_selected = row;
            assert!(app.is_search_limit_row());

            // Store row shifted by one and still opens the store.
            app.plugin_selected = app.total_plugin_items() - 1;
            app.toggle_selected_plugin();
            assert!(app.show_plugin_store);

            // Step presets with adjust (Left/Right).
            app.plugin_selected = row;
            app.download.search_limit = 15;
            app.adjust_selected_plugin(true);
            assert_eq!(app.download.search_limit, 25);
            app.adjust_selected_plugin(false);
            assert_eq!(app.download.search_limit, 15);
            // Enter/Space also steps forward.
            app.toggle_selected_plugin();
            assert_eq!(app.download.search_limit, 25);
            assert!(app.toast_text().is_some_and(|t| t.contains("Search result limit")));
        }
    }
}


