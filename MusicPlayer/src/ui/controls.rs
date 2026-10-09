use std::time::Duration;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};
use crate::{app::{App, RepeatMode}, theme};

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(Span::styled(" 󰒓 Controls ", Style::default().fg(theme::MAUVE).add_modifier(Modifier::BOLD)))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::MAUVE));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 2 {
        return;
    }

    let sub_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Line 1: Track title, Repeat, and Volume
            Constraint::Length(1), // Line 2: Seamless Sub-block Seekbar
        ])
        .split(inner);

    // --- Line 1: Track Title, Repeat Mode, Volume ---
    let (icon, label, track_name) = if let Some(playing_idx) = app.playlist.playing_index {
        let name = app.playlist.songs.get(playing_idx)
            .and_then(|p| p.file_stem())
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        if app.audio.is_paused {
            ("󰏤 ", "Paused: ", name)
        } else {
            ("󰐊 ", "Playing: ", name)
        }
    } else {
        ("󰓛 ", "Stopped: ", "No track selected".to_string())
    };

    let row1_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(20),
            Constraint::Length(14),
            Constraint::Length(24),
        ])
        .split(sub_chunks[0]);

    let title_box_w = row1_chunks[0].width as usize;
    let label_w = 2 + label.len();
    let avail_w = title_box_w.saturating_sub(label_w);
    let fitted_name = crate::ui::playlist::fit_width(&track_name, avail_w);

    let title_line = Line::from(vec![
        Span::styled(icon, Style::default().fg(theme::GREEN).add_modifier(Modifier::BOLD)),
        Span::styled(label, Style::default().fg(theme::SUBTEXT0)),
        Span::styled(fitted_name, Style::default().fg(theme::TEXT).add_modifier(Modifier::BOLD)),
    ]);

    // Repeat mode display (queue-loop wins: it overrides library looping).
    let (repeat_icon, repeat_text, repeat_color) = if app.queue.loop_enabled {
        ("󰑖 ", "Loop: Queue", theme::MAUVE)
    } else {
        match app.repeat_mode {
            RepeatMode::Playlist => ("󰑖 ", "Loop: All", theme::BLUE),
            RepeatMode::Track => ("󰑘 ", "Loop: One", theme::YELLOW),
            RepeatMode::Off => ("󰑗 ", "Loop: Off", theme::OVERLAY0),
        }
    };

    let repeat_line = Line::from(vec![
        Span::styled(repeat_icon, Style::default().fg(repeat_color)),
        Span::styled(repeat_text, Style::default().fg(repeat_color)),
    ]);

    // Volume bar: [━━━━━●───]
    let vol_percent = (app.audio.volume * 100.0).round() as u32;
    let vol_filled = (vol_percent / 10).min(10) as usize;
    let vol_empty = 10 - vol_filled;
    let vol_icon = if app.audio.volume == 0.0 { "󰖁 " } else { "󰕾 " };
    let vol_bar = format!("[{}●{}]", "━".repeat(vol_filled), "─".repeat(vol_empty.saturating_sub(1)));

    let vol_line = Line::from(vec![
        Span::styled(vol_icon, Style::default().fg(theme::YELLOW)),
        Span::styled(format!("{vol_percent:>3}% "), Style::default().fg(theme::TEXT)),
        Span::styled(vol_bar, Style::default().fg(theme::MAUVE)),
        Span::raw(" "),
    ]);

    frame.render_widget(Paragraph::new(title_line), row1_chunks[0]);
    frame.render_widget(Paragraph::new(repeat_line).alignment(Alignment::Center), row1_chunks[1]);
    frame.render_widget(Paragraph::new(vol_line).alignment(Alignment::Right), row1_chunks[2]);

    // --- Line 2: Seamless Sub-block Seekbar (Zero Gap) ---
    let current_pos = app.audio.position();
    let duration_opt = app.audio.duration;

    let time_left = format!(" {} ", format_time(current_pos));
    let time_right = match duration_opt {
        Some(total) => format!(" {} ", format_time(total)),
        None => " --:-- ".to_string(),
    };

    let total_width = sub_chunks[1].width as usize;
    let used_width = time_left.len() + time_right.len();
    let bar_width = total_width.saturating_sub(used_width).max(4);

    const SUB_BLOCKS: [&str; 8] = ["", "▏", "▎", "▍", "▌", "▋", "▊", "▉"];

    let (full_blocks, partial_char, empty_blocks) = match duration_opt {
        Some(total) if total.as_secs_f32() > 0.0 => {
            let ratio = (current_pos.as_secs_f32() / total.as_secs_f32()).clamp(0.0, 1.0);
            let total_substeps = (ratio * (bar_width * 8) as f32).round() as usize;
            let full = (total_substeps / 8).min(bar_width);
            let remainder = total_substeps % 8;
            let has_partial = remainder > 0 && full < bar_width;
            let partial = if has_partial { SUB_BLOCKS[remainder] } else { "" };
            let empty = bar_width.saturating_sub(full + if has_partial { 1 } else { 0 });
            (full, partial, empty)
        }
        _ => (0, "", bar_width),
    };

    let filled_str = "█".repeat(full_blocks);
    let empty_str = " ".repeat(empty_blocks);

    let mut spans = vec![
        Span::styled(time_left, Style::default().fg(theme::BLUE)),
        Span::styled(filled_str, Style::default().fg(theme::MAUVE)),
    ];

    if !partial_char.is_empty() {
        spans.push(Span::styled(
            partial_char,
            Style::default().fg(theme::MAUVE).bg(theme::SURFACE0),
        ));
    }

    if empty_blocks > 0 {
        spans.push(Span::styled(
            empty_str,
            Style::default().bg(theme::SURFACE0),
        ));
    }

    spans.push(Span::styled(time_right, Style::default().fg(theme::BLUE)));

    frame.render_widget(Paragraph::new(Line::from(spans)), sub_chunks[1]);
}

fn format_time(dur: Duration) -> String {
    let total_secs = dur.as_secs();
    let mins = total_secs / 60;
    let secs = total_secs % 60;
    format!("{mins:02}:{secs:02}")
}
