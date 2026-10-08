//! MPRIS D-Bus interface (`org.mpris.MediaPlayer2.lyra`).
//!
//! This is what feeds desktop "dynamic islands" / now-playing widgets
//! (quickshell MediaModule, GNOME Shell, KDE Plasma) and tools like
//! `playerctl` and media keys. Freedesktop *notifications* (see `notify.rs`)
//! are one-shot popups and a different system entirely.
//!
//! Design: the `mpris_server::Player` object is `!Send`, so it lives on its
//! own background thread with a single-threaded tokio runtime. The sync UI
//! thread talks to it through an unbounded channel (`MprisHandle`), and
//! island/media-key presses come back as [`MediaKey`] values polled from a
//! std channel in the main loop. If D-Bus is unavailable the handle degrades
//! to a silent no-op and the app runs exactly as before.

use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicI64, AtomicU32, Ordering},
        mpsc,
    },
    time::Duration,
};

/// Position push throttle: islands poll anyway, 4 Hz is plenty smooth.
const POSITION_PUSH_INTERVAL_MICROS: i64 = 250_000;

/// Control events coming *from* the island / media keys / playerctl,
/// applied on the UI thread.
#[derive(Debug, Clone, Copy)]
pub enum MediaKey {
    Play,
    Pause,
    PlayPause,
    Stop,
    Next,
    Previous,
    /// Relative seek requested by the controller (microseconds, signed).
    SeekByMicros(i64),
    /// Absolute position requested by the controller (microseconds).
    SetPositionMicros(i64),
    /// Absolute volume 0.0..1.0 requested by the controller.
    SetVolume(f32),
}

/// Track metadata published to MPRIS.
#[derive(Debug, Clone)]
pub struct MprisTrack {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub length: Duration,
    /// Local cover file for `mpris:artUrl`, if available.
    pub art_path: Option<PathBuf>,
}

enum Cmd {
    Track(MprisTrack),
    Playing(bool),
    Position(Duration),
    Volume(f32),
    Loop { track: bool, playlist: bool },
}

/// Sender side of the MPRIS bridge, owned by the UI thread.
/// All methods are non-blocking and silently ignored when MPRIS is down.
pub struct MprisHandle {
    tx: Option<tokio::sync::mpsc::UnboundedSender<Cmd>>,
    has_track: AtomicBool,
    last_playing: AtomicBool,
    last_vol_bits: AtomicU32,
    last_pos_micros: AtomicI64,
}

impl MprisHandle {
    fn disabled() -> Self {
        Self {
            tx: None,
            has_track: AtomicBool::new(false),
            last_playing: AtomicBool::new(false),
            last_vol_bits: AtomicU32::new(u32::MAX),
            last_pos_micros: AtomicI64::new(-1),
        }
    }

    /// Publish a new track (call on every track change).
    pub fn set_track(&self, track: MprisTrack) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(Cmd::Track(track));
            self.has_track.store(true, Ordering::Relaxed);
            // Force a position refresh right after the track switch.
            self.last_pos_micros.store(-1, Ordering::Relaxed);
        }
    }

    /// Publish play/pause state.
    pub fn set_playing(&self, playing: bool) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(Cmd::Playing(playing));
        }
    }

    /// Publish current volume 0.0..1.0.
    pub fn set_volume(&self, volume: f32) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(Cmd::Volume(volume));
        }
    }

    /// Publish repeat mode as MPRIS loop status.
    pub fn set_loop(&self, track: bool, playlist: bool) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(Cmd::Loop { track, playlist });
        }
    }

    /// Push the current position (throttled to ~4 Hz). Call every UI tick.
    pub fn tick_position(&self, pos: Duration) {
        let tx = match &self.tx {
            Some(tx) => tx,
            None => return,
        };
        if !self.has_track.load(Ordering::Relaxed) {
            return;
        }
        let micros = duration_to_micros(pos);
        let last = self.last_pos_micros.load(Ordering::Relaxed);
        if last >= 0 && (micros - last).abs() < POSITION_PUSH_INTERVAL_MICROS {
            return;
        }
        self.last_pos_micros.store(micros, Ordering::Relaxed);
        let _ = tx.send(Cmd::Position(pos));
    }

    /// Sync playing state + volume + position from the audio engine.
    /// Call every UI tick; pushes only on change (position throttled).
    /// Does nothing until the first track is published.
    pub fn tick(&self, playing: bool, pos: Duration, volume: f32) {
        if self.tx.is_none() || !self.has_track.load(Ordering::Relaxed) {
            return;
        }
        if self.last_playing.swap(playing, Ordering::Relaxed) != playing {
            self.set_playing(playing);
        }
        let bits = volume.to_bits();
        if self.last_vol_bits.swap(bits, Ordering::Relaxed) != bits {
            self.set_volume(volume);
        }
        self.tick_position(pos);
    }
}

