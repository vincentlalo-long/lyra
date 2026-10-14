pub mod cache;
pub mod player;
pub mod seek;

use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
use rodio::{OutputStream, Sink};

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
    /// Path of the currently playing track. Seeks reopen this file with a
    /// buffered streaming decoder instead of holding the whole file in RAM.
    /// (Old `cached_bytes: Arc<Vec<u8>>` kept 1 full song + decoded PCM in
    /// memory — 50-200MB for FLAC — and was never cleared.)
    current_path: Option<PathBuf>,
}
