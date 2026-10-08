use std::{
    io::{Read, Seek, SeekFrom},
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};
use anyhow::Result;
use rodio::{Decoder, OutputStream, Sink, Source};

/// Debounce interval before actually committing seek to hardware sink (150ms)
const SEEK_DEBOUNCE_MILLIS: u64 = 150;
/// Micro fade-in applied after a seek rebuild to mask MP3 frame artifacts.
/// 60ms covers 2+ MP3 frames (1 frame ~= 26ms @44.1kHz) so bit-reservoir
/// priming distortion at the cut point stays inaudible.
const SEEK_FADE_MILLIS: u64 = 60;

/// Zero-copy in-memory reader for cached track bytes.
/// `Cursor<Arc<Vec<u8>>>` does not implement `Read/Seek`, so we wrap
/// `Arc<Vec<u8>> + position` manually. `Send + Sync + 'static` for rodio.
struct SharedAudio {
    data: Arc<Vec<u8>>,
    pos: u64,
}

impl SharedAudio {
    fn new(data: Arc<Vec<u8>>) -> Self {
        Self { data, pos: 0 }
    }
}

impl Read for SharedAudio {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let start = self.pos as usize;
        if start >= self.data.len() {
            return Ok(0);
        }
        let n = (self.data.len() - start).min(buf.len());
        buf[..n].copy_from_slice(&self.data[start..start + n]);
        self.pos += n as u64;
        Ok(n)
    }
}

impl Seek for SharedAudio {
    fn seek(&mut self, style: SeekFrom) -> std::io::Result<u64> {
        let len = self.data.len() as i64;
        let new_pos: i64 = match style {
            SeekFrom::Start(n) => n as i64,
            SeekFrom::End(n) => len + n,
            SeekFrom::Current(n) => self.pos as i64 + n,
        };
        let clamped = new_pos.max(0).min(len) as u64;
        self.pos = clamped;
        Ok(clamped)
    }
}