/// Duration <-> MPRIS microseconds (i64) helpers.
pub fn duration_to_micros(d: Duration) -> i64 {
    d.as_micros().min(i64::MAX as u128) as i64
}

pub fn micros_to_duration(micros: i64) -> Duration {
    Duration::from_micros(micros.max(0) as u64)
}

/// Starts the MPRIS background thread. Never fails: without D-Bus the
/// returned handle is a silent no-op. Set `LYRA_NO_MPRIS=1` to opt out.
///
/// Incoming island/media-key presses arrive on the returned receiver,
/// which the main loop should drain every tick.
pub fn spawn() -> (MprisHandle, mpsc::Receiver<MediaKey>) {
    let (media_tx, media_rx) = mpsc::channel();

    if std::env::var_os("LYRA_NO_MPRIS").is_some() {
        return (MprisHandle::disabled(), media_rx);
    }

    let (cmd_tx, cmd_rx) = tokio::sync::mpsc::unbounded_channel();
    std::thread::spawn(move || {
        if run_server(cmd_rx, media_tx).is_err() {
            // No session bus / D-Bus down: island simply won't list us.
        }
    });

    (
        MprisHandle {
            tx: Some(cmd_tx),
            has_track: AtomicBool::new(false),
            last_playing: AtomicBool::new(false),
            last_vol_bits: AtomicU32::new(u32::MAX),
            last_pos_micros: AtomicI64::new(-1),
        },
        media_rx,
    )
}

/// Builds the published track info for a song file: ID3/filename metadata,
/// known duration, and a stable `file://` cover URL when art exists.
///
/// The cover is written to `<cache>/lyra/cover-<hash>.png` (one file per
/// song, so islands never show stale art through URL caching). Sibling
/// `cover-*.png` files from previous tracks are cleaned up best-effort.
pub fn track_for_song(
    song_path: &Path,
    length: Option<Duration>,
    cover: Option<&crate::cover::AlbumArt>,
) -> MprisTrack {
    let (artist, title, album) = crate::notify::track_meta(song_path);
    let art_path = cover
        .filter(|c| c.image.is_some())
        .and_then(|c| write_cover_file(song_path, c));

    MprisTrack {
        title,
        artist,
        album,
        length: length.unwrap_or(Duration::ZERO),
        art_path,
    }
}

fn cache_dir() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|h| {
                let mut p = PathBuf::from(h);
                p.push(".cache");
                p
            })
        })?;
    let mut dir = base;
    dir.push("lyra");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

fn write_cover_file(song_path: &Path, cover: &crate::cover::AlbumArt) -> Option<PathBuf> {
    let dir = cache_dir()?;
    let mut hasher = DefaultHasher::new();
    song_path.to_string_lossy().hash(&mut hasher);
    let name = format!("cover-{:016x}.png", hasher.finish());
    let path = dir.join(&name);

    // Best-effort cleanup of previous tracks' files (same dir, other hashes).
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.extension().is_some_and(|e| e == "png")
                && p.file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with("cover-"))
                && p != path
            {
                let _ = std::fs::remove_file(&p);
            }
        }
    }

    if !path.exists() && !cover.save_png(&path) {
        return None;
    }
    Some(path)
}

fn run_server(
    mut cmd_rx: tokio::sync::mpsc::UnboundedReceiver<Cmd>,
    media_tx: mpsc::Sender<MediaKey>,
) -> anyhow::Result<()> {
    use mpris_server::Player;

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    rt.block_on(async move {
        let player = Player::builder("lyra")
            .identity("lyra")
            .can_play(true)
            .can_pause(true)
            .can_go_next(true)
            .can_go_previous(true)
            .can_seek(true)
            .build()
            .await?;

        // Island / media-key presses -> UI thread (std mpsc send is sync).
        let tx = media_tx.clone();
        player.connect_play(move |_| {
            let _ = tx.send(MediaKey::Play);
        });
        let tx = media_tx.clone();
        player.connect_pause(move |_| {
            let _ = tx.send(MediaKey::Pause);
        });
        let tx = media_tx.clone();
        player.connect_play_pause(move |_| {
            let _ = tx.send(MediaKey::PlayPause);
        });
        let tx = media_tx.clone();
        player.connect_stop(move |_| {
            let _ = tx.send(MediaKey::Stop);
        });
        let tx = media_tx.clone();
        player.connect_next(move |_| {
            let _ = tx.send(MediaKey::Next);
        });
        let tx = media_tx.clone();
        player.connect_previous(move |_| {
            let _ = tx.send(MediaKey::Previous);
        });
        let tx = media_tx.clone();
        player.connect_seek(move |_, offset| {
            let _ = tx.send(MediaKey::SeekByMicros(offset.as_micros()));
        });
        let tx = media_tx.clone();
        player.connect_set_position(move |_, _, pos| {
            let _ = tx.send(MediaKey::SetPositionMicros(pos.as_micros()));
        });
        let tx = media_tx.clone();
        player.connect_set_volume(move |_, vol| {
            let _ = tx.send(MediaKey::SetVolume(vol as f32));
        });

        // Serve D-Bus while applying UI-thread commands.
        let local = tokio::task::LocalSet::new();
        local
            .run_until(async move {
                tokio::task::spawn_local(player.run());
                while let Some(cmd) = cmd_rx.recv().await {
                    if apply_cmd(&player, cmd).await.is_err() {
                        break;
                    }
                }
            })
            .await;
        Ok::<(), anyhow::Error>(())
    })?;

    Ok(())
}

