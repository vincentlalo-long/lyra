use std::path::PathBuf;

use super::{App, ViewMode, short_name};

impl App {

    /// Path under the cursor in file-list views (for enqueue / queue-delete).
    /// Queue view resolves to the queued item itself; Download view has none.
    fn selected_file(&self) -> Option<PathBuf> {
        match self.view_mode {
            ViewMode::Playlist => self.playlist.current_selected_song(),
            ViewMode::Queue => self.queue.items.get(self.queue.selected).cloned(),
            ViewMode::Browser => self.browser.entries.get(self.browser.selected).cloned().filter(|p| {
                p.is_file() && p.extension().is_some_and(|e| e.eq_ignore_ascii_case("mp3"))
            }),
            #[cfg(feature = "download")]
            ViewMode::Download => None,
            ViewMode::Plugins => None,
        }
    }

    /// `a`: append cursor song (or whole folder / m3u playlist in Browser) to the queue tail.
    pub fn enqueue_selected_back(&mut self) {
        if self.view_mode == ViewMode::Browser {
            if let Some(entry) = self.browser.entries.get(self.browser.selected).cloned() {
                if entry.is_dir() {
                    let songs = crate::playlist::find_mp3s_in_dir(&entry, false);
                    let n = songs.len();
                    if n > 0 {
                        for s in songs {
                            self.queue.enqueue_back(s);
                        }
                        self.queue.persist();
                        let folder_name = short_name(&entry);
                        self.set_toast(format!("Queued {n} songs from {folder_name}/"));
                    } else {
                        self.set_toast("Folder has no MP3 files".to_string());
                    }
                    return;
                } else if entry.extension().is_some_and(|e| e.eq_ignore_ascii_case("m3u") || e.eq_ignore_ascii_case("m3u8")) {
                    let songs = crate::playlist::load_m3u(&entry);
                    let n = songs.len();
                    if n > 0 {
                        for s in songs {
                            self.queue.enqueue_back(s);
                        }
                        self.queue.persist();
                        let name = short_name(&entry);
                        self.set_toast(format!("Queued {n} songs from {name}"));
                    } else {
                        self.set_toast("Playlist is empty or tracks missing".to_string());
                    }
                    return;
                }
            }
        }

        if let Some(path) = self.selected_file() {
            let name = short_name(&path);
            self.queue.enqueue_back(path);
            self.queue.persist();
            self.set_toast(format!("Queued #{}: {name}", self.queue.len()));
        }
    }

    /// `A`: play cursor song (or whole folder / m3u playlist in Browser) next (queue head).
    pub fn enqueue_selected_front(&mut self) {
        if self.view_mode == ViewMode::Browser {
            if let Some(entry) = self.browser.entries.get(self.browser.selected).cloned() {
                if entry.is_dir() {
                    let songs = crate::playlist::find_mp3s_in_dir(&entry, false);
                    let n = songs.len();
                    if n > 0 {
                        for s in songs.into_iter().rev() {
                            self.queue.enqueue_front(s);
                        }
                        self.queue.persist();
                        let folder_name = short_name(&entry);
                        self.set_toast(format!("Up next: {n} songs from {folder_name}/"));
                    } else {
                        self.set_toast("Folder has no MP3 files".to_string());
                    }
                    return;
                } else if entry.extension().is_some_and(|e| e.eq_ignore_ascii_case("m3u") || e.eq_ignore_ascii_case("m3u8")) {
                    let songs = crate::playlist::load_m3u(&entry);
                    let n = songs.len();
                    if n > 0 {
                        for s in songs.into_iter().rev() {
                            self.queue.enqueue_front(s);
                        }
                        self.queue.persist();
                        let name = short_name(&entry);
                        self.set_toast(format!("Up next: {n} songs from {name}"));
                    } else {
                        self.set_toast("Playlist is empty or tracks missing".to_string());
                    }
                    return;
                }
            }
        }

        if let Some(path) = self.selected_file() {
            let name = short_name(&path);
            self.queue.enqueue_front(path);
            self.queue.persist();
            self.set_toast(format!("Up next: {name}"));
        }
    }

    /// `e` on a queue row: arm a replacement. Jumps to the Browser so the
    /// user can pick another mp3; Enter confirms, Esc cancels. No-op on an
    /// empty queue.
    pub fn begin_replace(&mut self) {
        if self.view_mode != ViewMode::Queue || self.queue.is_empty() {
            return;
        }
        self.pending_replace = Some(self.queue.selected);
        self.view_mode = ViewMode::Browser;
        self.set_toast("Pick replacement (Enter) / Esc cancel".to_string());
    }

