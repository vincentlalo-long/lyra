use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    time::Duration,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LyricLine {
    pub timestamp: Duration,
    pub text: String,
}

#[derive(Clone, Debug, Default)]
pub struct Lyrics {
    pub lines: Vec<LyricLine>,
    pub synced: bool,
}

impl Lyrics {
    /// Loads matching .lrc file if present in the same directory as song
    /// (checking `<stem>.lrc`, `<stem>.vi.lrc`, `<stem>.en.lrc`, `<stem>.*.lrc`),
    /// falling back to embedded ID3 `USLT` lyrics when no sidecar `.lrc` exists.
    pub fn load_for_song(song_path: &Path) -> Option<Self> {
        if let Some(candidate) = Self::find_lrc_path_for_song(song_path) {
            if let Some(parsed) = Self::load_from_lrc_path(&candidate) {
                return Some(parsed);
            }
        }

        // Fallback: check embedded ID3 USLT lyrics frame inside the audio file
        if let Ok(tag) = id3::Tag::read_from_path(song_path) {
            for uslt in tag.lyrics() {
                if let Some(parsed) = Self::parse_str(&uslt.text) {
                    return Some(parsed);
                }
            }
        }

        None
    }

    /// Returns the path to the sidecar `.lrc` file if one exists and parses cleanly.
    pub fn find_lrc_path_for_song(song_path: &Path) -> Option<PathBuf> {
        for candidate in Self::candidate_lrc_paths(song_path) {
            if candidate.is_file() && Self::load_from_lrc_path(&candidate).is_some() {
                return Some(candidate);
            }
        }
        None
    }

