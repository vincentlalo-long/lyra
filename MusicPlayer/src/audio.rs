use std::{fs::File, io::BufReader, path::Path, time::Duration};
use anyhow::Result;
use rodio::{Decoder, OutputStream, Sink, Source};

pub struct AudioPlayer {
    _stream: OutputStream,
    sink: Sink,
    pub is_paused: bool,
    pub volume: f32,
    previous_volume: f32,
    has_started: bool,
    pub duration: Option<Duration>,
}

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
        })
    }

    pub fn play(&mut self, song_path: &Path) -> Result<()> {
        let file = File::open(song_path)?;
        let buffered_reader = BufReader::new(file);
        let audio_source = Decoder::new(buffered_reader)?;

        self.duration = audio_source.total_duration();

        self.sink.clear();
        self.sink.append(audio_source);
        self.sink.set_volume(self.volume);
        self.sink.play();
        self.is_paused = false;
        self.has_started = false;

        Ok(())
    }

    pub fn toggle_pause(&mut self) {
        if self.is_paused {
            self.sink.play();
            self.is_paused = false;
        } else {
            self.sink.pause();
            self.is_paused = true;
        }
    }

    pub fn position(&self) -> Duration {
        self.sink.get_pos()
    }

    pub fn seek_forward(&mut self, seconds: u64) {
        let current = self.sink.get_pos();
        let target = current + Duration::from_secs(seconds);
        if let Some(dur) = self.duration {
            if target < dur {
                let _ = self.sink.try_seek(target);
            }
        } else {
            let _ = self.sink.try_seek(target);
        }
    }

    pub fn seek_backward(&mut self, seconds: u64) {
        let current = self.sink.get_pos();
        let target = current.saturating_sub(Duration::from_secs(seconds));
        let _ = self.sink.try_seek(target);
    }

    pub fn volume_up(&mut self) {
        self.volume = (self.volume + 0.05).min(1.0);
        self.sink.set_volume(self.volume);
    }

    pub fn volume_down(&mut self) {
        self.volume = (self.volume - 0.05).max(0.0);
        self.sink.set_volume(self.volume);
    }

    pub fn toggle_mute(&mut self) {
        if self.volume > 0.0 {
            self.previous_volume = self.volume;
            self.volume = 0.0;
        } else {
            self.volume = if self.previous_volume > 0.0 { self.previous_volume } else { 0.5 };
        }
        self.sink.set_volume(self.volume);
    }

    /// Check if the track has finished playing
    pub fn is_finished(&mut self) -> bool {
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
