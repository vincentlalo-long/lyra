use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use crate::app::{App, ViewMode};

pub fn handle_key(app: &mut App, event: KeyEvent) -> bool {
    // If Help popup is open, any key closes it
    if app.show_help {
        app.show_help = false;
        return true;
    }

    // Search Mode
    if app.is_searching {
        match event.code {
            KeyCode::Esc => {
                app.is_searching = false;
                app.search_query.clear();
            }
            KeyCode::Enter => {
                app.on_enter();
            }
            KeyCode::Backspace => {
                if app.search_query.is_empty() {
                    app.is_searching = false;
                } else {
                    app.search_query.pop();
                }
            }
            KeyCode::Up => app.previous(),
            KeyCode::Down => app.next(),
            KeyCode::Char(c) => {
                app.search_query.push(c);
            }
            _ => {}
        }
        return true;
    }

    // Replay current track (Ctrl+r or capital R)
    if (event.modifiers.contains(KeyModifiers::CONTROL) && (event.code == KeyCode::Char('r') || event.code == KeyCode::Char('R')))
        || event.code == KeyCode::Char('R')
    {
        app.replay_current_track();
        return true;
    }

    // Normal Mode
    match event.code {
        // Quit or clear filter
        KeyCode::Char('q') | KeyCode::Char('Q') => return false,
        KeyCode::Esc => {
            if !app.search_query.is_empty() {
                app.search_query.clear();
            } else {
                return false;
            }
        }

        // Toggle Repeat Mode (r: Playlist -> Track -> Off)
        KeyCode::Char('r') => app.toggle_repeat(),

        // Toggle Help popup
        KeyCode::Char('?') => app.show_help = true,

        // Search
        KeyCode::Char('/') => {
            app.is_searching = true;
            app.search_query.clear();
        }

        // Switch left panel between Playlist and Browser
        KeyCode::Tab => app.toggle_view(),
        KeyCode::Char('1') => app.view_mode = ViewMode::Playlist,
        KeyCode::Char('2') => app.view_mode = ViewMode::Browser,
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
