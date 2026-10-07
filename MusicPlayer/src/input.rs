use crossterm::event::KeyCode;
use crate::app::{App, ViewMode};

/// Dispatches key presses to corresponding app actions.
/// Returns false when the app should quit, true otherwise.
pub fn handle_key(app: &mut App, key: KeyCode) -> bool {
    match key {
        // Quit
        KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => return false,

        // Switch view & Background scan
        KeyCode::Tab => app.toggle_view(),
        KeyCode::Char('1') => app.view_mode = ViewMode::Playlist,
        KeyCode::Char('2') => app.view_mode = ViewMode::Browser,
        KeyCode::Char('3') => app.view_mode = ViewMode::Lyrics,
        KeyCode::Char('.') => app.browser.toggle_hidden(),
        KeyCode::Char('s') | KeyCode::Char('S') => app.start_scan(),

        // Navigation
        KeyCode::Up | KeyCode::Char('k') => app.previous(),
        KeyCode::Down | KeyCode::Char('j') => app.next(),
        KeyCode::Enter => app.on_enter(),

        // Playback control
        KeyCode::Char(' ') => app.audio.toggle_pause(),
        KeyCode::Char('n') | KeyCode::Char('N') => app.play_next_track(),
        KeyCode::Char('p') | KeyCode::Char('P') => app.play_prev_track(),

        // Seek forward / backward (5 seconds)
        KeyCode::Left | KeyCode::Char('h') => app.audio.seek_backward(5),
        KeyCode::Right | KeyCode::Char('l') => app.audio.seek_forward(5),

        // Volume control
        KeyCode::Char('+') | KeyCode::Char('=') => app.audio.volume_up(),
        KeyCode::Char('-') | KeyCode::Char('_') => app.audio.volume_down(),
        KeyCode::Char('m') | KeyCode::Char('M') => app.audio.toggle_mute(),

        _ => {}
    }

    true
}