    fn candidate_lrc_paths(song_path: &Path) -> Vec<PathBuf> {
        let mut paths = vec![song_path.with_extension("lrc")];
        let Some(parent) = song_path.parent() else {
            return paths;
        };
        let Some(stem) = song_path.file_stem().and_then(|s| s.to_str()) else {
            return paths;
        };
        for lang in ["vi", "en", "ja", "ko", "zh", "lrclib"] {
            paths.push(parent.join(format!("{stem}.{lang}.lrc")));
        }
        if let Ok(entries) = std::fs::read_dir(parent) {
            let prefix = format!("{stem}.");
            let mut extra: Vec<PathBuf> = entries
                .flatten()
                .map(|e| e.path())
                .filter(|p| {
                    p.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("lrc"))
                        && p.file_name()
                            .and_then(|n| n.to_str())
                            .is_some_and(|n| n.starts_with(&prefix))
                })
                .collect();
            extra.sort();
            for p in extra {
                if !paths.contains(&p) {
                    paths.push(p);
                }
            }
        }
        paths
    }

    pub fn load_from_lrc_path(lrc_path: &Path) -> Option<Self> {
        let file = File::open(lrc_path).ok()?;
        let reader = BufReader::new(file);
        let mut raw = String::new();
        for line in reader.lines().map_while(Result::ok) {
            raw.push_str(&line);
            raw.push('\n');
        }
        Self::parse_str(&raw)
    }

    /// Parses `.lrc` content (either synced `[mm:ss.xx]` lines or plain text lines).
    pub fn parse_str(content: &str) -> Option<Self> {
        let mut synced_lines: Vec<(Duration, String)> = Vec::new();
        let mut plain_lines: Vec<String> = Vec::new();
        let mut offset_ms: i64 = 0;

        for raw_line in content.lines() {
            let trimmed = raw_line.trim();
            if trimmed.is_empty() {
                continue;
            }

            // Check for [offset:+/-ms] metadata tag
            if let Some(off) = parse_offset_tag(trimmed) {
                offset_ms = off;
                continue;
            }

            if trimmed.starts_with('[') {
                let mut timestamps = Vec::new();
                let mut rest = trimmed;
                let mut had_non_time_tag = false;

                while rest.starts_with('[') {
                    if let Some(close_bracket) = rest.find(']') {
                        let tag_inner = &rest[1..close_bracket];
                        if let Some(duration) = parse_timestamp(tag_inner) {
                            timestamps.push(duration);
                            rest = &rest[close_bracket + 1..];
                        } else if is_lrc_metadata_tag(tag_inner) {
                            had_non_time_tag = true;
                            rest = &rest[close_bracket + 1..];
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }

                let text = rest.trim().to_string();
                if !timestamps.is_empty() {
                    let display_text = if text.is_empty() {
                        "♪".to_string()
                    } else {
                        text
                    };
                    for ts in timestamps {
                        synced_lines.push((ts, display_text.clone()));
                    }
                } else if !had_non_time_tag && !trimmed.is_empty() {
                    plain_lines.push(trimmed.to_string());
                }
            } else {
                plain_lines.push(trimmed.to_string());
            }
        }

        if !synced_lines.is_empty() {
            let mut lines: Vec<LyricLine> = synced_lines
                .into_iter()
                .map(|(ts, text)| LyricLine {
                    timestamp: apply_offset(ts, offset_ms),
                    text,
                })
                .collect();
            lines.sort_by_key(|l| l.timestamp);

            // Deduplicate consecutive identical instrumental break markers ("♪")
            let mut deduped: Vec<LyricLine> = Vec::with_capacity(lines.len());
            for line in lines {
                if line.text == "♪" && deduped.last().is_some_and(|prev| prev.text == "♪") {
                    continue;
                }
                deduped.push(line);
            }
            if deduped.iter().all(|l| l.text == "♪") {
                return None;
            }
            return Some(Self {
                lines: deduped,
                synced: true,
            });
        }

        if !plain_lines.is_empty() {
            let lines = plain_lines
                .into_iter()
                .enumerate()
                .map(|(idx, text)| LyricLine {
                    timestamp: Duration::from_secs((idx as u64) * 3),
                    text,
                })
                .collect();
            return Some(Self {
                lines,
                synced: false,
            });
        }

        None
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

fn parse_offset_tag(line: &str) -> Option<i64> {
    let lower = line.to_ascii_lowercase();
    if lower.starts_with("[offset:") && lower.ends_with(']') {
        let inner = &line[8..line.len() - 1].trim();
        return inner.parse::<i64>().ok();
    }
    None
}

fn is_lrc_metadata_tag(tag: &str) -> bool {
    let Some((key, _)) = tag.split_once(':') else {
        return false;
    };
    matches!(
        key.trim().to_ascii_lowercase().as_str(),
        "ar" | "ti" | "al" | "au" | "by" | "length" | "re" | "ve" | "id" | "offset" | "tool"
    )
}

fn apply_offset(ts: Duration, offset_ms: i64) -> Duration {
    if offset_ms >= 0 {
        ts.saturating_add(Duration::from_millis(offset_ms as u64))
    } else {
        ts.saturating_sub(Duration::from_millis(offset_ms.unsigned_abs()))
    }
}

fn parse_timestamp(tag: &str) -> Option<Duration> {
    let parts: Vec<&str> = tag.split(':').collect();
    if parts.len() < 2 {
        return None;
    }

    let (hours, minutes, sec_str) = if parts.len() >= 3 {
        let h: u64 = parts[0].trim().parse().ok()?;
        let m: u64 = parts[1].trim().parse().ok()?;
        (h, m, parts[2].trim())
    } else {
        let m: u64 = parts[0].trim().parse().ok()?;
        (0, m, parts[1].trim())
    };

    let sec_parts: Vec<&str> = sec_str.split(|c| c == '.' || c == ',').collect();
    let seconds: u64 = sec_parts[0].trim().parse().ok()?;
    let millis: u64 = if sec_parts.len() > 1 {
        parse_fraction_to_millis(sec_parts[1].trim())
    } else {
        0
    };

    Some(Duration::from_millis(
        hours * 3_600_000 + minutes * 60_000 + seconds * 1_000 + millis,
    ))
}

fn parse_fraction_to_millis(ms_str: &str) -> u64 {
    let mut val = 0u64;
    let mut multiplier = 100u64;
    for c in ms_str.chars().take(3) {
        if let Some(digit) = c.to_digit(10) {
            val += (digit as u64) * multiplier;
            multiplier /= 10;
        } else {
            break;
        }
    }
    val
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_fraction_to_millis() {
        assert_eq!(parse_fraction_to_millis("4"), 400);
        assert_eq!(parse_fraction_to_millis("45"), 450);
        assert_eq!(parse_fraction_to_millis("456"), 456);
        assert_eq!(parse_fraction_to_millis("4567"), 456);
        assert_eq!(parse_fraction_to_millis("456789"), 456);
        assert_eq!(parse_fraction_to_millis("04"), 40);
        assert_eq!(parse_fraction_to_millis("004"), 4);
        assert_eq!(parse_fraction_to_millis(""), 0);
    }

    #[test]
    fn test_parse_timestamp() {
        // 1 digit: [01:23.4] -> 1m 23s 400ms = 83,400ms
        assert_eq!(
            parse_timestamp("01:23.4"),
            Some(Duration::from_millis(83_400))
        );

        // 2 digits: [01:23.45] -> 1m 23s 450ms = 83,450ms
        assert_eq!(
            parse_timestamp("01:23.45"),
            Some(Duration::from_millis(83_450))
        );

        // 3 digits: [01:23.456] -> 1m 23s 456ms = 83,456ms
        assert_eq!(
            parse_timestamp("01:23.456"),
            Some(Duration::from_millis(83_456))
        );

        // 4+ digits microsecond: [01:23.4567] -> 1m 23s 456ms = 83,456ms (NOT 4567ms!)
        assert_eq!(
            parse_timestamp("01:23.4567"),
            Some(Duration::from_millis(83_456))
        );

        // Comma separator: [01:23,50] -> 1m 23s 500ms = 83,500ms
        assert_eq!(
            parse_timestamp("01:23,50"),
            Some(Duration::from_millis(83_500))
        );

        // With hours: [01:02:03.50] -> 1h 2m 3s 500ms = 3,723,500ms
        assert_eq!(
            parse_timestamp("01:02:03.50"),
            Some(Duration::from_millis(3_723_500))
        );

        // No fractions: [01:23] -> 1m 23s = 83,000ms
        assert_eq!(
            parse_timestamp("01:23"),
            Some(Duration::from_millis(83_000))
        );

        // Metadata tag should be None
        assert_eq!(parse_timestamp("ar:Singer"), None);
    }

    #[test]
    fn test_parse_str_synced_with_offset_and_instrumental() {
        let lrc = "[ar:Artist]\n[offset:+500]\n[00:01.00] First line\n[00:05.00]\n[00:06.00]\n[00:10.00] Second line\n";
        let parsed = Lyrics::parse_str(lrc).expect("should parse synced LRC");
        assert!(parsed.synced);
        // Consecutive empty instrumental lines [00:05.00] and [00:06.00] are deduped into a single "♪"
        assert_eq!(parsed.lines.len(), 3);
        assert_eq!(parsed.lines[0].timestamp, Duration::from_millis(1500));
        assert_eq!(parsed.lines[0].text, "First line");
        assert_eq!(parsed.lines[1].timestamp, Duration::from_millis(5500));
        assert_eq!(parsed.lines[1].text, "♪");
        assert_eq!(parsed.lines[2].timestamp, Duration::from_millis(10500));
        assert_eq!(parsed.lines[2].text, "Second line");
    }

    #[test]
    fn test_parse_str_plain_lyrics_fallback() {
        let plain = "[ar:Artist]\n[ti:Song]\nLine one\nLine two\n";
        let parsed = Lyrics::parse_str(plain).expect("should parse plain lyrics");
        assert!(!parsed.synced);
        assert_eq!(parsed.lines.len(), 2);
        assert_eq!(parsed.lines[0].text, "Line one");
        assert_eq!(parsed.lines[1].text, "Line two");
    }
}