async fn apply_cmd(
    player: &mpris_server::Player,
    cmd: Cmd,
) -> anyhow::Result<()> {
    use mpris_server::{LoopStatus, Metadata, PlaybackStatus, Time};

    match cmd {
        Cmd::Track(t) => {
            let mut b = Metadata::builder()
                .title(t.title)
                .length(Time::from_micros(duration_to_micros(t.length)));
            if !t.artist.is_empty() {
                b = b.artist([t.artist]);
            }
            if !t.album.is_empty() {
                b = b.album(t.album);
            }
            if let Some(art) = t.art_path {
                b = b.art_url(format!("file://{}", art.display()));
            }
            player.set_metadata(b.build()).await?;
            player.set_playback_status(PlaybackStatus::Playing).await?;
            player.set_position(Time::ZERO);
        }
        Cmd::Playing(playing) => {
            player
                .set_playback_status(if playing {
                    PlaybackStatus::Playing
                } else {
                    PlaybackStatus::Paused
                })
                .await?;
        }
        Cmd::Position(pos) => {
            player.set_position(Time::from_micros(duration_to_micros(pos)));
        }
        Cmd::Volume(vol) => {
            player.set_volume(vol as f64).await?;
        }
        Cmd::Loop { track, playlist } => {
            player
                .set_loop_status(if track {
                    LoopStatus::Track
                } else if playlist {
                    LoopStatus::Playlist
                } else {
                    LoopStatus::None
                })
                .await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_duration_micros_roundtrip() {
        let d = Duration::from_millis(12_345);
        assert_eq!(micros_to_duration(duration_to_micros(d)), d);
        assert_eq!(micros_to_duration(-5), Duration::ZERO);
    }

    #[test]
    fn test_disabled_handle_is_silent_noop() {
        let h = MprisHandle::disabled();
        h.set_playing(true);
        h.tick(true, Duration::from_secs(10), 0.8);
        h.set_volume(0.5);
        h.set_loop(false, true);
        h.set_track(MprisTrack {
            title: "t".into(),
            artist: "a".into(),
            album: String::new(),
            length: Duration::from_secs(60),
            art_path: None,
        });
    }

    /// Live D-Bus test: publishes a uniquely-titled track and checks it via
    /// `playerctl`, including the island->app direction (PlayPause echo).
    /// Needs a real session bus + playerctl; opt in with LYRA_TEST_MPRIS_LIVE=1.
    #[test]
    fn test_mpris_live_on_bus() {
        use std::process::Command;

        if std::env::var_os("LYRA_TEST_MPRIS_LIVE").is_none() {
            return;
        }

        let unique_title = format!("LYRA_MPRIS_PROBE_{}", std::process::id());
        let (handle, rx) = spawn();
        handle.set_track(MprisTrack {
            title: unique_title.clone(),
            artist: "probe".into(),
            album: String::new(),
            length: Duration::from_secs(60),
            art_path: None,
        });
        handle.tick(true, Duration::from_secs(5), 0.8);

        // Wait for our player to appear on the bus (up to ~5s).
        let mut ours = None;
        for _ in 0..50 {
            let out = Command::new("playerctl").arg("-l").output();
            if let Ok(out) = out {
                let list = String::from_utf8_lossy(&out.stdout);
                if let Some(name) = list
                    .lines()
                    .map(str::trim)
                    .find(|n| n.starts_with("lyra"))
                {
                    ours = Some(name.to_string());
                    break;
                }
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        let player = ours.expect("lyra MPRIS player did not appear on the bus");
        eprintln!("MPRIS live probe: found player '{player}' on the bus");

        // Metadata round-trip: island sees our title.
        let out = Command::new("playerctl")
            .args(["-p", &player, "metadata", "title"])
            .output()
            .expect("playerctl metadata failed");
        let title = String::from_utf8_lossy(&out.stdout).trim().to_string();
        assert_eq!(title, unique_title);

        // Input direction: island PlayPause must reach our channel.
        let status = Command::new("playerctl")
            .args(["-p", &player, "play-pause"])
            .status()
            .expect("playerctl play-pause failed");
        assert!(status.success());
        let key = rx
            .recv_timeout(Duration::from_secs(5))
            .expect("no MediaKey arrived from MPRIS");
        assert!(matches!(key, MediaKey::PlayPause));
    }
}
