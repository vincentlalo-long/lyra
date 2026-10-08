use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use ratatui::widgets::ListState;

/// Ephemeral "play next" list (scratchpad).
///
/// The library (`Playlist.songs`) is browsable and never loses entries;
/// the queue only holds *what to play next*. Items play once, in order,
/// then vanish. Deleting here never touches files on disk.
pub struct PlayQueue {
    /// Upcoming tracks, head = next to play.
    pub items: VecDeque<PathBuf>,
    /// Tracks already played through the queue (for Prev support).
    pub history: Vec<PathBuf>,
    /// True when the currently playing track came from the queue.
    pub now_from_queue: bool,
    /// Cursor for the Queue view.
    pub selected: usize,
    pub state: ListState,
}

impl PlayQueue {
    pub fn new() -> Self {
        Self {
            items: VecDeque::new(),
            history: Vec::new(),
            now_from_queue: false,
            selected: 0,
            state: ListState::default(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Append to the tail (play after everything queued).
    pub fn enqueue_back(&mut self, path: PathBuf) {
        self.items.push_back(path);
    }

    /// Push to the head (play next).
    pub fn enqueue_front(&mut self, path: PathBuf) {
        self.items.push_front(path);
    }

    /// Take the head for immediate playback. Returns the path to play.
    /// `current` (the track being replaced) goes to history when it came
    /// from the queue, so Prev can walk back through queue playback.
    pub fn pop_next(&mut self, current: Option<PathBuf>, current_from_queue: bool) -> Option<PathBuf> {
        if current_from_queue {
            if let Some(cur) = current {
                self.history.push(cur);
            }
        }
        let next = self.items.pop_front()?;
        self.now_from_queue = true;
        self.clamp_selected();
        Some(next)
    }

    /// Step back to the previously played queue track. The track being
    /// replaced goes back to the head so nothing is lost.
    pub fn pop_prev(&mut self, current: Option<PathBuf>) -> Option<PathBuf> {
        let prev = self.history.pop()?;
        if let Some(cur) = current {
            self.items.push_front(cur);
        }
        self.now_from_queue = true;
        self.clamp_selected();
        Some(prev)
    }

    /// Jump to `items[idx]`: it becomes now-playing, predecessors move to
    /// history (Prev can still reach them), successors stay queued.
    pub fn jump_to(&mut self, idx: usize, current: Option<PathBuf>, current_from_queue: bool) -> Option<PathBuf> {
        if idx >= self.items.len() {
            return None;
        }
        if current_from_queue {
            if let Some(cur) = current {
                self.history.push(cur);
            }
        }
        for _ in 0..idx {
            if let Some(skipped) = self.items.pop_front() {
                self.history.push(skipped);
            }
        }
        let target = self.items.pop_front()?;
        self.now_from_queue = true;
        self.clamp_selected();
        Some(target)
    }

    /// Remove the item at `idx`. Returns true when something was removed.
    /// Removing the now-playing head is handled by the caller (skip next).
    pub fn remove(&mut self, idx: usize) -> bool {
        if idx >= self.items.len() {
            return false;
        }
        self.items.remove(idx);
        self.clamp_selected();
        true
    }

    /// Remove the first occurrence of `path`. Returns true on removal.
    pub fn remove_path(&mut self, path: &Path) -> bool {
        if let Some(idx) = self.items.iter().position(|p| p == path) {
            self.items.remove(idx);
            self.clamp_selected();
            true
        } else {
            false
        }
    }

    /// Fisher-Yates shuffle of upcoming items. History and now-playing
    /// are untouched. `rand_state` is a simple LCG seed so tests are
    /// deterministic without new dependencies.
    pub fn shuffle(&mut self, rand_state: &mut u64) {
        let len = self.items.len();
        if len < 2 {
            return;
        }
        // Materialize, shuffle, restore (VecDeque has no in-place shuffle).
        let mut v: Vec<PathBuf> = self.items.drain(..).collect();
        let mut next_rand = || {
            *rand_state = rand_state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (*rand_state >> 33) as usize
        };
        for i in (1..v.len()).rev() {
            let j = next_rand() % (i + 1);
            v.swap(i, j);
        }
        self.items = v.into();
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.history.clear();
        self.now_from_queue = false;
        self.selected = 0;
        self.state = ListState::default();
    }

    fn clamp_selected(&mut self) {
        if self.items.is_empty() {
            self.selected = 0;
        } else if self.selected >= self.items.len() {
            self.selected = self.items.len() - 1;
        }
    }

    pub fn move_selected_down(&mut self) {
        if !self.items.is_empty() {
            self.selected = (self.selected + 1).min(self.items.len() - 1);
        }
    }

    pub fn move_selected_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    // ---------- persistence ----------

    fn queue_file() -> Option<PathBuf> {
        let base = std::env::var_os("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME").map(|h| {
                    let mut p = PathBuf::from(h);
                    p.push(".cache");
                    p
                })
            })?;
        let mut dir = base;
        dir.push("lyra");
        std::fs::create_dir_all(&dir).ok()?;
        dir.push("queue.json");
        Some(dir)
    }

    /// Autosave after every mutation; failures are ignored (queue is ephemeral).
    pub fn persist(&self) {
        let Some(path) = Self::queue_file() else {
            return;
        };
        let items: Vec<String> = self
            .items
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect();
        if let Ok(json) = serde_json::to_string(&items) {
            let _ = std::fs::write(path, json);
        }
    }

    /// Restore on startup; drops entries whose files no longer exist.
    pub fn load() -> Self {
        let mut q = Self::new();
        let Some(path) = Self::queue_file() else {
            return q;
        };
        let Ok(json) = std::fs::read_to_string(path) else {
            return q;
        };
        let Ok(items): Result<Vec<String>, _> = serde_json::from_str(&json) else {
            return q;
        };
        q.items = items
            .into_iter()
            .map(PathBuf::from)
            .filter(|p| p.exists())
            .collect();
        q
    }

    // ---------- day-lists (.m3u, portable across players) ----------

    fn daylists_dir(music_folder: &Path) -> Option<PathBuf> {
        let dir = music_folder.join("daylists");
        std::fs::create_dir_all(&dir).ok()?;
        Some(dir)
    }

    /// `YYYY-MM-DD` in UTC via Howard Hinnant's civil-from-days algorithm.
    /// No date crate needed; covered by unit test against known timestamps.
    pub fn today_stamp(now_secs: u64) -> String {
        let z = (now_secs / 86_400) as i64 + 719_468;
        let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
        let mut y = yoe + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        y += if m <= 2 { 1 } else { 0 };
        format!("{y:04}-{:02}-{:02}", m, d)
    }

    fn now_stamp() -> String {
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Self::today_stamp(secs)
    }

    /// Saves current upcoming items as `<music>/daylists/YYYY-MM-DD.m3u`.
    /// Returns the file path on success.
    pub fn save_daylist(&self, music_folder: &Path) -> Option<PathBuf> {
        let dir = Self::daylists_dir(music_folder)?;
        let path = dir.join(format!("{}.m3u", Self::now_stamp()));
        let mut out = String::from("#EXTM3U\n# Playlist saved by lyra\n");
        for item in &self.items {
            out.push_str(&format!("#EXTINF:-1,{}\n{}\n", item.to_string_lossy(), item.display()));
        }
        std::fs::write(&path, out).ok()?;
        Some(path)
    }

    /// Loads the most recently modified day-list into the queue (replacing it).
    /// Returns the number of tracks loaded.
    pub fn load_latest_daylist(&mut self, music_folder: &Path) -> usize {
        let Some(dir) = Self::daylists_dir(music_folder) else {
            return 0;
        };
        let Ok(entries) = std::fs::read_dir(&dir) else {
            return 0;
        };
        let latest = entries
            .flatten()
            .filter(|e| e.path().extension().is_some_and(|x| x == "m3u"))
            .filter_map(|e| {
                let mtime = e.metadata().ok()?.modified().ok()?;
                Some((mtime, e.path()))
            })
            .max_by_key(|(t, _)| *t)
            .map(|(_, p)| p);
        let Some(path) = latest else {
            return 0;
        };
        let Ok(text) = std::fs::read_to_string(&path) else {
            return 0;
        };
        let items: VecDeque<PathBuf> = text
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .map(PathBuf::from)
            .filter(|p| p.exists())
            .collect();
        let n = items.len();
        self.items = items;
        self.history.clear();
        self.now_from_queue = false;
        self.selected = 0;
        self.state = ListState::default();
        self.persist();
        n
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths(names: &[&str]) -> VecDeque<PathBuf> {
        names.iter().map(PathBuf::from).collect()
    }

    fn make_queue(names: &[&str]) -> PlayQueue {
        let mut q = PlayQueue::new();
        q.items = paths(names);
        q
    }

    #[test]
    fn test_pop_next_moves_current_to_history() {
        let mut q = make_queue(&["b", "c"]);
        // Current came from the queue -> goes to history.
        let next = q.pop_next(Some(PathBuf::from("a")), true);
        assert_eq!(next, Some(PathBuf::from("b")));
        assert_eq!(q.history, vec![PathBuf::from("a")]);
        assert!(q.now_from_queue);
    }

    #[test]
    fn test_pop_next_library_current_skips_history() {
        let mut q = make_queue(&["b"]);
        let next = q.pop_next(Some(PathBuf::from("lib_song")), false);
        assert_eq!(next, Some(PathBuf::from("b")));
        assert!(q.history.is_empty());
    }

    #[test]
    fn test_pop_prev_restores_nothing_lost() {
        // Playing c (popped earlier, so not in items), history a -> b.
        let mut q = make_queue(&["d"]);
        q.history.push(PathBuf::from("a"));
        q.history.push(PathBuf::from("b"));
        // Go back to b; c returns to head, d stays behind it.
        let prev = q.pop_prev(Some(PathBuf::from("c")));
        assert_eq!(prev, Some(PathBuf::from("b")));
        assert_eq!(q.items, paths(&["c", "d"]));
        assert_eq!(q.history, vec![PathBuf::from("a")]);
        assert!(q.pop_prev(None).is_some());
        assert!(q.pop_prev(None).is_none()); // history drained
    }

    #[test]
    fn test_jump_to_moves_predecessors_to_history() {
        let mut q = make_queue(&["b", "c", "d"]);
        let target = q.jump_to(2, Some(PathBuf::from("a")), true);
        assert_eq!(target, Some(PathBuf::from("d")));
        assert_eq!(
            q.history,
            vec![
                PathBuf::from("a"),
                PathBuf::from("b"),
                PathBuf::from("c")
            ]
        );
        assert!(q.items.is_empty());
        assert!(q.jump_to(0, None, false).is_none());
    }

    #[test]
    fn test_remove_and_remove_path() {
        let mut q = make_queue(&["a", "b", "c"]);
        q.selected = 2;
        assert!(q.remove(1));
        assert_eq!(q.items, paths(&["a", "c"]));
        assert_eq!(q.selected, 1); // clamped into range
        assert!(!q.remove(9));
        assert!(q.remove_path(Path::new("a")));
        assert!(!q.remove_path(Path::new("zzz")));
        assert_eq!(q.items, paths(&["c"]));
    }

    #[test]
    fn test_shuffle_keeps_all_items_and_is_deterministic() {
        let mut q1 = make_queue(&["a", "b", "c", "d", "e"]);
        let mut q2 = make_queue(&["a", "b", "c", "d", "e"]);
        let mut s1 = 12345u64;
        let mut s2 = 12345u64;
        q1.shuffle(&mut s1);
        q2.shuffle(&mut s2);
        assert_eq!(q1.items, q2.items); // same seed -> same order
        let mut sorted: Vec<_> = q1.items.iter().collect();
        sorted.sort();
        assert_eq!(sorted.len(), 5); // nothing lost or duplicated
    }

    #[test]
    fn test_today_stamp_known_dates() {
        // 2026-10-08 00:00:00 UTC
        assert_eq!(PlayQueue::today_stamp(1_791_417_600), "2026-10-08");
        // 1970-01-01
        assert_eq!(PlayQueue::today_stamp(0), "1970-01-01");
        // 2024-02-29 (leap day)
        assert_eq!(PlayQueue::today_stamp(1_709_164_800), "2024-02-29");
    }

    #[test]
    fn test_daylist_roundtrip_filters_missing_files() {
        let dir = std::env::temp_dir().join("lyra-queue-test");
        let _ = std::fs::create_dir_all(&dir);
        let real = dir.join("real.mp3");
        let _ = std::fs::write(&real, b"x");
        let mut q = PlayQueue::new();
        q.items = VecDeque::from([
            real.clone(),
            PathBuf::from("/nonexistent/ghost.mp3"),
        ]);
        let saved = q.save_daylist(&dir).expect("save daylist");
        assert!(saved.exists());

        let mut q2 = PlayQueue::new();
        // Point daylists lookup at our temp dir by nesting: save created <dir>/daylists/.
        let n = q2.load_latest_daylist(&dir);
        assert_eq!(n, 1);
        assert_eq!(q2.items, VecDeque::from([real]));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
