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
pub mod plugin_dl;
mod queue;
mod scanner;
mod terminal;
mod theme;
mod ui;
#[cfg(feature = "update")]
mod update;

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
        println!("      --check-deps   Check runtime dependencies (Python, ffmpeg, yt-dlp)");
        println!("      --check-update Check GitHub for a newer Lyra release");
        println!("      --update       Download and install the latest Lyra release");
        return Ok(());
    }
    if args.iter().any(|a| a == "--check-deps") {
        return check_runtime_deps();
    }
    #[cfg(feature = "update")]
    {
        if args.iter().any(|a| a == "--check-update") {
            return update_cli(false);
        }
        if args.iter().any(|a| a == "--update") {
            return update_cli(true);
        }
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

/// Verifies the external tools needed by the Full TUI download pipeline.
/// Exits with status 1 when anything is missing so scripts can gate on it.
fn check_runtime_deps() -> Result<()> {
    let mut missing = false;
    println!("Lyra {} runtime dependency check\n", env!("CARGO_PKG_VERSION"));

    let mut check = |name: &str, cmd: &str, cmd_args: &[&str], hint: &str| {
        let available = std::process::Command::new(cmd)
            .args(cmd_args)
            .output()
            .map(|out| out.status.success())
            .unwrap_or(false);
        if available {
            println!("  [ok]      {name}");
        } else {
            missing = true;
            println!("  [MISSING] {name}  -> {hint}");
        }
    };

    check("python3", "python3", &["--version"], "install python3");
    check("ffmpeg", "ffmpeg", &["-version"], "install ffmpeg");
    check(
        "yt-dlp + pillow + mutagen",
        "python3",
        &["-c", "import yt_dlp, PIL, mutagen"],
        "python3 -m pip install --user yt-dlp pillow mutagen",
    );

    println!();
    if missing {
        println!("Missing dependencies detected.");
        println!("Install guide: README -> Runtime Dependencies (Prebuilt Binaries)");
        std::process::exit(1);
    }
    println!("All runtime dependencies are available.");
    Ok(())
}

#[cfg(feature = "update")]
fn update_cli(install: bool) -> Result<()> {
    match crate::update::check_for_update(true) {
        Ok(Some(manifest)) => {
            if install {
                match crate::update::install_update(&manifest) {
                    Ok(version) => {
                        println!("Updated Lyra to v{version}. Restart to use the new binary.");
                    }
                    Err(e) => {
                        eprintln!("Update failed: {e}");
                        std::process::exit(1);
                    }
                }
            } else {
                crate::update::print_check_result(&Some(manifest));
            }
        }
        Ok(None) => crate::update::print_check_result(&None),
        Err(e) => {
            eprintln!("Update check failed: {e}");
            std::process::exit(1);
        }
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

        // Sleep timer: pause playback when the timer expires.
        if let Some(start) = app.sleep_timer {
            if app.sleep_timer_minutes > 0
                && start.elapsed() >= std::time::Duration::from_secs(app.sleep_timer_minutes * 60)
            {
                app.sleep_timer = None;
                app.audio.toggle_pause();
                app.set_toast("Sleep timer: playback stopped".to_string());
            }
        }

        // Update background scan animation and receive results
        app.check_scan();

        // Check background YouTube download events (download plugin)
        #[cfg(feature = "download")]
        app.check_download_events();

        // Check background community-plugin installer events
        app.check_plugin_downloads();

        // Check background updater events (release check / self-update).
        #[cfg(feature = "update")]
        app.check_update_events();

        // Advance visualizer from elapsed time so animation speed stays stable
        // when terminal redraw latency changes.
        let now = std::time::Instant::now();
        let dt = now
            .saturating_duration_since(app.visualizer_last_tick)
            .as_secs_f32()
            .min(0.1);
        app.visualizer_last_tick = now;
        let speed = if !app.audio.is_paused { 3.2 } else { 0.8 };
        app.visualizer_phase = (app.visualizer_phase + dt * speed) % 10000.0;

        terminal.draw(|frame| ui::render(frame, app))?;
        ui::post_render(app)?;

        if event::poll(Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key_event) => {
                    if key_event.kind == KeyEventKind::Press
                        && !input::handle_key(app, key_event)
                    {
                        return Ok(());
                    }
                }
                Event::Mouse(mouse_event) => {
                    input::mouse::handle_mouse(app, mouse_event);
                }
                _ => {}
            }
        }
    }
}
