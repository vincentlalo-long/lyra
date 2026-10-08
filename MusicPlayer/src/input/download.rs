use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use crate::app::{App, ViewMode};

pub(super) fn handle_download_key(app: &mut App, event: KeyEvent) -> bool {
    // 0. Cover File Picker Popup (pick local image from disk)
    if app.download.show_cover_file_picker {
        match event.code {
            KeyCode::Esc => {
                app.download.show_cover_file_picker = false;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                app.download.cover_file_picker.previous("");
            }
            KeyCode::Down | KeyCode::Char('j') => {
                app.download.cover_file_picker.next("");
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                if let Some(selected_file) = app.download.cover_file_picker.enter() {
                    app.download.form_custom_cover = Some(selected_file.to_string_lossy().to_string());
                    app.download.selected_cover = None;
                    app.download.show_cover_file_picker = false;
                    app.download.show_cover_picker_modal = false;
                    app.download.update_cover_preview();
                }
            }
            _ => {}
        }
        return true;
    }

    // 0.5 Cover Search & Live Preview Modal
    if app.download.show_cover_picker_modal {
        if app.download.cover_picker_input_active {
            match event.code {
                KeyCode::Esc => {
                    app.download.cover_picker_input_active = false;
                }
                KeyCode::Down | KeyCode::Tab => {
                    app.download.cover_picker_input_active = false;
                }
                KeyCode::Enter => {
                    let q = app.download.cover_picker_search.trim().to_string();
                    if !q.is_empty() {
                        app.download.cover_candidates.clear();
                        app.download.selected_cover = None;
                        app.download.cover_preview_art = None;
                        app.download.cover_preview_path = None;
                        app.download.cover_picker_selected = 0;
                        app.downloader.fetch_covers_query(
                            app.download.form_artist.clone(),
                            app.download.form_title.clone(),
                            Some(q),
                            Some("all".to_string()),
                        );
                    }
                    app.download.cover_picker_input_active = false;
                }
                KeyCode::Backspace => {
                    app.download.cover_picker_search.pop();
                }
                KeyCode::Char('u') if event.modifiers.contains(KeyModifiers::CONTROL) => {
                    app.download.cover_picker_search.clear();
                }
                KeyCode::Char('w') if event.modifiers.contains(KeyModifiers::CONTROL) => {
                    let trimmed = app.download.cover_picker_search.trim_end();
                    if let Some(idx) = trimmed.rfind(' ') {
                        app.download.cover_picker_search.truncate(idx);
                    } else {
                        app.download.cover_picker_search.clear();
                    }
                }
                KeyCode::Char(c) => {
                    app.download.cover_picker_search.push(c);
                }
                _ => {}
            }
            return true;
        }

        match event.code {
            KeyCode::Esc => {
                app.download.show_cover_picker_modal = false;
            }
            KeyCode::Tab | KeyCode::Char('/') => {
                app.download.cover_picker_input_active = true;
            }
            KeyCode::Char('b') | KeyCode::Char('B') => {
                app.download.cover_file_picker = crate::browser::FileBrowser::for_images(&app.download.form_dir);
                app.download.show_cover_file_picker = true;
            }
            KeyCode::Char('c') | KeyCode::Char('C') | KeyCode::Delete => {
                app.download.form_custom_cover = None;
                app.download.selected_cover = None;
                app.download.update_cover_preview();
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if app.download.cover_picker_selected == 0 {
                    app.download.cover_picker_input_active = true;
                } else {
                    app.download.cover_picker_selected -= 1;
                    if app.download.cover_picker_selected > 0 {
                        app.download.selected_cover = Some(app.download.cover_picker_selected - 1);
                        app.download.form_custom_cover = None;
                    }
                    app.download.update_cover_preview();
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let total = 1 + app.download.cover_candidates.len();
                if total > 0 {
                    app.download.cover_picker_selected = (app.download.cover_picker_selected + 1) % total;
                    if app.download.cover_picker_selected > 0 {
                        app.download.selected_cover = Some(app.download.cover_picker_selected - 1);
                        app.download.form_custom_cover = None;
                    }
                    app.download.update_cover_preview();
                }
            }
            KeyCode::Enter => {
                if app.download.cover_picker_selected == 0 {
                    app.download.cover_file_picker = crate::browser::FileBrowser::for_images(&app.download.form_dir);
                    app.download.show_cover_file_picker = true;
                } else {
                    app.download.selected_cover = Some(app.download.cover_picker_selected - 1);
                    app.download.form_custom_cover = None;
                    app.download.show_cover_picker_modal = false;
                }
            }
            KeyCode::Backspace => {
                app.download.cover_picker_input_active = true;
                app.download.cover_picker_search.pop();
            }
            KeyCode::Char(c) if !matches!(c, ' ' | '\t' | '\r' | '\n') => {
                app.download.cover_picker_input_active = true;
                app.download.cover_picker_search.push(c);
            }
            _ => {}
        }
        return true;
    }

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
                3 => { app.download.form_genre.clear(); }
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
                3 => { delete_last_word(&mut app.download.form_genre); }
                _ => {}
            }
            return true;
        }

        match event.code {
            KeyCode::Esc => {
                app.download.show_metadata_form = false;
            }
            KeyCode::Tab | KeyCode::Down => {
                app.download.form_field_idx = (app.download.form_field_idx + 1) % 8;
            }
            KeyCode::BackTab | KeyCode::Up => {
                app.download.form_field_idx = if app.download.form_field_idx == 0 { 7 } else { app.download.form_field_idx - 1 };
            }
            KeyCode::Enter => {
                if app.download.form_field_idx == 4 {
                    app.download.dir_picker = crate::browser::FileBrowser::new(&app.download.form_dir);
                    app.download.show_dir_picker = true;
                } else if app.download.form_field_idx == 5 {
                    app.download.cover_picker_selected = match app.download.selected_cover {
                        Some(i) => i + 1,
                        None => if app.download.form_custom_cover.is_some() { 0 } else { 1 },
                    };
                    app.download.cover_picker_input_active = true;
                    app.download.update_cover_preview();
                    app.download.show_cover_picker_modal = true;
                } else if app.download.form_field_idx == 6 {
                    app.download.form_field_idx = 7;
                } else if app.download.form_field_idx < 7 {
                    app.download.form_field_idx += 1;
                } else {
                    app.start_metadata_download();
                }
            }
            KeyCode::Left => {
                if app.download.form_field_idx == 5 {
                    cycle_cover(app, -1);
                } else if app.download.form_field_idx == 6 {
                    app.download.form_lyrics_mode = if app.download.form_lyrics_mode == 0 { 2 } else { app.download.form_lyrics_mode - 1 };
                }
            }
            KeyCode::Right => {
                if app.download.form_field_idx == 5 {
                    cycle_cover(app, 1);
                } else if app.download.form_field_idx == 6 {
                    app.download.form_lyrics_mode = (app.download.form_lyrics_mode + 1) % 3;
                }
            }
            KeyCode::Backspace | KeyCode::Delete => {
                match app.download.form_field_idx {
                    0 => { app.download.form_title.pop(); }
                    1 => { app.download.form_artist.pop(); }
                    2 => { app.download.form_album.pop(); }
                    3 => { app.download.form_genre.pop(); }
                    5 => {
                        app.download.form_custom_cover = None;
                        app.download.selected_cover = None;
                        app.download.update_cover_preview();
                    }
                    _ => {}
                }
            }
            KeyCode::Char(' ') => {
                if app.download.form_field_idx == 4 {
                    app.download.dir_picker = crate::browser::FileBrowser::new(&app.download.form_dir);
                    app.download.show_dir_picker = true;
                } else if app.download.form_field_idx == 5 {
                    cycle_cover(app, 1);
                } else if app.download.form_field_idx == 6 {
                    app.download.form_lyrics_mode = (app.download.form_lyrics_mode + 1) % 3;
                } else if app.download.form_field_idx == 7 {
                    app.start_metadata_download();
                } else {
                    match app.download.form_field_idx {
                        0 => { app.download.form_title.push(' '); }
                        1 => { app.download.form_artist.push(' '); }
                        2 => { app.download.form_album.push(' '); }
                        3 => { app.download.form_genre.push(' '); }
                        _ => {}
                    }
                }
            }
            KeyCode::Char(c) => {
                if app.download.form_field_idx == 5 && (c == 'p' || c == 'P' || c == 'v' || c == 'V') {
                    app.download.cover_picker_selected = match app.download.selected_cover {
                        Some(i) => i + 1,
                        None => if app.download.form_custom_cover.is_some() { 0 } else { 1 },
                    };
                    app.download.cover_picker_input_active = true;
                    app.download.update_cover_preview();
                    app.download.show_cover_picker_modal = true;
                } else if app.download.form_field_idx == 5 && (c == 'b' || c == 'B') {
                    app.download.cover_file_picker = crate::browser::FileBrowser::for_images(&app.download.form_dir);
                    app.download.show_cover_file_picker = true;
                } else {
                    match app.download.form_field_idx {
                        0 => { app.download.form_title.push(c); }
                        1 => { app.download.form_artist.push(c); }
                        2 => { app.download.form_album.push(c); }
                        3 => { app.download.form_genre.push(c); }
                        _ => {}
                    }
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
                app.download.is_input_active = false;
                app.toggle_view();
            }
            KeyCode::Esc => {
                app.download.is_input_active = false;
                app.download.last_error = None;
                app.download.last_completed = None;
            }
            KeyCode::Enter => {
                app.on_enter();
            }
            KeyCode::Backspace => {
                app.download.query.pop();
            }
            KeyCode::Char('u') if event.modifiers.contains(KeyModifiers::CONTROL) => {
                app.download.query.clear();
            }
            KeyCode::Char('w') if event.modifiers.contains(KeyModifiers::CONTROL) => {
                delete_last_word(&mut app.download.query);
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

    // 4. Navigating search results & Normal Mode
    match event.code {
        KeyCode::Tab => {
            app.toggle_view();
            return true;
        }
        KeyCode::Esc => {
            app.download.last_error = None;
            app.download.last_completed = None;
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
            if app.download.search_results.is_empty() {
                app.download.is_input_active = true;
            } else {
                app.on_enter();
            }
            return true;
        }
        KeyCode::Char('e') | KeyCode::Char('E') => {
            app.expand_download_search();
            return true;
        }
        KeyCode::Char('1') => { app.view_mode = ViewMode::Playlist; return true; }
        KeyCode::Char('2') => { app.view_mode = ViewMode::Queue; return true; }
        KeyCode::Char('3') => { app.view_mode = ViewMode::Browser; return true; }
        KeyCode::Char('4') => { app.view_mode = ViewMode::Download; return true; }
        KeyCode::Char('5') => { app.view_mode = ViewMode::Plugins; return true; }
        KeyCode::Char('q') | KeyCode::Char('Q') => return false,
        KeyCode::Char('?') => { app.show_help = true; app.help_scroll = 0; return true; }
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

fn cycle_cover(app: &mut App, dir: i32) {
    app.download.form_custom_cover = None;
    let n = app.download.cover_candidates.len();
    if n == 0 {
        return;
    }
    // States: None (Auto best) -> 0 -> 1 ... -> n-1 -> None.
    let total = n as i32 + 1;
    let cur = match app.download.selected_cover {
        None => 0,
        Some(i) => i as i32 + 1,
    };
    let next = (cur + dir).rem_euclid(total);
    app.download.selected_cover = if next == 0 { None } else { Some(next as usize - 1) };
    app.download.update_cover_preview();
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

    #[test]
    fn test_download_mode_tab_switching_and_esc() {
        let temp_dir = std::env::temp_dir();
        if let Ok(mut app) = App::new(&temp_dir) {
            app.view_mode = ViewMode::Download;
            assert!(!app.download.is_input_active);

            // In normal mode, pressing '1' switches to Playlist view
            let key_1 = KeyEvent::new(KeyCode::Char('1'), KeyModifiers::NONE);
            handle_download_key(&mut app, key_1);
            assert_eq!(app.view_mode, ViewMode::Playlist);

            // Switch back to Download view
            app.view_mode = ViewMode::Download;
            assert!(!app.download.is_input_active);

            // Press '/' to focus search input
            let key_slash = KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE);
            handle_download_key(&mut app, key_slash);
            assert!(app.download.is_input_active);

            // Type some query
            let key_a = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
            handle_download_key(&mut app, key_a);
            assert_eq!(app.download.query, "a");

            // Press Esc to unfocus search input
            let key_esc = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
            handle_download_key(&mut app, key_esc);
            assert!(!app.download.is_input_active);

            // Now pressing '5' switches to Plugins view
            let key_5 = KeyEvent::new(KeyCode::Char('5'), KeyModifiers::NONE);
            handle_download_key(&mut app, key_5);
            assert_eq!(app.view_mode, ViewMode::Plugins);
        }
    }
}
