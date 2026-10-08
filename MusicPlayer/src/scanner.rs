use std::{
    fs,
    path::PathBuf,
    sync::mpsc::{channel, Receiver, Sender},
};

pub enum ScanMessage {
    Progress {
        folders_scanned: usize,
        songs_found: usize,
    },
    Finished(Vec<PathBuf>),
}

pub struct Scanner {
    pub is_scanning: bool,
    pub folders_scanned: usize,
    pub songs_found: usize,
    pub anim_tick: usize,
    /// Pending scan results awaiting user action (Replace / Append / Queue / Dismiss).
    pub pending_result: Option<Vec<PathBuf>>,
    receiver: Option<Receiver<ScanMessage>>,
}

impl Scanner {
    pub fn new() -> Self {
        Self {
            is_scanning: false,
            folders_scanned: 0,
            songs_found: 0,
            anim_tick: 0,
            pending_result: None,
            receiver: None,
        }
    }

    pub fn start(&mut self, root_folder: PathBuf, include_hidden: bool) {
        if self.is_scanning {
            return;
        }

        let (sender, receiver) = channel();
        self.is_scanning = true;
        self.folders_scanned = 0;
        self.songs_found = 0;
        self.anim_tick = 0;
        self.pending_result = None;
        self.receiver = Some(receiver);

        start_background_scan(root_folder, include_hidden, sender);
    }

    pub fn tick(&mut self) -> Option<Vec<PathBuf>> {
        if !self.is_scanning {
            return None;
        }

        self.anim_tick = self.anim_tick.wrapping_add(1);

        let mut finished_songs = None;

        if let Some(ref rx) = self.receiver {
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    ScanMessage::Progress { folders_scanned, songs_found } => {
                        self.folders_scanned = folders_scanned;
                        self.songs_found = songs_found;
                    }
                    ScanMessage::Finished(songs) => {
                        finished_songs = Some(songs);
                        break;
                    }
                }
            }
        }

        if let Some(songs) = finished_songs {
            self.is_scanning = false;
            self.receiver = None;
            Some(songs)
        } else {
            None
        }
    }
}

/// Spawns a background worker thread to recursively scan for .mp3 files
fn start_background_scan(
    root_folder: PathBuf,
    include_hidden: bool,
    sender: Sender<ScanMessage>,
) {
    std::thread::spawn(move || {
        let mut found_songs = Vec::new();
        let mut folder_stack = vec![root_folder];
        let mut folders_scanned = 0;

        while let Some(current_folder) = folder_stack.pop() {
            folders_scanned += 1;

            if folders_scanned % 5 == 0 {
                let _ = sender.send(ScanMessage::Progress {
                    folders_scanned,
                    songs_found: found_songs.len(),
                });
            }

            if let Ok(entries) = fs::read_dir(&current_folder) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let file_name = match path.file_name() {
                        Some(name) => name.to_string_lossy(),
                        None => continue,
                    };

                    if !include_hidden && file_name.starts_with('.') {
                        continue;
                    }

                    if path.is_dir() {
                        if !include_hidden && (file_name == "target" || file_name == "node_modules") {
                            continue;
                        }
                        folder_stack.push(path);
                    } else if path.is_file() {
                        if let Some(extension) = path.extension() {
                            if extension.eq_ignore_ascii_case("mp3") {
                                found_songs.push(path);
                            }
                        }
                    }
                }
            }
        }

        found_songs.sort();
        let _ = sender.send(ScanMessage::Finished(found_songs));
    });
}
