use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};
use crate::{app::App, theme};

pub fn render(frame: &mut Frame, app: &App) {
    let screen_area = frame.area();

    // Compact height so user can scroll smoothly through all keybindings
    let popup_height = 20.min(screen_area.height.saturating_sub(4));
    let popup_width = 74.min(screen_area.width.saturating_sub(4));

    let vertical_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((screen_area.height.saturating_sub(popup_height)) / 2),
            Constraint::Length(popup_height),
            Constraint::Min(0),
        ])
        .split(screen_area);

    let popup_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length((screen_area.width.saturating_sub(popup_width)) / 2),
            Constraint::Length(popup_width),
            Constraint::Min(0),
        ])
        .split(vertical_chunks[1])[1];

    let lines = vec![
        Line::from(Span::styled("  Navigation & Views", Style::default().fg(theme::BLUE).add_modifier(Modifier::BOLD))),
        Line::from(vec![
            Span::styled("    Tab / 1-5  ", Style::default().fg(theme::YELLOW)),
            Span::styled(
                "Switch Playlist / Queue / Browser / Extensions / Plugins",
                Style::default().fg(theme::TEXT),
            ),
        ]),
        Line::from(vec![
            Span::styled("    ↑ / k      ", Style::default().fg(theme::YELLOW)),
            Span::styled("Move cursor up (in lists & settings)", Style::default().fg(theme::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("    ↓ / j      ", Style::default().fg(theme::YELLOW)),
            Span::styled("Move cursor down (in lists & settings)", Style::default().fg(theme::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("    PgUp / PgDn", Style::default().fg(theme::YELLOW)),
            Span::styled("Scroll page up / down", Style::default().fg(theme::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("    g / G      ", Style::default().fg(theme::YELLOW)),
            Span::styled("Jump to top / bottom", Style::default().fg(theme::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("    C          ", Style::default().fg(theme::YELLOW)),
            Span::styled("Jump cursor to currently playing track", Style::default().fg(theme::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("    Enter / l  ", Style::default().fg(theme::YELLOW)),
            Span::styled("Play (Playlist) | Open folder / Add track (Browser) | Toggle (Plugins)", Style::default().fg(theme::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("    Bksp / h   ", Style::default().fg(theme::YELLOW)),
            Span::styled("Go to parent directory (in Browser)", Style::default().fg(theme::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("    P          ", Style::default().fg(theme::YELLOW)),
            Span::styled("Load folder into Playlist without auto-playing (in Browser)", Style::default().fg(theme::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("    i / I      ", Style::default().fg(theme::YELLOW)),
            Span::styled("Import track or album into active Playlist", Style::default().fg(theme::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("    L          ", Style::default().fg(theme::YELLOW)),
            Span::styled("Set current folder as permanent Music Library (in Browser)", Style::default().fg(theme::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("    d / Del    ", Style::default().fg(theme::YELLOW)),
            Span::styled("Remove song from Playlist (no file deletion)", Style::default().fg(theme::TEXT)),
        ]),
        Line::raw(""),
        Line::from(Span::styled("  Playback & Controls", Style::default().fg(theme::GREEN).add_modifier(Modifier::BOLD))),
        Line::from(vec![
            Span::styled("    Space      ", Style::default().fg(theme::YELLOW)),
            Span::styled("Play/Pause playback (or Toggle in Plugins)", Style::default().fg(theme::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("    ← / h      ", Style::default().fg(theme::YELLOW)),
            Span::styled("Seek -5s (Up in Browser | Decrease in Plugins)", Style::default().fg(theme::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("    → / l      ", Style::default().fg(theme::YELLOW)),
            Span::styled("Seek +5s (Open in Browser | Increase in Plugins)", Style::default().fg(theme::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("    [ / ]      ", Style::default().fg(theme::YELLOW)),
            Span::styled("Seek backward / forward 5s (any view)", Style::default().fg(theme::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("    n / p      ", Style::default().fg(theme::YELLOW)),
            Span::styled("Next / Previous track", Style::default().fg(theme::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("    + / -      ", Style::default().fg(theme::YELLOW)),
            Span::styled("Volume Up / Down (also adjusts in Plugins)", Style::default().fg(theme::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("    m          ", Style::default().fg(theme::YELLOW)),
            Span::styled("Toggle Mute", Style::default().fg(theme::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("    r          ", Style::default().fg(theme::YELLOW)),
            Span::styled("Cycle Repeat Mode (All -> One -> Off)", Style::default().fg(theme::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("    L          ", Style::default().fg(theme::YELLOW)),
            Span::styled("Toggle Queue Loop", Style::default().fg(theme::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("    Ctrl+r / R ", Style::default().fg(theme::YELLOW)),
            Span::styled("Replay current track from 00:00", Style::default().fg(theme::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("    v / y      ", Style::default().fg(theme::YELLOW)),
            Span::styled("Toggle Lyrics on / off (Studio <-> Karaoke)", Style::default().fg(theme::TEXT)),
        ]),
        Line::raw(""),
        Line::from(Span::styled("  Queue (Scratchpad, never deletes files)", Style::default().fg(theme::MAUVE).add_modifier(Modifier::BOLD))),
        Line::from(vec![
            Span::styled("    a / A      ", Style::default().fg(theme::YELLOW)),
            Span::styled("Queue song or folder/m3u (at tail / next up)", Style::default().fg(theme::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("    d / e      ", Style::default().fg(theme::YELLOW)),
            Span::styled("Unqueue track / Replace via Browser", Style::default().fg(theme::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("    c / z      ", Style::default().fg(theme::YELLOW)),
            Span::styled("Clear entire queue / Shuffle queue", Style::default().fg(theme::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("    w / o      ", Style::default().fg(theme::YELLOW)),
            Span::styled("Save / Load today's day-list (.m3u)", Style::default().fg(theme::TEXT)),
        ]),
        Line::raw(""),
        Line::from(Span::styled("  Plugin Manager & Settings", Style::default().fg(theme::BLUE).add_modifier(Modifier::BOLD))),
        Line::from(vec![
            Span::styled("    Space/Enter", Style::default().fg(theme::YELLOW)),
            Span::styled("Toggle Plugin / Edit Path / Open Store", Style::default().fg(theme::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("    d / Del    ", Style::default().fg(theme::YELLOW)),
            Span::styled("Remove selected plugin from active list", Style::default().fg(theme::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("    r          ", Style::default().fg(theme::YELLOW)),
            Span::styled("Restore removed plugin", Style::default().fg(theme::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("    e          ", Style::default().fg(theme::YELLOW)),
            Span::styled("Edit selected path (Enter to save, Esc to cancel)", Style::default().fg(theme::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("    D          ", Style::default().fg(theme::YELLOW)),
            Span::styled("Open Community Plugin Store modal", Style::default().fg(theme::TEXT)),
        ]),
        Line::raw(""),
        Line::from(Span::styled("  Search & System Tools", Style::default().fg(theme::PEACH).add_modifier(Modifier::BOLD))),
        Line::from(vec![
            Span::styled(
                if cfg!(feature = "genre") { "    / / f      " } else { "    /          " },
                Style::default().fg(theme::YELLOW),
            ),
            Span::styled(
                if cfg!(feature = "genre") {
                    "Search text / Genre picker (Enter/Space:tick, f:filter)"
                } else {
                    "Search / Filter library (Esc to cancel)"
                },
                Style::default().fg(theme::TEXT),
            ),
        ]),
        Line::from(vec![
            Span::styled("    t / T      ", Style::default().fg(theme::YELLOW)),
            Span::styled("Tag / Edit genre for selected song (Playlist & Browser)", Style::default().fg(theme::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("    s          ", Style::default().fg(theme::YELLOW)),
            Span::styled("Safe scan subdirs (Enter:Replace, a:Add, q:Queue)", Style::default().fg(theme::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("    .          ", Style::default().fg(theme::YELLOW)),
            Span::styled("Toggle hidden dotfiles in Browser", Style::default().fg(theme::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("    ?          ", Style::default().fg(theme::YELLOW)),
            Span::styled("Toggle this Help popup", Style::default().fg(theme::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("    q          ", Style::default().fg(theme::YELLOW)),
            Span::styled("Quit Lyra", Style::default().fg(theme::TEXT)),
        ]),
    ];

    let total_lines = lines.len();
    let inner_height = popup_area.height.saturating_sub(2) as usize;
    let max_scroll = total_lines.saturating_sub(inner_height) as u16;
    let scroll = app.help_scroll.min(max_scroll);

    let block = Block::default()
        .title(" 󰋖 Help & Keybindings ")
        .title_alignment(Alignment::Center)
        .title_bottom(Line::from(vec![
            Span::styled(" [↑/↓/PgUp/PgDn: Scroll | Esc/?: Close] ", Style::default().fg(theme::OVERLAY0)),
        ]))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::MAUVE).add_modifier(Modifier::BOLD));

    frame.render_widget(Clear, popup_area);
    frame.render_widget(Paragraph::new(lines).block(block).scroll((scroll, 0)), popup_area);

    if total_lines > inner_height {
        let mut scrollbar_state = ScrollbarState::new(total_lines)
            .position(scroll as usize)
            .viewport_content_length(inner_height);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None)
            .thumb_style(Style::default().fg(theme::MAUVE))
            .track_style(Style::default().fg(theme::SURFACE0));
        frame.render_stateful_widget(
            scrollbar,
            popup_area.inner(Margin { vertical: 1, horizontal: 0 }),
            &mut scrollbar_state,
        );
    }
}
