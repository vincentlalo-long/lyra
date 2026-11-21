## Summary

<!-- What changes, and why? Link the issue this fixes: "Fixes #123". -->

## Type of change

- [ ] Bug fix (no behaviour change outside the fix)
- [ ] New feature (core, plugin, or CLI)
- [ ] Documentation only
- [ ] Refactor (no behaviour change)
- [ ] Release / version bump

## Verification

<!-- Paste the commands you ran. CI must stay green. -->

- [ ] `cargo check --manifest-path MusicPlayer/Cargo.toml`
- [ ] `cargo check --manifest-path MusicPlayer/Cargo.toml --no-default-features`
- [ ] `cargo test --manifest-path MusicPlayer/Cargo.toml`
- [ ] `cargo test --manifest-path MusicPlayer/Cargo.toml --no-default-features`
- [ ] `python -m pytest -q` (if Python code changed)
- [ ] `cargo fmt --manifest-path MusicPlayer/Cargo.toml -- --check` (if Rust changed)
- [ ] `cargo clippy --manifest-path MusicPlayer/Cargo.toml --all-targets -- -D warnings` (if Rust changed)

## Checklist

- [ ] Core does not import from a plugin (see `CONTRIBUTING.md`).
- [ ] New behaviour is covered by a test, or the PR explains why a test is not practical.
- [ ] Docs updated when user-facing behaviour, keys, config, or plugin contracts changed.
- [ ] No secrets, credentials, personal paths, or generated binaries are included.
- [ ] No unrelated formatting or refactor noise.
