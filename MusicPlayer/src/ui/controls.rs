use std::time::Duration;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use crate::app::App;

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let sub_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Line 1: Track Status & Volume Bar
            Constraint::Length(1), // Line 2: Playback Progress & Seek Bar
            Constraint::Length(1), // Line 3: Live Lyric line ticker
            Constraint::Length(1), // Line 4: Keybindings help
        ])
        .split(Block::default().borders(Borders::ALL).title(" Controls ").inner(area));

    // 1. Status and Volume
    let status_str = if let Some(playing_index) = app.playlist.playing_index {
        let song_name = match app.playlist.songs.get(playing_index) {
            Some(path) => path.file_name().and_then(|n| n.to_str()).unwrap_or("Unknown"),
            None => "Unknown",
        };
        let play_status = if app.audio.is_paused { "[PAUSED]" } else { "[PLAYING]" };
        format!("Status: {play_status}  Track: {song_name}")
    } else {
        "Status: [STOPPED]  No track selected".to_string()
    };

    let vol_percent = (app.audio.volume * 100.0).round() as u32;
    let filled_vol = (vol_percent / 10).min(10) as usize;
    let empty_vol = 10 - filled_vol;
    let volume_display = format!("Vol: {:>3}% [{}{}]", vol_percent, "=".repeat(filled_vol), " ".repeat(empty_vol));

    let line1 = Line::from(vec![
        Span::styled(status_str, Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        Span::raw("    "),
        Span::styled(volume_display, Style::default().fg(if vol_percent == 0 { Color::Red } else { Color::Yellow })),
    ]);

    // 2. Playback Progress & Seek Bar
    let current_pos = app.audio.position();
    let duration_opt = app.audio.duration;

    let time_str = match duration_opt {
        Some(total) => format!("{} / {}", format_time(current_pos), format_time(total)),
        None => format!("{} / --:--", format_time(current_pos)),
    };

    let progress_bar = match duration_opt {
        Some(total) if total.as_secs() > 0 => {
            let percent = (current_pos.as_secs_f32() / total.as_secs_f32()).clamp(0.0, 1.0);
            let bar_len = 32;
            let filled = ((percent * bar_len as f32).round() as usize).min(bar_len);
            let marker = if filled < bar_len { ">" } else { "=" };
            let head = "=".repeat(filled.saturating_sub(1));
            let tail = "-".repeat(bar_len.saturating_sub(filled));
            format!("[{head}{marker}{tail}] {:>3}%", (percent * 100.0) as u32)
        }
        _ => "[--------------------------------]   0%".to_string(),
    };

    let line2 = Line::from(vec![
        Span::styled(format!("Time: {time_str}  "), Style::default().fg(Color::Cyan)),
        Span::styled(progress_bar, Style::default().fg(Color::Yellow)),
    ]);

    // 3. Live Lyric Ticker
    let active_lyric = app
        .lyrics
        .as_ref()
        .and_then(|l| l.current_line(current_pos))
        .map(|l| l.text.as_str());

    let line3 = match active_lyric {
        Some(text) if !text.is_empty() => Line::from(vec![
            Span::styled("💬 ", Style::default().fg(Color::Magenta)),
            Span::styled(format!("\"{text}\""), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        ]),
        _ => Line::from(Span::styled("💬 (No synced lyrics)", Style::default().fg(Color::DarkGray))),
    };

    // 4. Keybindings Help
    let keybindings_help = "[Tab] View [h/l, ←/→] Seek -/+5s [Space] Pause [n/p] Next/Prev [+/-] Vol [s] Scan [q] Quit";
    let line4 = Line::from(Span::styled(keybindings_help, Style::default().fg(Color::DarkGray)));

    // Outer block
    frame.render_widget(Block::default().borders(Borders::ALL).title(" Controls "), area);
    frame.render_widget(Paragraph::new(line1), sub_chunks[0]);
    frame.render_widget(Paragraph::new(line2), sub_chunks[1]);
    frame.render_widget(Paragraph::new(line3), sub_chunks[2]);
    frame.render_widget(Paragraph::new(line4), sub_chunks[3]);
}

fn format_time(dur: Duration) -> String {
    let total_secs = dur.as_secs();
    let mins = total_secs / 60;
    let secs = total_secs % 60;
    format!("{mins:02}:{secs:02}")
}
