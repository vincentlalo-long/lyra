mod app;
mod audio;
mod browser;
mod cover;
mod downloader;
mod input;
mod lyrics;
mod playlist;
mod scanner;
mod terminal;
mod theme;
mod ui;

use std::{path::PathBuf, time::Duration};
use anyhow::Result;
use app::App;
use crossterm::event::{self, Event, KeyEventKind};

fn main() -> Result<()> {
    // Read directory from CLI argument, default strictly to current working directory (.)
    let music_directory = match std::env::args().nth(1) {
        Some(custom_path) => PathBuf::from(custom_path),
        None => std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
    };

    let mut app = App::new(&music_directory)?;
    let mut terminal = terminal::init()?;

    let result = run_loop(&mut terminal, &mut app);
    terminal::restore(terminal)?;

    if let Err(error) = result {
        eprintln!("Error: {error:?}");
    }

    Ok(())
}

fn run_loop(terminal: &mut terminal::Tui, app: &mut App) -> Result<()> {
    loop {
        // Commit debounced seek when user finishes seeking
        app.audio.check_pending_seek();

        // Auto-advance to next track when current song finishes
        app.check_auto_advance();

        // Update background scan animation and receive results
        app.check_scan();

        // Check background YouTube download events
        app.check_download_events();

        terminal.draw(|frame| ui::render(frame, app))?;
        ui::post_render(app)?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key_event) = event::read()? {
                if key_event.kind == KeyEventKind::Press {
                    if !input::handle_key(app, key_event) {
                        return Ok(());
                    }
                }
            }
        }
    }
}