pub struct AudioPlayer {
    _stream: OutputStream,
    sink: Sink,
    pub is_paused: bool,
    pub volume: f32,
    previous_volume: f32,
    has_started: bool,
    pub duration: Option<Duration>,
    /// Seek coalescing: (target_position, execution_deadline)
    pending_seek: Option<(Duration, Instant)>,
    /// Absolute offset of the currently appended source.
    /// `Sink::get_pos()` counts samples since last `append()` (starts at 0),
    /// so after a rebuild-seek the true position is `pos_offset + sink.get_pos()`.
    pos_offset: Duration,
    /// In-memory cache of the currently playing track (1 song only, ~5-50MB).
    /// Seeks decode from RAM instead of disk I/O.
    cached_bytes: Option<Arc<Vec<u8>>>,
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
            pending_seek: None,
            pos_offset: Duration::ZERO,
            cached_bytes: None,
        })
    }

    pub fn play(&mut self, song_path: &Path) -> Result<()> {
        // Cache the whole track in RAM once: rapid seeks then decode from
        // memory instead of issuing disk seek()+re-parse on every keypress.
        let bytes = Arc::new(std::fs::read(song_path)?);
        let audio_source = Decoder::new(SharedAudio::new(Arc::clone(&bytes)))?;

        self.pending_seek = None;
        self.pos_offset = Duration::ZERO;
        self.cached_bytes = Some(Arc::clone(&bytes));
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
    pub fn position(&self) -> Duration {
        if let Some((target, _)) = self.pending_seek {
            target
        } else {
            self.pos_offset + self.sink.get_pos()
        }
    }

    /// Check and commit pending debounced seek if the deadline has elapsed
    pub fn check_pending_seek(&mut self) {
        if let Some((target, deadline)) = self.pending_seek {
            if Instant::now() >= deadline {
                self.pending_seek = None;
                self.apply_seek(target);
            }
        }
    }

    /// Execute the seek by rebuilding the sink from the in-memory cache with smooth fade-in.
    ///
    /// Why this eliminates audio glitches / blips:
    /// - During the 150ms debounce interval, volume was set to 0.0 immediately on keypress,
    ///   giving the Linux ALSA/Pulse/PipeWire hardware DMA ring buffer (~50-80ms) plenty of
    ///   time to completely drain any stale audio residue in total silence.
    /// - A 30ms `fade_in` is applied to the newly decoded stream, ensuring a zero-crossing
    ///   smooth entrance without pops/clicks or MP3 bit-reservoir boundary artifacts.
    fn apply_seek(&mut self, target: Duration) {
        if let Some(bytes) = self.cached_bytes.clone() {
            match Decoder::new(SharedAudio::new(bytes)) {
                Ok(mut fresh) => {
                    if fresh.try_seek(target).is_ok() {
                        let faded = fresh.fade_in(Duration::from_millis(SEEK_FADE_MILLIS));
                        self.sink.stop();
                        self.sink.append(faded);
                        // New source counts from 0, so remember absolute base.
                        self.pos_offset = target;
                    } else {
                        // Cold-seek failed (source would be broken/intact-uncertain):
                        // keep the live stream and seek it instead of appending
                        // a source stuck at the wrong position.
                        let _ = self.sink.try_seek(target);
                        // Live-seek keeps the sink's absolute counter.
                        self.pos_offset = Duration::ZERO;
                    }
                }
                Err(_) => {
                    let _ = self.sink.try_seek(target);
                    self.pos_offset = Duration::ZERO;
                }
            }
        } else {
            let _ = self.sink.try_seek(target);
            self.pos_offset = Duration::ZERO;
        }

        if !self.is_paused {
            self.sink.set_volume(self.volume);
            self.sink.play();
        } else {
            self.sink.pause();
            self.sink.set_volume(self.volume);
        }
    }

    /// Seek forward with target coalescing (accumulating multiple rapid key presses)
    pub fn seek_forward(&mut self, seconds: u64) {
        if self.duration.is_none() && self.sink.empty() {
            return;
        }

        // Mute volume immediately WITHOUT pause so hardware DMA buffer drains in silence
        self.sink.set_volume(0.0);

        // Base on the true (offset-aware) position, not the raw sink counter
        // which restarts at 0 after every rebuild-append.
        let base = self.position();
        let target = base + Duration::from_secs(seconds);
        let clamped = if let Some(dur) = self.duration {
            if dur > Duration::from_millis(500) {
                target.min(dur - Duration::from_millis(500))
            } else {
                target.min(dur)
            }
        } else {
            target
        };
        let deadline = Instant::now() + Duration::from_millis(SEEK_DEBOUNCE_MILLIS);
        self.pending_seek = Some((clamped, deadline));
    }

    /// Seek backward with target coalescing (accumulating multiple rapid key presses)
    pub fn seek_backward(&mut self, seconds: u64) {
        if self.duration.is_none() && self.sink.empty() {
            return;
        }

        // Mute volume immediately WITHOUT pause so hardware DMA buffer drains in silence
        self.sink.set_volume(0.0);

        // Base on the true (offset-aware) position, not the raw sink counter
        // which restarts at 0 after every rebuild-append.
        let base = self.position();
        let clamped = base.saturating_sub(Duration::from_secs(seconds));
        let deadline = Instant::now() + Duration::from_millis(SEEK_DEBOUNCE_MILLIS);
        self.pending_seek = Some((clamped, deadline));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seek_coalescing_and_debouncing() {
        if let Ok(mut player) = AudioPlayer::new() {
            player.duration = Some(Duration::from_secs(60));

            // Rapid consecutive seeks accumulate without delay in position()
            player.seek_forward(5);
            assert_eq!(player.position(), Duration::from_secs(5));

            player.seek_forward(5);
            assert_eq!(player.position(), Duration::from_secs(10));

            player.seek_forward(5);
            assert_eq!(player.position(), Duration::from_secs(15));

            player.seek_backward(3);
            assert_eq!(player.position(), Duration::from_secs(12));

            // While seeking, track is not finished
            assert!(!player.is_finished());

            // Still pending before deadline
            player.check_pending_seek();
            assert!(player.pending_seek.is_some());

            // After debounce interval, seek is committed to sink
            std::thread::sleep(Duration::from_millis(SEEK_DEBOUNCE_MILLIS + 20));
            player.check_pending_seek();
            assert!(player.pending_seek.is_none());
        }
    }

    #[test]
    fn test_shared_audio_read_and_seek() {
        use std::io::{Read, Seek, SeekFrom};

        let data = Arc::new(vec![1u8, 2, 3, 4, 5]);
        let mut audio = SharedAudio::new(Arc::clone(&data));

        // Sequential reads advance position
        let mut buf = [0u8; 2];
        assert_eq!(audio.read(&mut buf).unwrap(), 2);
        assert_eq!(buf, [1, 2]);

        // Seek to absolute offset and read tail
        assert_eq!(audio.seek(SeekFrom::Start(4)).unwrap(), 4);
        let mut tail = [0u8; 2];
        assert_eq!(audio.read(&mut tail).unwrap(), 1);
        assert_eq!(tail[0], 5);

        // Seeking past end clamps, reads return EOF
        assert_eq!(audio.seek(SeekFrom::Start(99)).unwrap(), 5);
        assert_eq!(audio.read(&mut tail).unwrap(), 0);
    }

    #[test]
    fn test_seek_clamping_to_duration() {
        if let Ok(mut player) = AudioPlayer::new() {
            player.duration = Some(Duration::from_secs(10));

            // Seeking past duration clamps to duration - 500ms
            player.seek_forward(30);
            assert_eq!(player.position(), Duration::from_millis(9500));

            // Seeking before 0 saturates to 0
            player.seek_backward(50);
            assert_eq!(player.position(), Duration::ZERO);
        }
    }
}
