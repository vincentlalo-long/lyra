use std::{
    io::{BufRead, BufReader, Read},
    path::PathBuf,
    process::{Command, Stdio},
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc, Mutex,
    },
    thread,
};
use serde::Deserialize;

use super::events::{CoverCandidate, DownloadEvent, DownloadRequest, SearchItem};

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

fn format_python_error(stderr: &str, default_msg: &str) -> String {
    let trimmed = stderr.trim();
    if trimmed.contains("No module named 'yt_dlp'")
        || trimmed.contains("No module named 'mutagen'")
        || trimmed.contains("No module named 'PIL'")
    {
        return "Missing Python dependencies. Run: pip install yt-dlp pillow mutagen".to_string();
    }
    if trimmed.is_empty() {
        return default_msg.to_string();
    }
    let lines: Vec<&str> = trimmed.lines().filter(|l| !l.trim().is_empty()).collect();
    if let Some(err_line) = lines.iter().rev().find(|l| {
        l.contains("Error") || l.contains("ERROR") || l.contains("Exception")
    }) {
        err_line.trim().to_string()
    } else if let Some(last) = lines.last() {
        last.trim().to_string()
    } else {
        trimmed.to_string()
    }
}

fn format_spawn_error(action: &str, e: &std::io::Error) -> String {
    if e.kind() == std::io::ErrorKind::NotFound {
        "Python 3 ('python3') not found on system. Please install python3.".to_string()
    } else {
        format!("Failed to {action}: {e}")
    }
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

    pub fn find_script() -> PathBuf {
        // 1. Explicit override via environment variable
        if let Ok(env_path) = std::env::var("LYRA_PYTHON_SCRIPT") {
            let p = PathBuf::from(env_path);
            if p.exists() {
                return p;
            }
        }

        // 2. Relative to current working directory (e.g. developing inside repo root or MusicPlayer)
        let cwd_candidates = [
            PathBuf::from("main.py"),
            PathBuf::from("../main.py"),
            PathBuf::from("../../main.py"),
        ];
        for c in &cwd_candidates {
            if c.exists() && c.parent().unwrap_or(std::path::Path::new("")).join("lyra").exists() {
                if let Ok(abs) = c.canonicalize() {
                    return abs;
                }
                return c.clone();
            }
        }

        // 3. Search relative to the running executable directory
        if let Ok(exe) = std::env::current_exe() {
            let mut curr = exe.parent();
            while let Some(parent) = curr {
                let candidate = parent.join("main.py");
                if candidate.exists() && parent.join("lyra").exists() {
                    return candidate;
                }
                curr = parent.parent();
            }
        }

        // 4. Check known development repo locations in user's home directory
        if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
            let dev_paths = [
                home.join("projects/lyra/main.py"),
                home.join("project/lyra/main.py"),
                home.join("lyra/main.py"),
                home.join("code/lyra/main.py"),
                home.join("src/lyra/main.py"),
            ];
            for p in &dev_paths {
                if p.exists() && p.parent().map(|d| d.join("lyra").exists()).unwrap_or(false) {
                    return p.clone();
                }
            }
        }

        // 5. Check standard user data / config directories
        if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
            let user_paths = [
                home.join(".local/share/lyra/scripts/main.py"),
                home.join(".local/share/lyra/main.py"),
                home.join(".config/lyra/scripts/main.py"),
                home.join(".config/lyra/main.py"),
            ];
            for p in &user_paths {
                if p.exists() {
                    return p.clone();
                }
            }
        }

        // 6. Standalone binary fallback: extract embedded Python helper scripts
        if let Some(extracted) = super::embed::ensure_extracted_scripts() {
            return extracted;
        }

        PathBuf::from("main.py")
    }

    pub fn search(&self, query: String) {
        self.search_with_limit(query, 15);
    }

    pub fn search_with_limit(&self, query: String, limit: usize) {
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
                .arg(limit.to_string())
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
                    let msg = format_python_error(&err, "No search results found.");
                    let _ = sender.send(DownloadEvent::Error(msg));
                }
                Err(e) => {
                    let _ = sender.send(DownloadEvent::Error(format_spawn_error("run search", &e)));
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
                    let msg = format_python_error(&err, "Failed to parse video info");
                    let _ = sender.send(DownloadEvent::Error(msg));
                }
                Err(e) => {
                    let _ = sender.send(DownloadEvent::Error(format_spawn_error("fetch info", &e)));
                }
            }
        });
    }

    pub fn fetch_covers(&self, artist: String, title: String) {
        self.fetch_covers_query(artist, title, None, Some("auto".to_string()));
    }

    pub fn fetch_covers_query(
        &self,
        artist: String,
        title: String,
        query: Option<String>,
        source: Option<String>,
    ) {
        let sender = self.sender.clone();
        let script = Self::find_script();
        let _ = sender.send(DownloadEvent::CoverLoading);

        thread::spawn(move || {
            let script_dir = script.parent().unwrap_or(std::path::Path::new("."));
            let mut cmd = Command::new("python3");
            cmd.env("PYTHONPATH", script_dir)
                .arg(&script)
                .arg("cover");

            if let Some(q) = query.filter(|s| !s.trim().is_empty()) {
                cmd.arg(q.trim());
            } else {
                if !artist.is_empty() {
                    cmd.arg("--artist").arg(&artist);
                }
                if !title.is_empty() {
                    cmd.arg("--title").arg(&title);
                }
            }

            let src = source.unwrap_or_else(|| "auto".to_string());
            cmd.arg("--source").arg(src);
            cmd.arg("--limit").arg("16");
            cmd.arg("--json");

            let output = cmd.output();
            match output {
                Ok(out) => {
                    let text = String::from_utf8_lossy(&out.stdout);
                    if let Ok(res) = serde_json::from_str::<RawCoverResult>(text.trim()) {
                        let items = res.items.unwrap_or_default();
                        let _ = sender.send(DownloadEvent::CoverResults(items));
                        return;
                    }
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

            let cover_arg = req
                .custom_cover
                .as_ref()
                .filter(|s| !s.is_empty())
                .or(req.cover_url.as_ref().filter(|s| !s.is_empty()));
            if let Some(cov) = cover_arg {
                cmd.arg("--cover").arg(cov);
            }

            if !req.album.is_empty() {
                cmd.arg("--album").arg(&req.album);
            }

            if let Some(genre) = &req.genre {
                if !genre.trim().is_empty() {
                    cmd.arg("--genre").arg(genre.trim());
                }
            }

            if req.no_lyrics {
                cmd.arg("--no-lyrics");
            } else if req.no_auto_lyrics {
                cmd.arg("--no-auto-lyrics");
            }

            let mut child = match cmd.spawn() {
                Ok(c) => c,
                Err(e) => {
                    let _ = sender.send(DownloadEvent::Error(format_spawn_error("start download process", &e)));
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
                let stderr_trimmed = stderr_output.trim();
                let mut err_msg = if !stderr_trimmed.is_empty() {
                    format_python_error(stderr_trimmed, "")
                } else {
                    String::new()
                };

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

    #[test]
    fn test_find_script_always_resolves_existing_script() {
        let script = Downloader::find_script();
        assert!(script.exists(), "Resolved script '{:?}' must exist", script);
        assert!(script.ends_with("main.py"), "Script must be main.py");
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
            genre: None,
            cover_mode: "blur_pad".to_string(),
            cover_url: None,
            custom_cover: None,
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
            genre: None,
            cover_mode: "blur_pad".to_string(),
            cover_url: None,
            custom_cover: None,
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
