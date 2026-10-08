use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    sync::mpsc,
    time::Duration,
};

use super::types::{Cmd, MediaKey, MprisHandle, MprisTrack, duration_to_micros};


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
        MprisHandle::with_sender(cmd_tx),
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
    let (artist, title, album) = crate::meta::track_meta(song_path);
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
