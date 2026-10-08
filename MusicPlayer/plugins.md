# Lyra plugins (compile-time Cargo features)

Core is always on: audio engine, playlist, queue, browser, scanner,
config, metadata (`meta.rs`). Everything else is a removable plugin.

| Feature | Removes | Also drops |
|---|---|---|
| `download` (default) | YouTube search/download tab, `main.py` worker bridge | nothing else |
| `mpris` (default) | Dynamic-island / `playerctl` / media keys | `mpris-server`, `tokio` deps |
| `notify` (default) | Desktop "now playing" popups | nothing else |
| `genre` (default) | Genre DB, `f` picker, genre filter | nothing else |

```bash
cargo install --path MusicPlayer                     # everything
cargo install --path MusicPlayer --no-default-features
cargo install --path MusicPlayer --no-default-features --features download,mpris
```

`--no-default-features` builds, tests (24 tests), and runs with zero
warnings. Each single feature also builds warning-free on its own.

## Adding a new plugin (`myfeat`)

1. `Cargo.toml`: add `myfeat = []` to `[features]` (+ `dep:...` for new deps, always `optional = true`), and to `default` if it ships on.
2. Code lives in `src/myfeat.rs` (or `src/myfeat/` when >200 lines):
   `mod.rs` (re-exports) + `types.rs` + `server.rs`/`worker.rs` + `view.rs` as needed.
3. `main.rs`: `#[cfg(feature = "myfeat")] mod myfeat;`
4. State: `#[cfg]` fields + constructor lines in `app/mod.rs` (separate
   `#[cfg] use` statements — attributes are **not** allowed inside a
   `use {...}` group).
5. Views: new `ViewMode` variant gets `#[cfg]`; every `match` on it needs a
   gated arm (compiler lists them).
6. Keys: gate arms in `input/` (`#[cfg]` works on match arms). Dynamic text
   in widgets: use `cfg!(...)`, never `#[cfg]` inside `vec![]`/expressions
   (use `{ #[cfg]...; }` statement blocks instead).
7. Shared helpers used by ≥2 plugins go to core (`meta.rs` precedent:
   `track_meta` serves both `notify` and `mpris`).
8. Tests: unit tests live with the code (skipped automatically when the
   feature is off); run `cargo test` (default) and
   `cargo test --no-default-features` before merging.

## Rules

- Core must never import from a plugin (`mpris`/`notify` → `meta`, never back).
- `pub(crate)` for cross-module items inside a plugin; plain `pub` only at
  the `mod.rs` re-export boundary.
- Zero-warning policy holds per feature combination in the matrix above.
