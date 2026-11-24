//! Browser action modals: prompts, text editing, filesystem moves, and keys.
//!
//! This used to be a single 1.2k-line file. Each modal's key handling now lives
//! in its own submodule and dispatches through `App::handle_browser_modal_key`.

mod caret;
mod cover_ops;
mod keys;
mod modals;
mod move_track;
#[cfg(test)]
mod tests;
