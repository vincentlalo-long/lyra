use std::{
    path::PathBuf,
    sync::atomic::{AtomicBool, AtomicI64, AtomicU32, Ordering},
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

pub(crate) enum Cmd {
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
    pub(crate) fn disabled() -> Self {
        Self {
            tx: None,
            has_track: AtomicBool::new(false),
            last_playing: AtomicBool::new(false),
            last_vol_bits: AtomicU32::new(u32::MAX),
            last_pos_micros: AtomicI64::new(-1),
        }
    }

    /// Connected handle for the live server thread.
    pub(crate) fn with_sender(tx: tokio::sync::mpsc::UnboundedSender<Cmd>) -> Self {
        Self {
            tx: Some(tx),
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
        use crate::mpris::server::spawn;

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
