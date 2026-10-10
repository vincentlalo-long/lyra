use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use ratatui::widgets::ListState;

/// How many played tracks Prev can walk back through. Beyond this,
/// the oldest entries are dropped on push so memory stays flat.
pub const MAX_HISTORY: usize = 200;

/// Ephemeral "play next" list (scratchpad).
///
/// The library (`Playlist.songs`) is browsable and never loses entries;
/// the queue only holds *what to play next*. Items play once, in order,
/// then vanish. Deleting here never touches files on disk.
pub struct PlayQueue {
    /// Upcoming tracks, head = next to play.
    pub items: VecDeque<PathBuf>,
    /// Tracks already played through the queue (for Prev support).
    /// Capped at [`MAX_HISTORY`] — Prev walks back within the window,
    /// so an all-day/looped session can't grow this Vec without bound.
    pub history: Vec<PathBuf>,
    /// True when the currently playing track came from the queue.
    pub now_from_queue: bool,
    /// Queue-loop (`L`): when items drain, refill from the snapshot instead
    /// of falling through to the library.
    pub loop_enabled: bool,
    /// What one loop round replays. Synced on every mutation while looping
    /// so mid-loop edits join future rounds; cleared with the queue.
    loop_snapshot: Vec<PathBuf>,
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
            loop_enabled: false,
            loop_snapshot: Vec::new(),
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
        self.sync_loop_snapshot();
    }

    /// Push to the head (play next).
    pub fn enqueue_front(&mut self, path: PathBuf) {
        self.items.push_front(path);
        self.sync_loop_snapshot();
    }

    /// Take the head for immediate playback. Returns the path to play.
    /// `current` (the track being replaced) goes to history when it came
    /// from the queue, so Prev can walk back through queue playback.
    /// When loop mode is on and items drain, one round is refilled from the
    /// snapshot instead of returning `None`.
    pub fn pop_next(&mut self, current: Option<PathBuf>, current_from_queue: bool) -> Option<PathBuf> {
        if current_from_queue {
            if let Some(cur) = current {
                self.push_history(cur);
            }
        }
        if self.items.is_empty() && self.loop_enabled && !self.loop_snapshot.is_empty() {
            self.items = self.loop_snapshot.clone().into();
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
                self.push_history(cur);
            }
        }
        for _ in 0..idx {
            if let Some(skipped) = self.items.pop_front() {
                self.push_history(skipped);
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
        self.sync_loop_snapshot();
        self.clamp_selected();
        true
    }

    /// Remove the first occurrence of `path`. Returns true on removal.
    pub fn remove_path(&mut self, path: &Path) -> bool {
        if let Some(idx) = self.items.iter().position(|p| p == path) {
            self.items.remove(idx);
            self.sync_loop_snapshot();
            self.clamp_selected();
            true
        } else {
            false
        }
    }

    /// Replace the item at `idx`, keeping order. Returns false when `idx`
    /// is out of range (no-op).
    pub fn replace(&mut self, idx: usize, path: PathBuf) -> bool {
        if idx >= self.items.len() {
            return false;
        }
        self.items[idx] = path;
        self.sync_loop_snapshot();
        true
    }

    /// Toggle queue-loop on/off. Enabling snapshots the current upcoming
    /// items as one loop round; disabling keeps items but stops refilling.
    pub fn set_loop(&mut self, enabled: bool) {
        self.loop_enabled = enabled;
        if enabled {
            self.sync_loop_snapshot();
        }
    }

    /// One loop round replays the queue as last edited (consumption via
    /// pop_next/pop_prev/jump_to never shrinks it).
    fn sync_loop_snapshot(&mut self) {
        self.loop_snapshot = self.items.iter().cloned().collect();
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
        self.sync_loop_snapshot();
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.history.clear();
        self.loop_snapshot.clear();
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

    /// Pushes to history, dropping the oldest entries past [`MAX_HISTORY`]
    /// so a long/looped session keeps flat memory.
    fn push_history(&mut self, path: PathBuf) {
        self.history.push(path);
        let overflow = self.history.len().saturating_sub(MAX_HISTORY);
        if overflow > 0 {
            self.history.drain(..overflow);
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
        let doc = serde_json::json!({ "items": items, "loop_enabled": self.loop_enabled });
        let _ = std::fs::write(path, doc.to_string());
    }

    /// Restore on startup; drops entries whose files no longer exist.
    /// Reads the current `{items, loop_enabled}` object, falling back to the
    /// legacy bare array (loop off).
    pub fn load() -> Self {
        let mut q = Self::new();
        let Some(path) = Self::queue_file() else {
            return q;
        };
        let Ok(json) = std::fs::read_to_string(path) else {
            return q;
        };
        let Ok(doc) = serde_json::from_str::<serde_json::Value>(&json) else {
            return q;
        };
        // Legacy files were a bare array (loop off); current files are an object.
        let items: Vec<String> = if doc.is_array() {
            serde_json::from_value(doc).unwrap_or_default()
        } else {
            q.loop_enabled = doc
                .get("loop_enabled")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            doc.get("items")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default()
        };
        q.items = items
            .into_iter()
            .map(PathBuf::from)
            .filter(|p| p.exists())
            .collect();
        q.sync_loop_snapshot();
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
        self.sync_loop_snapshot();
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
    fn test_queue_loop_refills_on_drain() {        let mut q = make_queue(&["a", "b"]);
        q.set_loop(true);

        assert_eq!(q.pop_next(None, false), Some(PathBuf::from("a")));
        assert_eq!(q.pop_next(None, false), Some(PathBuf::from("b")));
        // Drained + loop on -> one round refilled, playback continues.
        assert_eq!(q.pop_next(None, false), Some(PathBuf::from("a")));

        // Disabling mid-round: remaining items play out, then a real drain.
        q.set_loop(false);
        assert_eq!(q.pop_next(None, false), Some(PathBuf::from("b")));
        assert_eq!(q.pop_next(None, false), None);
    }

    #[test]
    fn test_history_capped_for_long_looped_sessions() {
        let mut q = make_queue(&["a", "b"]);
        q.set_loop(true);
        // Simulate 3 full days of looping: history must stay flat.
        for i in 0..1000 {
            let cur = Some(PathBuf::from(format!("track-{i}")));
            let _ = q.pop_next(cur, true);
        }
        assert_eq!(q.history.len(), MAX_HISTORY);
        // Newest entries are kept (Prev still works in the window).
        assert_eq!(q.history.last(), Some(&PathBuf::from("track-999")));
        // jump_to bulk-skips respect the cap too.
        let mut q2 = make_queue(&["x", "y", "z"]);
        for i in 0..(MAX_HISTORY + 50) {
            q2.enqueue_back(PathBuf::from(format!("n-{i}")));
        }
        let _ = q2.jump_to(MAX_HISTORY + 49, Some(PathBuf::from("cur")), true);
        assert!(q2.history.len() <= MAX_HISTORY);
    }

    #[test]
    fn test_queue_loop_snapshot_tracks_edits_not_consumption() {
        let mut q = make_queue(&["a", "b"]);
        q.set_loop(true);
        // Consumption must not shrink the loop round...
        assert_eq!(q.pop_next(None, false), Some(PathBuf::from("a")));
        // ...but edits join future rounds. Round is now [c]: `a` was
        // consumed before the edits, consumption never re-enters the round.
        q.enqueue_back(PathBuf::from("c"));
        q.remove_path(Path::new("b"));
        assert_eq!(q.pop_next(None, false), Some(PathBuf::from("c")));
        // Refilled round replays exactly [c].
        assert_eq!(q.pop_next(None, false), Some(PathBuf::from("c")));
        assert_eq!(q.pop_next(None, false), Some(PathBuf::from("c")));
    }

    #[test]
    fn test_replace_keeps_order_and_syncs_loop() {
        let mut q = make_queue(&["a", "b", "c"]);
        q.set_loop(true);
        assert!(q.replace(1, PathBuf::from("B2")));
        assert_eq!(q.items, paths(&["a", "B2", "c"]));
        assert!(!q.replace(9, PathBuf::from("x")));
        // Replacement is part of future loop rounds.
        assert_eq!(q.pop_next(None, false), Some(PathBuf::from("a")));
        assert_eq!(q.pop_next(None, false), Some(PathBuf::from("B2")));
        assert_eq!(q.pop_next(None, false), Some(PathBuf::from("c")));
        assert_eq!(q.pop_next(None, false), Some(PathBuf::from("a")));
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
