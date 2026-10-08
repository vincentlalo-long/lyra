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
                KeyCode::Char('u') if event.modifiers.contains(KeyModifiers::CONTROL) => {
                    if let Some(t) = app.genre_tagger.as_mut() {
                        t.input.clear();
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

    // Dedicated handler for Download ViewMode (download plugin)
    #[cfg(feature = "download")]
    if app.view_mode == ViewMode::Download {
        return download::handle_download_key(app, event);
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

        // Toggle queue-loop (replays the whole queue; `r` cycle untouched).
        // Note: with CapsLock on, `l` (seek) arrives as `L`.
        KeyCode::Char('L') => app.toggle_queue_loop(),

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

        // Switch tabs (1..4 or Tab)
        KeyCode::Tab => app.toggle_view(),
        KeyCode::Char('1') => app.view_mode = ViewMode::Playlist,
        KeyCode::Char('2') => app.view_mode = ViewMode::Queue,
        KeyCode::Char('3') => app.view_mode = ViewMode::Browser,
        #[cfg(feature = "download")]
        KeyCode::Char('4') => app.view_mode = ViewMode::Download,
        #[cfg(not(feature = "download"))]
        KeyCode::Char('4') => app.view_mode = ViewMode::Plugins,
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
        KeyCode::Char('n') | KeyCode::Char('N') => app.play_next_track(),
        KeyCode::Char('P') => {
            if app.view_mode == ViewMode::Browser {
                app.play_browser_folder();
            } else {
                app.play_prev_track();
            }
        }
        KeyCode::Char('p') => app.play_prev_track(),

        // Scratchpad queue (ephemeral: never touches files on disk) / Plugin management
        KeyCode::Char('a') => app.enqueue_selected_back(),
        KeyCode::Char('A') => app.enqueue_selected_front(),
        KeyCode::Char('d') | KeyCode::Delete => {
            if app.view_mode == ViewMode::Playlist {
                app.remove_playlist_selected();
            } else if app.view_mode == ViewMode::Queue {
                app.delete_queue_selected();
            } else if app.view_mode == ViewMode::Plugins {
                app.remove_selected_plugin();
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

        // Volume control
        KeyCode::Char('+') | KeyCode::Char('=') => {
            app.audio.volume_up();
            if app.view_mode == ViewMode::Plugins {
                let pct = (app.audio.volume * 100.0).round() as u32;
                app.set_toast(format!("Volume: {pct}%"));
            }
        }
        KeyCode::Char('-') | KeyCode::Char('_') => {
            app.audio.volume_down();
            if app.view_mode == ViewMode::Plugins {
                let pct = (app.audio.volume * 100.0).round() as u32;
                app.set_toast(format!("Volume: {pct}%"));
            }
        }
        KeyCode::Char('m') | KeyCode::Char('M') => app.audio.toggle_mute(),

        _ => {}
    }

    true
}
