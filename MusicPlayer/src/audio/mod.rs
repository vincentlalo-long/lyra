pub mod cache;
pub mod player;
pub mod seek;

use std::{
    sync::Arc,
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
    /// In-memory cache of the currently playing track (1 song only, ~5-50MB).
    /// Seeks decode from RAM instead of disk I/O.
    cached_bytes: Option<Arc<Vec<u8>>>,
}
