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
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--version" || a == "-v" || a == "-V") {
        println!("lyra {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("lyra {} - Modern TUI Music Player", env!("CARGO_PKG_VERSION"));
        println!();
        println!("Usage: lyra [OPTIONS] [MUSIC_DIRECTORY]");
        println!();
        println!("Options:");
        println!("  -v, --version    Print version information");
        println!("  -h, --help       Print help information");
        return Ok(());
    }

    // Music folder priority: CLI arg > ~/.config/lyra/config.toml > ~/Music > CWD.
    let cli_dir = args.into_iter().skip(1).find(|a| !a.starts_with('-')).map(PathBuf::from);
    let config = config::Config::load();
    let (music_directory, warning) = config.resolve_music_folder(cli_dir.clone());

    // If user passed a CLI directory, persist it so future launches remember it.
    // If no config exists yet, only auto-pin the standard ~/Music folder —
    // never an arbitrary CWD (launching from a random directory must not
    // permanently hijack the library and look like a "reset" next time).
    if let Some(ref arg) = cli_dir {
        let _ = config::Config::save_setting("music_folder", &arg.to_string_lossy());
    } else if config.music_folder.is_none() && music_directory.exists() {
        let is_default_music = std::env::var_os("HOME")
            .map(|h| PathBuf::from(h).join("Music"))
            .is_some_and(|d| d == music_directory);
        if is_default_music {
            let _ = config::Config::save_setting("music_folder", &music_directory.to_string_lossy());
        }
    }

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
        // Always drained (unbounded channel); only handled when enabled.
        #[cfg(feature = "mpris")]
        {
            let mpris_on = app.plugin_enabled("mpris");
            while let Ok(key) = app.media_rx.try_recv() {
                if mpris_on {
                    app.handle_media_key(key);
                }
            }
        }

        // Commit debounced seek when user finishes seeking
        app.audio.check_pending_seek();

        // Publish playing state + position to the dynamic island (MPRIS).
        #[cfg(feature = "mpris")]
        if app.plugin_enabled("mpris") {
            app.mpris_tick();
        }

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
