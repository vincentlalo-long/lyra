use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::App;
use crate::browser::BrowserActionModal;

#[test]
fn test_browser_create_album() {
    let temp_dir = std::env::temp_dir().join("lyra_test_create_album");
    let _ = std::fs::remove_dir_all(&temp_dir);
    let _ = std::fs::create_dir_all(&temp_dir);

    if let Ok(mut app) = App::new(&temp_dir) {
        app.browser.current_dir = temp_dir.clone();
        app.browser.refresh();

        app.browser_create_album_prompt();
        assert!(matches!(
            app.browser_modal,
            Some(BrowserActionModal::NewAlbum { .. })
        ));

        // Type "Summer 2026"
        for c in "Summer 2026".chars() {
            app.handle_browser_modal_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
        }
        // Press Enter
        app.handle_browser_modal_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        assert!(app.browser_modal.is_none());
        assert!(temp_dir.join("Summer 2026").is_dir());
    }

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_browser_modal_caret_edits_mid_text() {
    let temp_dir = std::env::temp_dir().join("lyra_test_modal_caret");
    let _ = std::fs::remove_dir_all(&temp_dir);
    let _ = std::fs::create_dir_all(&temp_dir);

    if let Ok(mut app) = App::new(&temp_dir) {
        app.browser.current_dir = temp_dir.clone();
        app.browser.refresh();

        app.browser_create_album_prompt();
        for c in "abcd".chars() {
            app.handle_browser_modal_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
        }
        // Caret at end; move left twice and insert mid-text.
        app.handle_browser_modal_key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));
        app.handle_browser_modal_key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));
        assert_eq!(app.browser_caret, 2);
        app.handle_browser_modal_key(KeyEvent::new(KeyCode::Char('X'), KeyModifiers::NONE));
        // Backspace removes the inserted char.
        app.handle_browser_modal_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
        // Delete removes the char at the caret ('c').
        app.handle_browser_modal_key(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE));
        // Home + type at front, End + type at back.
        app.handle_browser_modal_key(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE));
        app.handle_browser_modal_key(KeyEvent::new(KeyCode::Char('Z'), KeyModifiers::NONE));
        app.handle_browser_modal_key(KeyEvent::new(KeyCode::End, KeyModifiers::NONE));
        app.handle_browser_modal_key(KeyEvent::new(KeyCode::Char('Y'), KeyModifiers::NONE));

        match &app.browser_modal {
            Some(BrowserActionModal::NewAlbum { input }) => {
                assert_eq!(input, "ZabdY");
            }
            other => panic!("expected NewAlbum modal, got {other:?}"),
        }
        app.handle_browser_modal_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(temp_dir.join("ZabdY").is_dir());
    }

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_browser_delete_empty_album_and_protect_non_empty() {
    let temp_dir = std::env::temp_dir().join("lyra_test_delete_album");
    let _ = std::fs::remove_dir_all(&temp_dir);
    let _ = std::fs::create_dir_all(&temp_dir);

    let empty_album = temp_dir.join("EmptyAlbum");
    let non_empty_album = temp_dir.join("NonEmptyAlbum");
    let _ = std::fs::create_dir_all(&empty_album);
    let _ = std::fs::create_dir_all(&non_empty_album);
    let _ = std::fs::write(non_empty_album.join("song.mp3"), b"audio data");

    if let Ok(mut app) = App::new(&temp_dir) {
        app.browser.current_dir = temp_dir.clone();
        app.browser.refresh();

        // Select EmptyAlbum
        if let Some(pos) = app.browser.entries.iter().position(|p| p == &empty_album) {
            app.browser.selected = pos;
            app.browser_delete_selected();
            assert!(!empty_album.exists());
        }

        // Select NonEmptyAlbum
        app.browser.refresh();
        if let Some(pos) = app
            .browser
            .entries
            .iter()
            .position(|p| p == &non_empty_album)
        {
            app.browser.selected = pos;
            app.browser_delete_selected();
            // Must NOT be deleted!
            assert!(non_empty_album.exists());
            assert!(non_empty_album.join("song.mp3").exists());
        }
    }

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_browser_move_track_and_update_id3() {
    let temp_dir = std::env::temp_dir().join("lyra_test_move_track");
    let _ = std::fs::remove_dir_all(&temp_dir);
    let _ = std::fs::create_dir_all(&temp_dir);

    let source_file = temp_dir.join("track1.mp3");
    let target_album = temp_dir.join("TargetAlbum");
    let _ = std::fs::create_dir_all(&target_album);
    let _ = std::fs::write(&source_file, b"test audio content");

    if let Ok(mut app) = App::new(&temp_dir) {
        app.browser.current_dir = temp_dir.clone();
        app.browser.refresh();

        app.execute_move_track(source_file.clone(), &target_album, "TargetAlbum");

        assert!(!source_file.exists());
        assert!(target_album.join("track1.mp3").exists());
    }

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_browser_set_current_as_music_folder() {
    let temp_dir = std::env::temp_dir().join("lyra_test_set_lib");
    let sub_album = temp_dir.join("MyNewAlbum");
    let _ = std::fs::create_dir_all(&sub_album);
    let song = sub_album.join("test.mp3");
    let _ = std::fs::write(&song, b"mp3");

    if let Ok(mut app) = App::new(&temp_dir) {
        app.browser.current_dir = sub_album.clone();
        app.browser_set_current_as_music_folder();

        assert_eq!(app.music_folder, sub_album);
        assert_eq!(app.playlist.songs.len(), 1);
        assert_eq!(app.playlist.songs[0], song);
    }

    let _ = std::fs::remove_dir_all(&temp_dir);
}
