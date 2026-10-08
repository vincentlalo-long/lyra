#![allow(dead_code)]
use std::{
    io::{BufRead, BufReader, Read},
    path::PathBuf,
    process::{Command, Stdio},
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc, Mutex,
    },
    thread,
    time::Instant,
};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct SearchItem {
    pub id: String,
    pub url: String,
    pub title: String,
    pub artist: String,
    pub uploader: String,
    pub duration: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CoverCandidate {
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub artist: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub album: String,
    #[serde(default)]
    pub year: String,
    #[serde(default)]
    pub cover_url: String,
    #[serde(default)]
    pub score: f32,
}

#[derive(Debug, Clone)]
pub struct DownloadRequest {
    pub url: String,
    pub output_dir: PathBuf,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub cover_mode: String,
    pub cover_url: Option<String>,
    pub no_lyrics: bool,
    pub no_auto_lyrics: bool,
}

#[derive(Debug, Clone)]
pub enum DownloadEvent {
    Searching,
    SearchResults(Vec<SearchItem>),
    InfoLoaded {
        url: String,
        title: String,
        artist: String,
    },
    CoverLoading,
    CoverResults(Vec<CoverCandidate>),
    Progress {
        percent: f32,
        speed: String,
        eta: String,
        stage: String,
    },
    Status(String),
    Completed {
        title: String,
        artist: String,
        mp3_path: String,
        lrc_path: Option<String>,
        cover_source: Option<String>,
    },
    Error(String),
}

pub struct DownloadState {
    pub query: String,
    pub is_input_active: bool,
    pub is_searching: bool,
    pub search_results: Vec<SearchItem>,
    pub selected_result: usize,
    pub result_list_state: ratatui::widgets::ListState,

    // Mode 2: Metadata Review / Edit Form
    pub show_metadata_form: bool,
    pub form_field_idx: usize, // 0: Title, 1: Artist, 2: Album, 3: Save to (Directory), 4: Cover pick
    pub form_url: String,
    pub form_title: String,
    pub form_artist: String,
    pub form_album: String,
    pub form_dir: PathBuf,
    pub form_cover_mode: String,
    pub form_lyrics_mode: usize, // 0: All (Manual + Auto), 1: Creator Only (No Auto), 2: Disabled

    // Studio cover candidates for field 4 (None = Auto best, Some(i) = picked).
    pub cover_candidates: Vec<CoverCandidate>,
    pub selected_cover: Option<usize>,
    pub is_cover_loading: bool,

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
            is_input_active: true,
            is_searching: false,
            search_results: Vec::new(),
            selected_result: 0,
            result_list_state: ratatui::widgets::ListState::default(),

            show_metadata_form: false,
            form_field_idx: 0,
            form_url: String::new(),
            form_title: String::new(),
            form_artist: String::new(),
            form_album: String::new(),
            form_dir: initial_dir.to_path_buf(),
            form_cover_mode: "auto".to_string(),
            form_lyrics_mode: 0,
            cover_candidates: Vec::new(),
            selected_cover: None,
            is_cover_loading: false,

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
}

#[derive(Deserialize)]
struct RawSearchResult {
    items: Option<Vec<SearchItem>>,
    message: Option<String>,
}

#[derive(Deserialize)]
struct RawInfoData {
    url: String,
    title: String,
    artist: String,
}

#[derive(Deserialize)]
struct RawInfoResult {
    data: Option<RawInfoData>,
    message: Option<String>,
}

#[derive(Deserialize)]
struct RawCoverResult {
    items: Option<Vec<CoverCandidate>>,
}

#[derive(Deserialize)]
struct RawEvent {
    #[serde(rename = "type")]
    event_type: String,
    percent: Option<f32>,
    speed: Option<String>,
    eta: Option<String>,
    stage: Option<String>,
    #[serde(alias = "audio_path")]
    mp3_path: Option<String>,
    #[serde(alias = "lyric_path")]
    lrc_path: Option<String>,
    title: Option<String>,
    artist: Option<String>,
    message: Option<String>,
    cover_source: Option<String>,
}

pub struct Downloader {
    sender: Sender<DownloadEvent>,
    pub receiver: Receiver<DownloadEvent>,
    child_pid: Arc<Mutex<Option<u32>>>,
}

impl Downloader {
    pub fn new() -> Self {
        let (sender, receiver) = channel();
        Self {
            sender,
            receiver,
            child_pid: Arc::new(Mutex::new(None)),
        }
    }

    pub fn cancel(&self) {
        if let Ok(mut lock) = self.child_pid.lock() {
            if let Some(pid) = lock.take() {
                #[cfg(unix)]
                {
                    let _ = Command::new("kill").args(["-TERM", &pid.to_string()]).status();
                }
            }
        }
    }

    fn find_script() -> PathBuf {
        if let Ok(env_path) = std::env::var("LYRA_PYTHON_SCRIPT") {
            let p = PathBuf::from(env_path);
            if p.exists() {
                return p;
            }
        }
        let candidates = [
            PathBuf::from("/home/danglong/projects/lyra/main.py"),
            PathBuf::from("main.py"),
            PathBuf::from("../main.py"),
        ];
        for c in candidates {
            if c.exists() {
                return c;
            }
        }
        PathBuf::from("main.py")
    }

    pub fn search(&self, query: String) {
        let sender = self.sender.clone();
        let script = Self::find_script();
        let _ = sender.send(DownloadEvent::Searching);

        thread::spawn(move || {
            let script_dir = script.parent().unwrap_or(std::path::Path::new("."));
            let output = Command::new("python3")
                .env("PYTHONPATH", script_dir)
                .arg(&script)
                .arg("search")
                .arg(&query)
                .arg("--limit")
                .arg("5")
                .arg("--json")
                .output();

            match output {
                Ok(out) => {
                    let text = String::from_utf8_lossy(&out.stdout);
                    if let Ok(res) = serde_json::from_str::<RawSearchResult>(text.trim()) {
                        if let Some(items) = res.items {
                            let _ = sender.send(DownloadEvent::SearchResults(items));
                            return;
                        }
                    }
                    let err = String::from_utf8_lossy(&out.stderr);
                    let msg = if err.trim().is_empty() {
                        "No search results found.".to_string()
                    } else {
                        err.trim().to_string()
                    };
                    let _ = sender.send(DownloadEvent::Error(msg));
                }
                Err(e) => {
                    let _ = sender.send(DownloadEvent::Error(format!("Failed to run search: {e}")));
                }
            }
        });
    }

    pub fn fetch_info(&self, url: String) {
        let sender = self.sender.clone();
        let script = Self::find_script();

        thread::spawn(move || {
            let script_dir = script.parent().unwrap_or(std::path::Path::new("."));
            let output = Command::new("python3")
                .env("PYTHONPATH", script_dir)
                .arg(&script)
                .arg("info")
                .arg(&url)
                .arg("--json")
                .output();

            match output {
                Ok(out) => {
                    let text = String::from_utf8_lossy(&out.stdout);
                    if let Ok(res) = serde_json::from_str::<RawInfoResult>(text.trim()) {
                        if let Some(data) = res.data {
                            let _ = sender.send(DownloadEvent::InfoLoaded {
                                url: data.url,
                                title: data.title,
                                artist: data.artist,
                            });
                            return;
                        }
                    }
                    let err = String::from_utf8_lossy(&out.stderr);
                    let msg = if !err.trim().is_empty() {
                        err.trim().to_string()
                    } else {
                        "Failed to parse video info".to_string()
                    };
                    let _ = sender.send(DownloadEvent::Error(msg));
                }
                Err(e) => {
                    let _ = sender.send(DownloadEvent::Error(format!("Failed to fetch info: {e}")));
                }
            }
        });
    }

    pub fn fetch_covers(&self, artist: String, title: String) {
        let sender = self.sender.clone();
        let script = Self::find_script();
        let _ = sender.send(DownloadEvent::CoverLoading);

        thread::spawn(move || {
            let script_dir = script.parent().unwrap_or(std::path::Path::new("."));
            let output = Command::new("python3")
                .env("PYTHONPATH", script_dir)
                .arg(&script)
                .arg("cover")
                .arg("--artist")
                .arg(&artist)
                .arg("--title")
                .arg(&title)
                .arg("--limit")
                .arg("5")
                .arg("--json")
                .output();

            match output {
                Ok(out) => {
                    let text = String::from_utf8_lossy(&out.stdout);
                    if let Ok(res) = serde_json::from_str::<RawCoverResult>(text.trim()) {
                        let items = res.items.unwrap_or_default();
                        let _ = sender.send(DownloadEvent::CoverResults(items));
                        return;
                    }
                    // No parseable candidates: not fatal, pipeline still has
                    // YouTube fallback. Report empty list to stop the spinner.
                    let _ = sender.send(DownloadEvent::CoverResults(Vec::new()));
                }
                Err(_) => {
                    let _ = sender.send(DownloadEvent::CoverResults(Vec::new()));
                }
            }
        });
    }

    pub fn start_download(&self, req: DownloadRequest) {
        let sender = self.sender.clone();
        let script = Self::find_script();
        let child_pid_clone = Arc::clone(&self.child_pid);

        thread::spawn(move || {
            let script_dir = script.parent().unwrap_or(std::path::Path::new("."));
            let mut cmd = Command::new("python3");
            cmd.env("PYTHONPATH", script_dir)
                .arg(&script)
                .arg("get")
                .arg(&req.url)
                .arg("-o")
                .arg(&req.output_dir)
                .arg("--cover-mode")
                .arg(&req.cover_mode)
                .arg("--cover-source")
                .arg("auto")
                .arg("--name")
                .arg(&req.title)
                .arg("--singer")
                .arg(&req.artist)
                .arg("--json")
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());

            if let Some(url) = req.cover_url.as_ref() {
                if !url.is_empty() {
                    cmd.arg("--cover-url").arg(url);
                }
            }

            if !req.album.is_empty() {
                cmd.arg("--album").arg(&req.album);
            }

            if req.no_lyrics {
                cmd.arg("--no-lyrics");
            } else if req.no_auto_lyrics {
                cmd.arg("--no-auto-lyrics");
            }

            let mut child = match cmd.spawn() {
                Ok(c) => c,
                Err(e) => {
                    let _ = sender.send(DownloadEvent::Error(format!("Failed to start download process: {e}")));
                    return;
                }
            };

            let pid = child.id();
            if let Ok(mut lock) = child_pid_clone.lock() {
                *lock = Some(pid);
            }

            // Concurrently drain stderr in a separate thread to prevent pipe buffer deadlock
            let stderr_handle = if let Some(stderr) = child.stderr.take() {
                Some(thread::spawn(move || {
                    let mut reader = BufReader::new(stderr);
                    let mut err_str = String::new();
                    let _ = reader.read_to_string(&mut err_str);
                    err_str
                }))
            } else {
                None
            };

            let mut completed = false;
            let mut error_emitted = false;

            if let Some(stdout) = child.stdout.take() {
                let reader = BufReader::new(stdout);
                for line in reader.lines().flatten() {
                    let line = line.trim();
                    if line.starts_with('{') {
                        if let Ok(ev) = serde_json::from_str::<RawEvent>(line) {
                            match ev.event_type.as_str() {
                                "progress" => {
                                    let _ = sender.send(DownloadEvent::Progress {
                                        percent: ev.percent.unwrap_or(0.0),
                                        speed: ev.speed.unwrap_or_default(),
                                        eta: ev.eta.unwrap_or_default(),
                                        stage: ev.stage.unwrap_or_else(|| "Downloading...".into()),
                                    });
                                }
                                "status" => {
                                    if let Some(st) = ev.stage {
                                        let _ = sender.send(DownloadEvent::Status(st));
                                    }
                                }
                                "done" => {
                                    completed = true;
                                    let _ = sender.send(DownloadEvent::Completed {
                                        title: ev.title.unwrap_or_else(|| req.title.clone()),
                                        artist: ev.artist.unwrap_or_else(|| req.artist.clone()),
                                        mp3_path: ev.mp3_path.unwrap_or_default(),
                                        lrc_path: ev.lrc_path,
                                        cover_source: ev.cover_source,
                                    });
                                }
                                "error" => {
                                    error_emitted = true;
                                    let _ = sender.send(DownloadEvent::Error(
                                        ev.message.unwrap_or_else(|| "Unknown error occurred".into()),
                                    ));
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }

            let stderr_output = if let Some(handle) = stderr_handle {
                handle.join().unwrap_or_default()
            } else {
                String::new()
            };

            let wait_res = child.wait();

            if let Ok(mut lock) = child_pid_clone.lock() {
                if lock.as_ref() == Some(&pid) {
                    *lock = None;
                }
            }

            // Check if process finished cleanly with Completed or Error
            if !completed && !error_emitted {
                let mut err_msg = String::new();
                let stderr_trimmed = stderr_output.trim();

                if !stderr_trimmed.is_empty() {
                    let lines: Vec<&str> = stderr_trimmed.lines().filter(|l| !l.trim().is_empty()).collect();
                    if let Some(err_line) = lines.iter().rev().find(|l| {
                        l.contains("Error") || l.contains("ERROR") || l.contains("Exception")
                    }) {
                        err_msg = err_line.trim().to_string();
                    } else if let Some(last) = lines.last() {
                        err_msg = last.trim().to_string();
                    } else {
                        err_msg = stderr_trimmed.to_string();
                    }
                }

                if err_msg.is_empty() {
                    match wait_res {
                        Ok(status) => {
                            if !status.success() {
                                err_msg = match status.code() {
                                    Some(code) => format!("Download process failed with exit code {code}"),
                                    None => "Download process terminated unexpectedly".to_string(),
                                };
                            } else {
                                err_msg = "Download process completed without emitting results".to_string();
                            }
                        }
                        Err(e) => {
                            err_msg = format!("Failed to wait for download process: {e}");
                        }
                    }
                }

                let _ = sender.send(DownloadEvent::Error(err_msg));
            }
        });
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
    fn test_downloader_emits_error_on_invalid_url() {
        let downloader = Downloader::new();
        downloader.start_download(DownloadRequest {
            url: "https://www.youtube.com/watch?v=invalid_test_fail_id".to_string(),
            output_dir: PathBuf::from("/tmp"),
            title: "Test".to_string(),
            artist: "Test".to_string(),
            album: "".to_string(),
            cover_mode: "blur_pad".to_string(),
            cover_url: None,
            no_lyrics: false,
            no_auto_lyrics: false,
        });

        let start = std::time::Instant::now();
        let mut got_error = false;
        while start.elapsed() < std::time::Duration::from_secs(10) {
            if let Ok(event) = downloader.receiver.recv_timeout(std::time::Duration::from_millis(500)) {
                if let DownloadEvent::Error(msg) = event {
                    assert!(!msg.is_empty(), "Error message should not be empty");
                    got_error = true;
                    break;
                }
            }
        }
        assert!(got_error, "Expected DownloadEvent::Error on invalid URL");
    }

    #[test]
    fn test_downloader_cancel() {
        let downloader = Downloader::new();
        downloader.start_download(DownloadRequest {
            url: "https://www.youtube.com/watch?v=dQw4w9WgXcQ".to_string(),
            output_dir: PathBuf::from("/tmp"),
            title: "Test".to_string(),
            artist: "Test".to_string(),
            album: "".to_string(),
            cover_mode: "blur_pad".to_string(),
            cover_url: None,
            no_lyrics: false,
            no_auto_lyrics: false,
        });

        // Give it 150ms to spawn child process
        std::thread::sleep(std::time::Duration::from_millis(150));
        downloader.cancel();

        let start = std::time::Instant::now();
        let mut finished = false;
        while start.elapsed() < std::time::Duration::from_secs(5) {
            if let Ok(event) = downloader.receiver.recv_timeout(std::time::Duration::from_millis(500)) {
                if let DownloadEvent::Error(_) = event {
                    finished = true;
                    break;
                }
            }
        }
        assert!(finished, "Expected process to terminate on cancel");
    }
}
