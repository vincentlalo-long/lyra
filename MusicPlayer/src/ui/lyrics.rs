use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};
use crate::app::{App, LyricVerifyStatus};

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let verify_status = app
        .current_playing_path
        .as_ref()
        .and_then(|p| app.lyric_verify_cache.get(p));

    let (badge, title_color) = match &app.lyrics {
        Some(l) if !l.lines.is_empty() => match (l.synced, verify_status) {
            (true, Some(LyricVerifyStatus::Checking)) => {
                (" 󰎈 Lyrics [Synced • Checking…] ".to_string(), app.theme.info)
            }
            (true, Some(LyricVerifyStatus::Verified { confidence })) => {
                (format!(" 󰎈 Lyrics [Synced ✓ {confidence}%] "), app.theme.success)
            }
            (true, Some(LyricVerifyStatus::Mismatch { confidence, .. })) => (
                format!(" 󰎈 Lyrics [Synced ⚠️ Mismatch {confidence}% — g: Fix] "),
                app.theme.danger,
            ),
            (false, Some(LyricVerifyStatus::Mismatch { confidence, .. })) => (
                format!(" 󰎈 Lyrics [Plain ⚠️ Mismatch {confidence}% — g: Fix] "),
                app.theme.danger,
            ),
            (false, Some(LyricVerifyStatus::Plain { confidence })) => (
                format!(" 󰎈 Lyrics [Plain ✓ {confidence}% • g: Get Synced] "),
                app.theme.warning,
            ),
            (true, _) => (" 󰎈 Lyrics [Synced • V: Verify] ".to_string(), app.theme.accent),
            (false, _) => (" 󰎈 Lyrics [Plain • g: Get Synced] ".to_string(), app.theme.warning),
        },
        _ => (" 󰎈 Lyrics ".to_string(), app.theme.accent),
    };

    let border_color = match verify_status {
        Some(LyricVerifyStatus::Mismatch { .. }) => app.theme.danger,
        _ => app.theme.accent,
    };

    let block = Block::default()
        .title(Span::styled(
            badge,
            Style::default().fg(title_color).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color));

    let lyrics = match &app.lyrics {
        Some(l) if !l.lines.is_empty() => l,
        _ => {
            let empty_msg = Paragraph::new(vec![
                Line::raw(""),
                Line::raw(""),
                Line::from(Span::styled(
                    "󰎈  No synchronized lyrics (.lrc) found",
                    Style::default().fg(app.theme.faint),
                )),
                Line::raw(""),
                Line::from(Span::styled(
                    "Press 'g' to search & download lyrics via LRCLIB",
                    Style::default().fg(app.theme.info),
                )),
                Line::from(Span::styled(
                    "or drop a matching .lrc file in the same folder",
                    Style::default().fg(app.theme.faint),
                )),
            ])
            .alignment(Alignment::Center)
            .block(block);

            frame.render_widget(empty_msg, area);
            return;
        }
    };

    let current_pos = app.audio.position();
    let active_index = if lyrics.synced {
        lyrics
            .lines
            .iter()
            .position(|l| l.timestamp > current_pos)
            .unwrap_or(lyrics.lines.len())
            .saturating_sub(1)
    } else if let Some(total) = app.audio.duration {
        let ratio = if total.as_secs_f32() > 0.0 {
            (current_pos.as_secs_f32() / total.as_secs_f32()).clamp(0.0, 0.999)
        } else {
            0.0
        };
        ((ratio * lyrics.lines.len() as f32) as usize).min(lyrics.lines.len().saturating_sub(1))
    } else {
        0
    };

    let inner_height = area.height.saturating_sub(2) as usize;
    let inner_width = area.width.saturating_sub(4) as usize;
    let half_height = inner_height / 2;
    let start_idx = active_index.saturating_sub(half_height);
    let end_idx = (start_idx + inner_height).min(lyrics.lines.len());

    let mut text_lines = Vec::new();

    // Pad top to center the active line
    let pad_top = half_height.saturating_sub(active_index);
    for _ in 0..pad_top {
        text_lines.push(Line::raw(""));
    }

    for i in start_idx..end_idx {
        let line_data = &lyrics.lines[i];
        let distance = if i >= active_index {
            i - active_index
        } else {
            active_index - i
        };

        // Depth of Field styling
        let (style, prefix, suffix) = match distance {
            0 => (
                Style::default().fg(app.theme.warning).add_modifier(Modifier::BOLD),
                "▶  ",
                "  ◀",
            ),
            1 => (Style::default().fg(app.theme.text), "   ", ""),
            2 => (Style::default().fg(app.theme.muted), "   ", ""),
            3 => (Style::default().fg(app.theme.faint), "   ", ""),
            _ => (Style::default().fg(app.theme.faint), "   ", ""),
        };

        let max_text_chars = inner_width.saturating_sub(prefix.chars().count() + suffix.chars().count());
        let body = if max_text_chars > 3 && line_data.text.chars().count() > max_text_chars {
            let mut s: String = line_data.text.chars().take(max_text_chars.saturating_sub(1)).collect();
            s.push('…');
            s
        } else {
            line_data.text.clone()
        };

        let formatted = format!("{prefix}{body}{suffix}");
        text_lines.push(Line::from(Span::styled(formatted, style)));
    }

    let paragraph = Paragraph::new(text_lines)
        .alignment(Alignment::Center)
        .block(block);

    frame.render_widget(paragraph, area);
}
