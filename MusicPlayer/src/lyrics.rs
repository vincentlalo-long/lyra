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

            // Parse timestamp format: e.g. [mm:ss.xx] or multiple [mm:ss.xx][mm:ss.xx]
            if trimmed.starts_with('[') {
                let mut timestamps = Vec::new();
                let mut rest = trimmed;

                while rest.starts_with('[') {
                    if let Some(close_bracket) = rest.find(']') {
                        let time_str = &rest[1..close_bracket];
                        if let Some(duration) = parse_timestamp(time_str) {
                            timestamps.push(duration);
                        }
                        rest = &rest[close_bracket + 1..];
                    } else {
                        break;
                    }
                }

                let text = rest.trim().to_string();
                if !timestamps.is_empty() && !text.is_empty() {
                    for ts in timestamps {
                        lines.push(LyricLine {
                            timestamp: ts,
                            text: text.clone(),
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
}
