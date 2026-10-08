use std::time::{Duration, Instant};
use rodio::{Decoder, Source};

use super::{cache::SharedAudio, AudioPlayer};

/// Debounce interval before actually committing seek to hardware sink (150ms)
const SEEK_DEBOUNCE_MILLIS: u64 = 150;
/// Micro fade-in applied after a seek rebuild to mask MP3 frame artifacts.
const SEEK_FADE_MILLIS: u64 = 60;

impl AudioPlayer {
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

    pub(crate) fn apply_seek(&mut self, target: Duration) {
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
        let base = self.position();
        self.seek_to(base + Duration::from_secs(seconds));
    }

    /// Seek backward with target coalescing (accumulating multiple rapid key presses)
    pub fn seek_backward(&mut self, seconds: u64) {
        let base = self.position();
        self.seek_to(base.saturating_sub(Duration::from_secs(seconds)));
    }

    /// Absolute seek with target coalescing. Shared by keyboard seeks and
    /// external controllers (MPRIS SetPosition / Seek).
    pub fn seek_to(&mut self, target: Duration) {
        if self.duration.is_none() && self.sink.empty() {
            return;
        }

        // Mute volume immediately WITHOUT pause so hardware DMA buffer drains in silence
        self.sink.set_volume(0.0);

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
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

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
