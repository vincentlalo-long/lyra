mod app;
mod audio;
mod browser;
mod terminal;
mod ui;

use std::{path::PathBuf, time::Duration};
use anyhow::Result;
use app::{App, ViewMode};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};

fn main() -> Result<()> {
    // Read optional directory argument (e.g. `cargo run -- ~/Music`)
    // Defaults to `../output` if omitted
    let music_directory = match std::env::args().nth(1) {
        Some(custom_path) => PathBuf::from(custom_path),
        None => PathBuf::from("../output"),
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
        terminal.draw(|frame| ui::render(frame, app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key_event) = event::read()? {
                if key_event.kind == KeyEventKind::Press {
                    match key_event.code {
                        KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                        KeyCode::Tab => app.toggle_view(),
                        KeyCode::Char('1') => app.view_mode = ViewMode::Playlist,
                        KeyCode::Char('2') => app.view_mode = ViewMode::Browser,
                        KeyCode::Up | KeyCode::Char('k') => app.previous(),
                        KeyCode::Down | KeyCode::Char('j') => app.next(),
                        KeyCode::Enter => app.on_enter(),
                        KeyCode::Char(' ') => app.audio.toggle_pause(),
                        _ => {}
                    }
                }
            }
        }
    }
}
