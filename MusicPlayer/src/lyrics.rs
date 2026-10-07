use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
    time::Duration,
};

#[derive(Clone, Debug)]
pub struct LyricLine {
    pub timestamp: Duration,
    pub text: String,
}

#[derive(Clone, Debug, Default)]
pub struct Lyrics {
    pub lines: Vec<LyricLine>,
}

impl Lyrics {
    /// Loads matching .lrc file if present in the same directory as song
    pub fn load_for_song(song_path: &Path) -> Option<Self> {
        let lrc_path = song_path.with_extension("lrc");
        if !lrc_path.exists() {
            return None;
        }

        let file = File::open(&lrc_path).ok()?;
        let reader = BufReader::new(file);
        let mut lines = Vec::new();

        for line_res in reader.lines() {
            let line = line_res.ok()?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            // Parse timestamp format: [mm:ss.xx]
            if trimmed.starts_with('[') {
                if let Some(close_bracket) = trimmed.find(']') {
                    let time_str = &trimmed[1..close_bracket];
                    let text = trimmed[close_bracket + 1..].trim().to_string();

                    if let Some(duration) = parse_timestamp(time_str) {
                        lines.push(LyricLine {
                            timestamp: duration,
                            text,
                        });
                    }
                }
            }
        }

        if lines.is_empty() {
            return None;
        }

        lines.sort_by_key(|l| l.timestamp);
        Some(Self { lines })
    }

    /// Finds the currently active lyric line for the given playback position
    #[allow(dead_code)]
    pub fn current_line(&self, position: Duration) -> Option<&LyricLine> {
        self.lines
            .iter()
            .take_while(|l| l.timestamp <= position)
            .last()
    }
}

fn parse_timestamp(tag: &str) -> Option<Duration> {
    let parts: Vec<&str> = tag.split(':').collect();
    if parts.len() < 2 {
        return None;
    }

    let minutes: u64 = parts[0].parse().ok()?;
    let sec_parts: Vec<&str> = parts[1].split('.').collect();
    let seconds: u64 = sec_parts[0].parse().ok()?;
    let millis: u64 = if sec_parts.len() > 1 {
        let ms_str = sec_parts[1];
        if ms_str.len() == 2 {
            ms_str.parse::<u64>().ok()? * 10
        } else {
            ms_str.parse::<u64>().ok()?
        }
    } else {
        0
    };

    Some(Duration::from_millis(
        minutes * 60_000 + seconds * 1_000 + millis,
    ))
}
