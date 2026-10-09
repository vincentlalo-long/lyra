use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use crate::app::{App, ViewMode};

#[cfg(feature = "download")]
mod download;

pub fn handle_key(app: &mut App, event: KeyEvent) -> bool {
    // If Help popup is open, handle scroll & dismissal
    if app.show_help {
        match event.code {
            KeyCode::Up | KeyCode::Char('k') => {
                app.help_scroll = app.help_scroll.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                app.help_scroll = app.help_scroll.saturating_add(1);
            }
            KeyCode::PageUp => {
                app.help_scroll = app.help_scroll.saturating_sub(4);
            }
            KeyCode::PageDown => {
                app.help_scroll = app.help_scroll.saturating_add(4);
            }
            KeyCode::Home | KeyCode::Char('g') => {
                app.help_scroll = 0;
            }
            KeyCode::End | KeyCode::Char('G') => {
                app.help_scroll = 100;
            }
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Char('?') | KeyCode::Enter => {
                app.show_help = false;
            }
            _ => {}
        }
        return true;
    }

    // Genre picker modal owns all keys while open (any view).
    #[cfg(feature = "genre")]
    if app.genre_picker.is_some() {
        match event.code {
            KeyCode::Esc => app.genre_picker = None,
            KeyCode::Up | KeyCode::Char('k') => {
                if let Some(p) = app.genre_picker.as_mut() {
                    p.move_up();
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let Some(p) = app.genre_picker.as_mut() {
                    p.move_down();
                }
            }
            KeyCode::Char(' ') | KeyCode::Enter => {
                let is_apply = app.genre_picker.as_ref().map(|p| p.is_apply_button_selected()).unwrap_or(false);
                if is_apply {
                    app.filter_by_picked_genre();
                } else if let Some(p) = app.genre_picker.as_mut() {
                    p.toggle_current();
                    p.move_down();
                }
            }
            KeyCode::Char('f') | KeyCode::Char('F') => app.filter_by_picked_genre(),
            KeyCode::Char('q') | KeyCode::Char('Q') => app.enqueue_picked_genre(),
            _ => {}
        }
        return true;
    }

    // Genre tagger / editor modal owns all keys while open
    #[cfg(feature = "genre")]
    if app.genre_tagger.is_some() {
        let is_focus_sug = app.genre_tagger.as_ref().map(|t| t.focus_suggestions).unwrap_or(false);
        if is_focus_sug {
            match event.code {
                KeyCode::Esc => app.genre_tagger = None,
                KeyCode::Tab | KeyCode::BackTab | KeyCode::Up | KeyCode::Char('k') => {
                    if let Some(t) = app.genre_tagger.as_mut() {
                        t.focus_suggestions = false;
                    }
                }
                KeyCode::Left | KeyCode::Char('h') => {
                    if let Some(t) = app.genre_tagger.as_mut() {
                        t.selected_suggestion = t.selected_suggestion.saturating_sub(1);
                    }
                }
                KeyCode::Right | KeyCode::Char('l') => {
                    if let Some(t) = app.genre_tagger.as_mut() {
                        let max = t.suggestions.len().saturating_sub(1);
                        t.selected_suggestion = (t.selected_suggestion + 1).min(max);
                    }
                }
                KeyCode::Enter | KeyCode::Char(' ') => {
                    app.append_suggestion_to_genre_tagger();
                }
                _ => {}
            }
        } else {
            let has_ctrl = event.modifiers.contains(KeyModifiers::CONTROL);
            let has_alt = event.modifiers.contains(KeyModifiers::ALT);
            if has_alt {
                return true;
            }
            if has_ctrl {
                match event.code {
                    KeyCode::Char('u') | KeyCode::Char('U') => {
                        if let Some(t) = app.genre_tagger.as_mut() {
                            t.input.clear();
                        }
                    }
                    KeyCode::Char('w') | KeyCode::Char('W') => {
                        if let Some(t) = app.genre_tagger.as_mut() {
                            delete_last_word(&mut t.input);
                        }
                    }
                    _ => {}
                }
                return true;
            }
            match event.code {
                KeyCode::Esc => app.genre_tagger = None,
                KeyCode::Tab | KeyCode::Down => {
                    if let Some(t) = app.genre_tagger.as_mut() {
                        t.focus_suggestions = true;
                    }
                }
                KeyCode::Enter => app.save_genre_tagger(),
                KeyCode::Backspace => {
                    if let Some(t) = app.genre_tagger.as_mut() {
                        t.input.pop();
                    }
                }
                KeyCode::Char(c) => {
                    if let Some(t) = app.genre_tagger.as_mut() {
                        t.input.push(c);
                    }
                }
                _ => {}
            }
        }
        return true;
    }

    // Scan result prompt modal owns all keys while active
    if app.scanner.pending_result.is_some() {
        match event.code {
            KeyCode::Enter => {
                if let Some(songs) = app.scanner.pending_result.take() {
                    let n = songs.len();
                    app.playlist.set_songs(songs);
                    app.view_mode = ViewMode::Playlist;
                    app.set_toast(format!("Playlist replaced with {n} songs"));
                }
            }
            KeyCode::Char('a') | KeyCode::Char('A') => {
                if let Some(songs) = app.scanner.pending_result.take() {
                    let n = songs.len();
                    app.playlist.append_songs(songs);
                    app.view_mode = ViewMode::Playlist;
                    app.set_toast(format!("Appended {n} songs to playlist"));
                }
            }
            KeyCode::Char('q') | KeyCode::Char('Q') => {
                if let Some(songs) = app.scanner.pending_result.take() {
                    let n = songs.len();
                    for s in songs {
                        app.queue.enqueue_back(s);
                    }
                    app.queue.persist();
                    app.set_toast(format!("Added {n} songs to queue"));
                }
            }
            KeyCode::Esc => {
                app.scanner.pending_result = None;
                app.set_toast("Scan results dismissed".to_string());
            }
            _ => {}
        }
        return true;
    }

    // Path editing modal in Plugins view
    if app.editing_path_index.is_some() {
        let has_ctrl = event.modifiers.contains(KeyModifiers::CONTROL);
        let has_alt = event.modifiers.contains(KeyModifiers::ALT);
        if has_alt {
            return true;
        }
        if has_ctrl {
            match event.code {
                KeyCode::Char('u') | KeyCode::Char('U') => app.editing_path_input.clear(),
                KeyCode::Char('w') | KeyCode::Char('W') => delete_last_word(&mut app.editing_path_input),
                _ => {}
            }
            return true;
        }
        match event.code {
            KeyCode::Esc => app.cancel_editing_path(),
            KeyCode::Enter => app.confirm_editing_path(),
            KeyCode::Backspace => {
                app.editing_path_input.pop();
            }
            KeyCode::Char(c) => {
                app.editing_path_input.push(c);
            }
            _ => {}
        }
        return true;
    }

    // Community plugin store modal
    if app.show_plugin_store {
        match event.code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                app.show_plugin_store = false;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                app.store_selected = app.store_selected.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if !app.downloadable_plugins.is_empty() {
                    app.store_selected = (app.store_selected + 1).min(app.downloadable_plugins.len() - 1);
                }
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                app.install_store_plugin();
            }
            _ => {}
        }
        return true;
    }

    // Dedicated handler for Extensions ViewMode (runs active interactive extension)
    #[cfg(feature = "download")]
    if app.view_mode == ViewMode::Extensions {
        let is_download_enabled = app
            .plugins
            .iter()
            .find(|p| p.id == "download")
            .map(|p| p.enabled && !p.is_removed)
            .unwrap_or(true);
        if is_download_enabled {
            return download::handle_download_key(app, event);
        }
    }

    // Browser action modal (new album, rename album, edit track ID3, move track) owns all keys while active
    if app.view_mode == ViewMode::Browser && app.browser_modal.is_some() {
        return app.handle_browser_modal_key(event);
    }

    // Search Mode (Playlist / Browser)
    if app.is_searching {
        let has_ctrl = event.modifiers.contains(KeyModifiers::CONTROL);
        let has_alt = event.modifiers.contains(KeyModifiers::ALT);

        if has_ctrl {
            match event.code {
                KeyCode::Char('u') | KeyCode::Char('U') => {
                    app.search_query.clear();
                }
                KeyCode::Char('w') | KeyCode::Char('W') => {
                    delete_last_word(&mut app.search_query);
                }
                KeyCode::Char('c') | KeyCode::Char('C') => {
                    app.is_searching = false;
                    app.search_query.clear();
                }
                _ => {}
            }
            return true;
        }

        if has_alt {
            return true;
        }

        match event.code {
            KeyCode::Esc => {
                app.is_searching = false;
                app.search_query.clear();
            }
            KeyCode::Enter => {
                app.on_enter();
            }
            KeyCode::Backspace => {
                app.search_query.pop();
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

    let has_ctrl = event.modifiers.contains(KeyModifiers::CONTROL);
    let has_alt = event.modifiers.contains(KeyModifiers::ALT);

    // Ctrl+C quits application cleanly from normal mode
    if has_ctrl && (event.code == KeyCode::Char('c') || event.code == KeyCode::Char('C')) {
        return false;
    }

    // Replay current track (Ctrl+r or capital R)
    if (has_ctrl && (event.code == KeyCode::Char('r') || event.code == KeyCode::Char('R')))
        || (!has_ctrl && !has_alt && event.code == KeyCode::Char('R'))
    {
        app.replay_current_track();
        return true;
    }

    // If Alt or Ctrl is held in Normal Mode, do not trigger single-key shortcuts
    // (e.g. Alt+Z is Unikey toggle, Ctrl+Z, Ctrl+S, etc.)
    if has_ctrl || has_alt {
        return true;
    }

    // Normal Mode
    match event.code {
        // Quit, clear filter, or cancel replace (Esc never quits mid-replace).
        KeyCode::Char('q') | KeyCode::Char('Q') => return false,
        KeyCode::Esc => {
            if app.pending_replace.is_some() {
                app.cancel_replace();
            } else if !app.search_query.is_empty() {
                app.search_query.clear();
            } else if {
                // Genre filter (if the plugin is compiled in).
                #[cfg(feature = "genre")]
                {
                    app.clear_genre_filter()
                }
                #[cfg(not(feature = "genre"))]
                {
                    false
                }
            } {
                // Toast set inside; stay in the app.
            } else {
                return false;
            }
        }

        // Toggle Repeat Mode (r: Playlist -> Track -> Off) or Restore plugin in Plugins view
        KeyCode::Char('r') => {
            if app.view_mode == ViewMode::Plugins {
                app.restore_selected_plugin();
            } else {
                app.toggle_repeat();
            }
        }

        // Toggle queue-loop or in Browser: Set current folder as permanent Music Library path
        KeyCode::Char('L') => {
            if app.view_mode == ViewMode::Browser {
                app.browser_set_current_as_music_folder();
            } else {
                app.toggle_queue_loop();
            }
        }

        // Toggle Lyrics display (v / y: Studio mode <-> Karaoke mode)
        KeyCode::Char('v') | KeyCode::Char('V') | KeyCode::Char('y') | KeyCode::Char('Y') => app.toggle_lyrics(),

        // Toggle Help popup
        KeyCode::Char('?') => {
            app.show_help = true;
            app.help_scroll = 0;
        }

        // Search in Playlist/Browser
        KeyCode::Char('/') => {
            app.is_searching = true;
            app.search_query.clear();
        }

        // Switch tabs (1..5 or Tab)
        KeyCode::Tab => app.toggle_view(),
        KeyCode::Char('1') => app.view_mode = ViewMode::Playlist,
        KeyCode::Char('2') => app.view_mode = ViewMode::Queue,
        KeyCode::Char('3') => app.view_mode = ViewMode::Browser,
        KeyCode::Char('4') => app.view_mode = ViewMode::Extensions,
        KeyCode::Char('5') => app.view_mode = ViewMode::Plugins,
        KeyCode::Char('i') | KeyCode::Char('I') => {
            if app.view_mode == ViewMode::Browser {
                app.import_browser_folder_into_playlist();
            } else if app.view_mode == ViewMode::Playlist {
                app.view_mode = ViewMode::Browser;
                app.set_toast("󰉋 Browser: Navigate to album and press 'i' to import, or Enter to play".to_string());
            }
        }
        KeyCode::Char('.') => app.browser.toggle_hidden(),
        KeyCode::Char('s') | KeyCode::Char('S') => app.start_scan(),

        // Jump to currently playing track (inspired by rmpc 'C')
        KeyCode::Char('C') => {
            if let Some(playing_idx) = app.playlist.playing_index {
                app.playlist.selected = playing_idx;
                app.playlist.state.select(Some(playing_idx));
            }
        }

        // Navigation
        KeyCode::Up | KeyCode::Char('k') => app.previous(),
        KeyCode::Down | KeyCode::Char('j') => app.next(),
        KeyCode::PageUp => app.page_up(),
        KeyCode::PageDown => app.page_down(),
        KeyCode::Home | KeyCode::Char('g') => app.go_to_top(),
        KeyCode::End | KeyCode::Char('G') => app.go_to_bottom(),
        KeyCode::Enter => app.on_enter(),
        KeyCode::Backspace => {
            if app.view_mode == ViewMode::Browser {
                app.browser_go_parent();
            }
        }

        // Playback control
        KeyCode::Char(' ') => {
            if app.view_mode == ViewMode::Plugins {
                app.toggle_selected_plugin();
            } else {
                app.audio.toggle_pause();
            }
        }
        KeyCode::Char('n') => app.play_next_track(),
        KeyCode::Char('N') => {
            if app.view_mode == ViewMode::Browser {
                app.browser_create_album_prompt();
            } else {
                app.play_next_track();
            }
        }
        KeyCode::Char('P') => {
            if app.view_mode == ViewMode::Browser {
                app.play_browser_folder();
            } else {
                app.play_prev_track();
            }
        }
        KeyCode::Char('p') => app.play_prev_track(),

        // Scratchpad queue (ephemeral: never touches files on disk) / Plugin management / Browser delete
        KeyCode::Char('a') => app.enqueue_selected_back(),
        KeyCode::Char('A') => app.enqueue_selected_front(),
        KeyCode::Char('d') | KeyCode::Delete => {
            if app.view_mode == ViewMode::Playlist {
                app.remove_playlist_selected();
            } else if app.view_mode == ViewMode::Queue {
                app.delete_queue_selected();
            } else if app.view_mode == ViewMode::Plugins {
                app.remove_selected_plugin();
            } else if app.view_mode == ViewMode::Browser {
                app.browser_delete_selected();
            }
        }
        KeyCode::Char('D') => {
            if app.view_mode == ViewMode::Plugins {
                app.show_plugin_store = true;
                app.store_selected = 0;
                app.set_toast("Opened Plugin Store (UI Preview)".to_string());
            }
        }
        KeyCode::Char('e') => {
            if app.view_mode == ViewMode::Plugins {
                app.start_editing_selected_path();
            } else if app.view_mode == ViewMode::Browser {
                if app.pending_replace.is_some() {
                    app.begin_replace();
                } else {
                    app.browser_edit_prompt();
                }
            } else {
                app.begin_replace();
            }
        }
        KeyCode::Char('c') => app.clear_queue(),
        KeyCode::Char('z') => app.shuffle_queue(),
        KeyCode::Char('w') => { app.save_daylist(); }
        KeyCode::Char('o') => { app.load_daylist(); }

        // Genre picker (f: filter/queue) & Genre tagger (t: tag/edit genre)
        #[cfg(feature = "genre")]
        KeyCode::Char('f') | KeyCode::Char('F') => app.open_genre_picker(),
        #[cfg(feature = "genre")]
        KeyCode::Char('t') | KeyCode::Char('T') => app.open_genre_tagger(),

        // Seek forward / backward (5 seconds), or adjust setting in Plugins, or navigate in Browser
        KeyCode::Left | KeyCode::Char('h') => {
            if app.view_mode == ViewMode::Plugins {
                app.adjust_selected_plugin(false);
            } else if app.view_mode == ViewMode::Browser {
                app.browser_go_parent();
            } else {
                app.audio.seek_backward(5);
            }
        }
        KeyCode::Right | KeyCode::Char('l') => {
            if app.view_mode == ViewMode::Plugins {
                app.adjust_selected_plugin(true);
            } else if app.view_mode == ViewMode::Browser {
                app.on_enter();
            } else {
                app.audio.seek_forward(5);
            }
        }
        KeyCode::Char('[') => {
            app.audio.seek_backward(5);
        }
        KeyCode::Char(']') => {
            app.audio.seek_forward(5);
        }

        // Volume control / Move Track in Browser
        KeyCode::Char('+') | KeyCode::Char('=') => {
            if app.view_mode == ViewMode::Browser {
                app.browser_create_album_prompt();
            } else {
                app.audio.volume_up();
                if app.view_mode == ViewMode::Plugins {
                    let pct = (app.audio.volume * 100.0).round() as u32;
                    app.set_toast(format!("Volume: {pct}%"));
                }
            }
        }
        KeyCode::Char('-') | KeyCode::Char('_') => {
            app.audio.volume_down();
            if app.view_mode == ViewMode::Plugins {
                let pct = (app.audio.volume * 100.0).round() as u32;
                app.set_toast(format!("Volume: {pct}%"));
            }
        }
        KeyCode::Char('m') | KeyCode::Char('M') => {
            if app.view_mode == ViewMode::Browser {
                app.browser_move_track_prompt();
            } else {
                app.audio.toggle_mute();
            }
        }
        _ => {}
    }

    true
}

pub(crate) fn delete_last_word(s: &mut String) {
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
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[test]
    fn test_five_fixed_tabs_key_switching() {
        let temp_dir = std::env::temp_dir();
        if let Ok(mut app) = App::new(&temp_dir) {
            assert_eq!(app.view_mode, ViewMode::Playlist);

            // Test 1..5 keys
            let key_2 = KeyEvent::new(KeyCode::Char('2'), KeyModifiers::NONE);
            handle_key(&mut app, key_2);
            assert_eq!(app.view_mode, ViewMode::Queue);

            let key_3 = KeyEvent::new(KeyCode::Char('3'), KeyModifiers::NONE);
            handle_key(&mut app, key_3);
            assert_eq!(app.view_mode, ViewMode::Browser);

            let key_4 = KeyEvent::new(KeyCode::Char('4'), KeyModifiers::NONE);
            handle_key(&mut app, key_4);
            assert_eq!(app.view_mode, ViewMode::Extensions);

            let key_5 = KeyEvent::new(KeyCode::Char('5'), KeyModifiers::NONE);
            handle_key(&mut app, key_5);
            assert_eq!(app.view_mode, ViewMode::Plugins);

            let key_1 = KeyEvent::new(KeyCode::Char('1'), KeyModifiers::NONE);
            handle_key(&mut app, key_1);
            assert_eq!(app.view_mode, ViewMode::Playlist);

            // Test Tab key cycle
            let key_tab = KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE);
            handle_key(&mut app, key_tab);
            assert_eq!(app.view_mode, ViewMode::Queue);

            handle_key(&mut app, key_tab);
            assert_eq!(app.view_mode, ViewMode::Browser);

            handle_key(&mut app, key_tab);
            assert_eq!(app.view_mode, ViewMode::Extensions);

            handle_key(&mut app, key_tab);
            assert_eq!(app.view_mode, ViewMode::Plugins);

            handle_key(&mut app, key_tab);
            assert_eq!(app.view_mode, ViewMode::Playlist);
        }
    }

    #[test]
    fn test_search_mode_ime_unikey_resilience() {
        let temp_dir = std::env::temp_dir();
        if let Ok(mut app) = App::new(&temp_dir) {
            // Press '/' to start search
            let key_slash = KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE);
            handle_key(&mut app, key_slash);
            assert!(app.is_searching);
            assert_eq!(app.search_query, "");

            // Unikey typing 'd' -> 'd'
            let key_d = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE);
            handle_key(&mut app, key_d);
            assert_eq!(app.search_query, "d");

            // Unikey typing second 'd' (to form 'đ'): IME sends Backspace then 'đ'
            let key_backspace = KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE);
            handle_key(&mut app, key_backspace);
            // Crucial: Empty search_query must NOT close search mode!
            assert_eq!(app.search_query, "");
            assert!(app.is_searching, "Search mode must stay active even when backspace empties the query during IME composition");

            // IME sends 'đ'
            let key_d_bar = KeyEvent::new(KeyCode::Char('đ'), KeyModifiers::NONE);
            handle_key(&mut app, key_d_bar);
            assert_eq!(app.search_query, "đ");
            assert!(app.is_searching);

            // User toggles Unikey with Alt+Z: must NOT type 'z' into search query
            let key_alt_z = KeyEvent::new(KeyCode::Char('z'), KeyModifiers::ALT);
            handle_key(&mut app, key_alt_z);
            assert_eq!(app.search_query, "đ", "Alt+Z IME toggle must not append 'z' to query");
            assert!(app.is_searching);

            // Type additional Vietnamese characters
            for c in " bài hát".chars() {
                handle_key(&mut app, KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
            }
            assert_eq!(app.search_query, "đ bài hát");

            // Test Ctrl+W (delete last word)
            let key_ctrl_w = KeyEvent::new(KeyCode::Char('w'), KeyModifiers::CONTROL);
            handle_key(&mut app, key_ctrl_w);
            assert_eq!(app.search_query, "đ bài");

            // Test Ctrl+U (clear line)
            let key_ctrl_u = KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL);
            handle_key(&mut app, key_ctrl_u);
            assert_eq!(app.search_query, "");
            assert!(app.is_searching);

            // Esc exits search mode
            let key_esc = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
            handle_key(&mut app, key_esc);
            assert!(!app.is_searching);
        }
    }

    #[test]
    fn test_normal_mode_modifier_protection() {
        let temp_dir = std::env::temp_dir();
        if let Ok(mut app) = App::new(&temp_dir) {
            assert_eq!(app.view_mode, ViewMode::Playlist);

            // Alt+Z (Unikey toggle shortcut): must NOT shuffle queue or trigger 'z'
            let initial_toast = app.toast_text().map(|s| s.to_string());
            let key_alt_z = KeyEvent::new(KeyCode::Char('z'), KeyModifiers::ALT);
            let cont = handle_key(&mut app, key_alt_z);
            assert!(cont);
            assert_eq!(app.toast_text().map(|s| s.to_string()), initial_toast, "Alt+Z must not trigger shuffle_queue");

            // Ctrl+S: must NOT trigger library scan
            let key_ctrl_s = KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL);
            let cont_s = handle_key(&mut app, key_ctrl_s);
            assert!(cont_s);
            assert!(app.scanner.pending_result.is_none());

            // Ctrl+C: must cleanly return false to quit
            let key_ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
            let cont_c = handle_key(&mut app, key_ctrl_c);
            assert!(!cont_c, "Ctrl+C in normal mode must cleanly request exit");
        }
    }

    #[test]
    fn test_browser_modal_capital_r_and_unikey_not_intercepted() {
        use crate::browser::BrowserActionModal;
        let temp_dir = std::env::temp_dir();
        if let Ok(mut app) = App::new(&temp_dir) {
            app.view_mode = ViewMode::Browser;
            app.browser_modal = Some(BrowserActionModal::NewAlbum { input: String::new() });

            // Typing capital 'R' inside browser modal must NOT trigger replay_current_track
            let key_cap_r = KeyEvent::new(KeyCode::Char('R'), KeyModifiers::SHIFT);
            let handled = handle_key(&mut app, key_cap_r);
            assert!(handled);

            match &app.browser_modal {
                Some(BrowserActionModal::NewAlbum { input }) => {
                    assert_eq!(input, "R", "Capital 'R' must be typed into the modal input field");
                }
                _ => panic!("Expected NewAlbum modal to remain active"),
            }

            // Alt+Z toggle inside browser modal must NOT append 'z'
            let key_alt_z = KeyEvent::new(KeyCode::Char('z'), KeyModifiers::ALT);
            handle_key(&mut app, key_alt_z);
            match &app.browser_modal {
                Some(BrowserActionModal::NewAlbum { input }) => {
                    assert_eq!(input, "R", "Alt+Z must not append 'z' inside modal");
                }
                _ => panic!("Expected NewAlbum modal to remain active"),
            }
        }
    }
}
