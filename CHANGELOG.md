# Changelog

All notable changes to Lyra are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- `LICENSE` (MIT), `SECURITY.md`, and `CODE_OF_CONDUCT.md`.
- Pull request and issue templates, plus Dependabot configuration.
- CI workflow covering the Rust feature matrix, Rust tests, and Python tests.
- One-command installer (`scripts/install.sh`) for prebuilt binaries, with
  SHA-256 verification and `--minimal`, `--dir`, `--version` options.
- `Makefile` shortcuts (`make verify`, `make check`, `make lint`, `make run`)
  and a shared `scripts/check.sh` warning gate used by both CI and `make`.

### Fixed

- Build warnings in both the default and `--no-default-features` configurations.

### Changed

- `app/browser_ops` (1.2k lines) is split into focused submodules: caret
  helpers, modal entry points, cover/move filesystem ops, and one file per
  modal's key handler. Behavior is unchanged.
- Built-in plugin identity is now a typed `PluginId` registry: feature gates,
  Plugin Manager metadata, and runtime checks share one source of truth, and
  `App::plugin_enabled` takes a `PluginId` instead of a string.
- Releases now only run for `v*` tags, verify the tag matches
  `MusicPlayer/Cargo.toml`, and run the test suite before publishing.

## [0.3.4] - 2026-11-09

### Added

- Official metadata search (Spotify, MusicBrainz, Deezer, iTunes) with YouTube
  fallback for download resolution.

### Fixed

- Lyrics download and selection fixes.
- Top tab navigation and UI consistency fixes.
