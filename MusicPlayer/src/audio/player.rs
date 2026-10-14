use std::{fs::File, io::BufReader, path::Path, time::Duration};
use anyhow::Result;
use rodio::{Decoder, OutputStream, Sink, Source};

use super::AudioPlayer;

impl AudioPlayer {
    pub fn new() -> Result<Self> {
        let (_stream, stream_handle) = OutputStream::try_default()?;
        let sink = Sink::try_new(&stream_handle)?;
        let initial_volume = 0.8;
        sink.set_volume(initial_volume);

        Ok(Self {
            _stream,
            sink,
            is_paused: false,
            volume: initial_volume,
            previous_volume: initial_volume,
            has_started: false,
            duration: None,
            pending_seek: None,
            pos_offset: Duration::ZERO,
            current_path: None,
        })
    }

    fn open_stream(song_path: &Path) -> Result<Decoder<BufReader<File>>> {
        let file = File::open(song_path)?;
        Decoder::new(BufReader::new(file)).map_err(anyhow::Error::from)
    }

    pub fn play(&mut self, song_path: &Path) -> Result<()> {
        // Streaming playback: decode directly from disk via BufReader.
        // Previously the whole file was loaded with `fs::read` into an
        // `Arc<Vec<u8>>` and kept for the session (~5-50MB MP3, 50-200MB
        // FLAC, plus decoded PCM on top). Now RAM stays flat regardless
        // of file size; seeks reopen the file (see `seek.rs`).
        let audio_source = Self::open_stream(song_path)?;

        self.pending_seek = None;
        self.pos_offset = Duration::ZERO;
        self.current_path = Some(song_path.to_path_buf());
        self.duration = audio_source.total_duration();

        self.sink.stop();
        self.sink.append(audio_source);
        self.sink.set_volume(self.volume);
        self.sink.play();
        self.is_paused = false;
        self.has_started = false;

        Ok(())
    }


    pub fn toggle_pause(&mut self) {
        // Flush any pending seek immediately before toggling pause state
        if let Some((target, _)) = self.pending_seek.take() {
            self.apply_seek(target);
        }

        if self.is_paused {
            self.sink.set_volume(self.volume);
            self.sink.play();
            self.is_paused = false;
        } else {
            self.sink.pause();
            self.is_paused = true;
        }
    }

    /// Returns current playback position. If a seek operation is pending/coalescing,
    /// returns the accumulated target position for zero-latency UI updates.
    /// Otherwise the true position is `pos_offset + sink.get_pos()` because the
    /// sink counter restarts at 0 on every rebuild-append.

    /// Absolute volume set (used by external controllers, e.g. MPRIS).
    #[cfg(feature = "mpris")]
    pub fn set_volume_absolute(&mut self, value: f32) {
        self.volume = value.clamp(0.0, 1.0);
        if self.volume > 0.0 {
            self.previous_volume = self.volume;
        }
        if self.pending_seek.is_none() {
            self.sink.set_volume(self.volume);
        }
    }

    pub fn volume_up(&mut self) {
        self.volume = (self.volume + 0.05).min(1.0);
        if self.pending_seek.is_none() {
            self.sink.set_volume(self.volume);
        }
    }

    pub fn volume_down(&mut self) {
        self.volume = (self.volume - 0.05).max(0.0);
        if self.pending_seek.is_none() {
            self.sink.set_volume(self.volume);
        }
    }

    pub fn toggle_mute(&mut self) {
        if self.volume > 0.0 {
            self.previous_volume = self.volume;
            self.volume = 0.0;
        } else {
            self.volume = if self.previous_volume > 0.0 { self.previous_volume } else { 0.5 };
        }
        // While a seek is debouncing the sink is force-muted; defer applying
        // so we don't unmute mid-seek and leak audio. apply_seek() restores
        // self.volume when it commits.
        if self.pending_seek.is_none() {
            self.sink.set_volume(self.volume);
        }
    }

    /// Check if the track has finished playing

    pub fn is_finished(&mut self) -> bool {
        // While seeking, the track is not finished
        if self.pending_seek.is_some() {
            return false;
        }
        if !self.has_started {
            if !self.sink.empty() {
                self.has_started = true;
            }
            false
        } else {
            self.sink.empty()
        }
    }
}