    /// Completes a pending replace with the Browser cursor song.
    /// Returns true when a replacement happened (caller returns to Queue view).
    pub fn confirm_replace(&mut self) -> bool {
        let Some(idx) = self.pending_replace else {
            return false;
        };
        let pick = self.browser.entries.get(self.browser.selected).cloned().filter(|p| {
            p.is_file() && p.extension().is_some_and(|e| e.eq_ignore_ascii_case("mp3"))
        });
        let Some(path) = pick else {
            self.set_toast("Pick an mp3 file".to_string());
            return false;
        };
        if self.queue.replace(idx, path.clone()) {
            self.queue.persist();
            self.pending_replace = None;
            self.view_mode = ViewMode::Queue;
            self.set_toast(format!("Replaced with: {}", short_name(&path)));
            true
        } else {
            // Queue changed under our feet (e.g. drained meanwhile).
            self.pending_replace = None;
            self.view_mode = ViewMode::Queue;
            self.set_toast("Queue changed, replace cancelled".to_string());
            false
        }
    }

    /// Aborts a pending replace, back to the Queue view.
    pub fn cancel_replace(&mut self) {
        if self.pending_replace.take().is_some() {
            self.view_mode = ViewMode::Queue;
            self.set_toast("Replace cancelled".to_string());
        }
    }

    /// `Enter` on a queue row: play it now. Predecessors move to history
    /// (Prev still reaches them), successors stay queued.
    pub fn play_queue_selected(&mut self) {
        if self.queue.is_empty() {
            return;
        }
        let idx = self.queue.selected;
        let current = self.current_playing_path.clone();
        let was_from_queue = self.queue.now_from_queue;
        if let Some(path) = self.queue.jump_to(idx, current, was_from_queue) {
            self.queue.persist();
            self.play_track(&path);
            self.queue.now_from_queue = true;
        }
    }

    /// `d`: delete from the queue, never from disk.
    /// - Queue view: remove the cursor row.
    /// - File views: if the cursor song is the currently playing queue track,
    ///   skip it (advance); otherwise unqueue its first occurrence (no-op
    ///   when absent).
    pub fn delete_queue_selected(&mut self) {
        if self.view_mode == ViewMode::Queue {
            if let Some(removed) = self.queue.items.get(self.queue.selected).cloned() {
                let name = short_name(&removed);
                self.queue.remove(self.queue.selected);
                self.queue.persist();
                self.set_toast(format!("Unqueued: {name}"));
            }
            return;
        }
        let Some(path) = self.selected_file() else {
            return;
        };
        let is_now_playing = self.current_playing_path.as_ref() == Some(&path);
        if is_now_playing && self.queue.now_from_queue {
            // Deleting what is playing = skip it.
            let name = short_name(&path);
            self.queue.now_from_queue = false;
            self.play_next_track();
            self.set_toast(format!("Skipped: {name}"));
        } else if self.queue.remove_path(&path) {
            self.queue.persist();
            self.set_toast(format!("Unqueued: {}", short_name(&path)));
        } else {
            self.set_toast("Not in queue".to_string());
        }
    }

    /// `c`: drop the whole scratchpad, back to plain library playback.
    pub fn clear_queue(&mut self) {
        let n = self.queue.len();
        self.queue.clear();
        self.queue.persist();
        self.set_toast(if n == 0 {
            "Queue already empty".to_string()
        } else {
            format!("Queue cleared ({n} removed)")
        });
    }

    /// `z`: shuffle upcoming queue items (now-playing untouched).
    pub fn shuffle_queue(&mut self) {
        let n = self.queue.len();
        if n < 2 {
            self.set_toast("Nothing to shuffle".to_string());
            return;
        }
        let mut seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9E3779B97F4A7C15);
        if seed == 0 {
            seed = 0x9E3779B97F4A7C15;
        }
        self.queue.shuffle(&mut seed);
        self.queue.persist();
        self.set_toast(format!("Queue shuffled ({n} songs)"));
    }

    /// `w`: save upcoming queue as today's portable day-list.
    /// Returns the saved file for potential UI feedback.
    pub fn save_daylist(&mut self) -> Option<PathBuf> {
        if self.queue.is_empty() {
            self.set_toast("Queue empty, nothing to save".to_string());
            return None;
        }
        let saved = self.queue.save_daylist(&self.music_folder.clone());
        self.browser.refresh();
        match &saved {
            Some(p) => self.set_toast(format!(
                "Day-list saved: {}",
                p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
            )),
            None => self.set_toast("Save failed".to_string()),
        }
        saved
    }

    /// `o`: replace the queue with the most recent day-list.
    /// Returns the number of tracks loaded.
    pub fn load_daylist(&mut self) -> usize {
        let folder = self.music_folder.clone();
        let n = self.queue.load_latest_daylist(&folder);
        self.set_toast(if n == 0 {
            "No day-list found".to_string()
        } else {
            format!("Loaded {n} songs from day-list")
        });
        n
    }
}
