//! MPRIS D-Bus interface (`org.mpris.MediaPlayer2.lyra`).
//!
//! Feeds desktop dynamic islands / now-playing widgets and `playerctl`.
//! See `types.rs` (shared handle) and `server.rs` (background D-Bus thread).

pub mod server;
pub mod types;

pub use server::{spawn, track_for_song};
pub use types::{MediaKey, MprisHandle, micros_to_duration};
