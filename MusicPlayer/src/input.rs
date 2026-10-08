use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use crate::app::{App, ViewMode};

pub fn handle_key(app: &mut App, event: KeyEvent) -> bool {
    // If Help popup is open, any key closes it
    if app.show_help {
        app.show_help = false;
        return true;
    }

    // Dedicated handler for Download ViewMode
    if app.view_mode == ViewMode::Download {
        return handle_download_key(app, event);
    }

    // Search Mode (Playlist / Browser)
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
            KeyCode::PageUp => app.page_up(),
            KeyCode::PageDown => app.page_down(),
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

        // Search in Playlist/Browser
        KeyCode::Char('/') => {
            app.is_searching = true;
            app.search_query.clear();
        }

        // Switch tabs
        KeyCode::Tab => app.toggle_view(),
        KeyCode::Char('1') => app.view_mode = ViewMode::Playlist,
        KeyCode::Char('2') => app.view_mode = ViewMode::Browser,
        KeyCode::Char('3') => app.view_mode = ViewMode::Download,
        KeyCode::Char('.') => app.browser.toggle_hidden(),
        KeyCode::Char('s') | KeyCode::Char('S') => app.start_scan(),

        // Navigation
        KeyCode::Up | KeyCode::Char('k') => app.previous(),
        KeyCode::Down | KeyCode::Char('j') => app.next(),
        KeyCode::PageUp => app.page_up(),
        KeyCode::PageDown => app.page_down(),
        KeyCode::Home | KeyCode::Char('g') => app.go_to_top(),
        KeyCode::End | KeyCode::Char('G') => app.go_to_bottom(),
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

fn handle_download_key(app: &mut App, event: KeyEvent) -> bool {
    // 1. Directory Picker Popup
    if app.download.show_dir_picker {
        match event.code {
            KeyCode::Esc => {
                app.download.show_dir_picker = false;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                app.download.dir_picker.previous("");
            }
            KeyCode::Down | KeyCode::Char('j') => {
                app.download.dir_picker.next("");
            }
            KeyCode::Enter => {
                app.download.dir_picker.enter();
            }
            KeyCode::Char(' ') => {
                // Select current picker folder as destination
                app.download.form_dir = app.download.dir_picker.current_dir.clone();
                app.download.show_dir_picker = false;
            }
            _ => {}
        }
        return true;
    }

    // 2. Mode 2: Metadata Form Modal
    if app.download.show_metadata_form {
        // Quick submit via Ctrl+Enter or Ctrl+S from any field
        if event.modifiers.contains(KeyModifiers::CONTROL)
            && (event.code == KeyCode::Enter || event.code == KeyCode::Char('s') || event.code == KeyCode::Char('S'))
        {
            app.start_metadata_download();
            return true;
        }

        // Quick clear current text field via Ctrl+U or Ctrl+K
        if event.modifiers.contains(KeyModifiers::CONTROL)
            && (event.code == KeyCode::Char('u') || event.code == KeyCode::Char('k'))
        {
            match app.download.form_field_idx {
                0 => { app.download.form_title.clear(); }
                1 => { app.download.form_artist.clear(); }
                2 => { app.download.form_album.clear(); }
                _ => {}
            }
            return true;
        }

        // Quick delete word via Ctrl+W or Ctrl+Backspace
        if event.modifiers.contains(KeyModifiers::CONTROL)
            && (event.code == KeyCode::Char('w') || event.code == KeyCode::Backspace)
        {
            match app.download.form_field_idx {
                0 => { delete_last_word(&mut app.download.form_title); }
                1 => { delete_last_word(&mut app.download.form_artist); }
                2 => { delete_last_word(&mut app.download.form_album); }
                _ => {}
            }
            return true;
        }

        match event.code {
            KeyCode::Esc => {
                app.download.show_metadata_form = false;
            }
            KeyCode::Tab | KeyCode::Down => {
                app.download.form_field_idx = (app.download.form_field_idx + 1) % 7;
            }
            KeyCode::BackTab | KeyCode::Up => {
                app.download.form_field_idx = if app.download.form_field_idx == 0 { 6 } else { app.download.form_field_idx - 1 };
            }
            KeyCode::Enter => {
                if app.download.form_field_idx == 3 {
                    app.download.dir_picker = crate::browser::FileBrowser::new(&app.download.form_dir);
                    app.download.show_dir_picker = true;
                } else if app.download.form_field_idx == 4 {
                    app.download.form_field_idx = 5;
                } else if app.download.form_field_idx == 5 {
                    app.download.form_field_idx = 6;
                } else if app.download.form_field_idx < 6 {
                    app.download.form_field_idx += 1;
                } else {
                    app.start_metadata_download();
                }
            }
            KeyCode::Left => {
                if app.download.form_field_idx == 4 {
                    app.download.form_cover_mode = if app.download.form_cover_mode == "blur_pad" {
                        "center_crop".into()
                    } else {
                        "blur_pad".into()
                    };
                } else if app.download.form_field_idx == 5 {
                    app.download.form_lyrics_mode = if app.download.form_lyrics_mode == 0 { 2 } else { app.download.form_lyrics_mode - 1 };
                }
            }
            KeyCode::Right => {
                if app.download.form_field_idx == 4 {
                    app.download.form_cover_mode = if app.download.form_cover_mode == "blur_pad" {
                        "center_crop".into()
                    } else {
                        "blur_pad".into()
                    };
                } else if app.download.form_field_idx == 5 {
                    app.download.form_lyrics_mode = (app.download.form_lyrics_mode + 1) % 3;
                }
            }
            KeyCode::Backspace | KeyCode::Delete => {
                match app.download.form_field_idx {
                    0 => { app.download.form_title.pop(); }
                    1 => { app.download.form_artist.pop(); }
                    2 => { app.download.form_album.pop(); }
                    _ => {}
                }
            }
            KeyCode::Char(' ') => {
                if app.download.form_field_idx == 3 {
                    app.download.dir_picker = crate::browser::FileBrowser::new(&app.download.form_dir);
                    app.download.show_dir_picker = true;
                } else if app.download.form_field_idx == 4 {
                    app.download.form_cover_mode = if app.download.form_cover_mode == "blur_pad" {
                        "center_crop".into()
                    } else {
                        "blur_pad".into()
                    };
                } else if app.download.form_field_idx == 5 {
                    app.download.form_lyrics_mode = (app.download.form_lyrics_mode + 1) % 3;
                } else if app.download.form_field_idx == 6 {
                    app.start_metadata_download();
                } else {
                    match app.download.form_field_idx {
                        0 => { app.download.form_title.push(' '); }
                        1 => { app.download.form_artist.push(' '); }
                        2 => { app.download.form_album.push(' '); }
                        _ => {}
                    }
                }
            }
            KeyCode::Char(c) => {
                match app.download.form_field_idx {
                    0 => { app.download.form_title.push(c); }
                    1 => { app.download.form_artist.push(c); }
                    2 => { app.download.form_album.push(c); }
                    _ => {}
                }
            }
            _ => {}
        }
        return true;
    }

    // 2.5 Active download cancellation via Esc or Ctrl+C
    if app.download.is_downloading {
        if event.code == KeyCode::Esc || (event.modifiers.contains(KeyModifiers::CONTROL) && event.code == KeyCode::Char('c')) {
            app.downloader.cancel();
            app.download.is_downloading = false;
            app.download.last_error = Some("Download cancelled by user.".to_string());
            return true;
        }
    }

    // 3. Main Download View: Query input active
    if app.download.is_input_active {
        match event.code {
            KeyCode::Tab => {
                app.toggle_view();
            }
            KeyCode::Esc => {
                if !app.download.query.is_empty() {
                    app.download.query.clear();
                } else if app.download.last_error.is_some() || app.download.last_completed.is_some() {
                    app.download.last_error = None;
                    app.download.last_completed = None;
                } else if !app.download.search_results.is_empty() {
                    app.download.is_input_active = false;
                }
            }
            KeyCode::Enter => {
                app.on_enter();
            }
            KeyCode::Backspace => {
                app.download.query.pop();
            }
            KeyCode::Down => {
                if !app.download.search_results.is_empty() {
                    app.download.is_input_active = false;
                }
            }
            KeyCode::Char(c) => {
                app.download.query.push(c);
            }
            _ => {}
        }
        return true;
    }

    // 4. Navigating search results
    match event.code {
        KeyCode::Tab => {
            app.toggle_view();
            return true;
        }
        KeyCode::Esc => {
            app.download.last_error = None;
            app.download.last_completed = None;
            app.download.is_input_active = true;
            return true;
        }
        KeyCode::Char('/') | KeyCode::Char('i') => {
            app.download.is_input_active = true;
            return true;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if app.download.selected_result == 0 {
                app.download.is_input_active = true;
            } else {
                app.download.selected_result -= 1;
            }
            return true;
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if !app.download.search_results.is_empty() {
                app.download.selected_result = (app.download.selected_result + 1) % app.download.search_results.len();
            }
            return true;
        }
        KeyCode::Enter => {
            app.on_enter();
            return true;
        }
        KeyCode::Char('1') => { app.view_mode = ViewMode::Playlist; return true; }
        KeyCode::Char('2') => { app.view_mode = ViewMode::Browser; return true; }
        KeyCode::Char('3') => { app.view_mode = ViewMode::Download; return true; }
        KeyCode::Char('q') | KeyCode::Char('Q') => return false,
        KeyCode::Char('?') => { app.show_help = true; return true; }
        KeyCode::Char(' ') => { app.audio.toggle_pause(); return true; }
        KeyCode::Char('n') | KeyCode::Char('N') => { app.play_next_track(); return true; }
        KeyCode::Char('p') | KeyCode::Char('P') => { app.play_prev_track(); return true; }
        KeyCode::Left | KeyCode::Char('h') => { app.audio.seek_backward(5); return true; }
        KeyCode::Right | KeyCode::Char('l') => { app.audio.seek_forward(5); return true; }
        KeyCode::Char('+') | KeyCode::Char('=') => { app.audio.volume_up(); return true; }
        KeyCode::Char('-') | KeyCode::Char('_') => { app.audio.volume_down(); return true; }
        KeyCode::Char('m') | KeyCode::Char('M') => { app.audio.toggle_mute(); return true; }
        _ => {}
    }

    true
}

fn delete_last_word(s: &mut String) {
    while s.ends_with(' ') {
        s.pop();
    }
    while let Some(c) = s.chars().last() {
        if c == ' ' {
            break;
        }
        s.pop();
    }
    while s.ends_with(' ') {
        s.pop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delete_last_word() {
        let mut s = "Hello World Album".to_string();
        delete_last_word(&mut s);
        assert_eq!(s, "Hello World");
        delete_last_word(&mut s);
        assert_eq!(s, "Hello");
        delete_last_word(&mut s);
        assert_eq!(s, "");
    }
}
