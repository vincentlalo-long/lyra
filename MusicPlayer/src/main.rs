mod app;
mod audio;
mod browser;
mod config;
mod cover;
#[cfg(feature = "download")]
mod download;
#[cfg(feature = "genre")]
mod genre;
mod input;
mod lyrics;
mod meta;
#[cfg(feature = "mpris")]
mod mpris;
#[cfg(feature = "notify")]
mod notify;
mod playlist;
pub mod plugin;
mod queue;
mod scanner;
mod terminal;
mod theme;
mod ui;

use std::{path::PathBuf, time::Duration};
use anyhow::Result;
use app::App;
use crossterm::event::{self, Event, KeyEventKind};

fn main() -> Result<()> {
    // Music folder priority: CLI arg > ~/.config/lyra/config.toml > CWD.
    let cli_dir = std::env::args().nth(1).map(PathBuf::from);
    let config = config::Config::load();
    let (music_directory, warning) = config.resolve_music_folder(cli_dir);

    let mut app = App::new(&music_directory)?;
    if let Some(w) = warning {
        app.set_toast(w);
    }
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
        // Island / media-key / playerctl presses (MPRIS input side).
        #[cfg(feature = "mpris")]
        while let Ok(key) = app.media_rx.try_recv() {
            app.handle_media_key(key);
        }

        // Commit debounced seek when user finishes seeking
        app.audio.check_pending_seek();

        // Publish playing state + position to the dynamic island (MPRIS).
        #[cfg(feature = "mpris")]
        app.mpris_tick();

        // Auto-advance to next track when current song finishes
        app.check_auto_advance();

        // Update background scan animation and receive results
        app.check_scan();

        // Check background YouTube download events (download plugin)
        #[cfg(feature = "download")]
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
