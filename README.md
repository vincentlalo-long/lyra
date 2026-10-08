# lyra

A modular, pluggable terminal music player written in Rust.

Lyra is designed around a decoupled, add-on architecture: core playback is isolated, while extensions (stream downloader, studio artwork scraper, synced lyrics visualizer, MPRIS bridge, and genre tagger) can be enabled, disabled, or stripped away entirely depending on how you build it.

---

## Philosophy & Comparison

Lyra is not designed to claim absolute superiority over established unix audio tools like `cmus`, `mpd`, or `rmpc`. Those tools have decades of refinement and excel at their specific models.

Instead, Lyra was written out of personal daily workflow preferences:
- A self-contained terminal player that runs without requiring a background daemon setup.
- The ability to discover, fetch, inspect studio cover art, and grab synchronized `.lrc` lyrics in the same terminal session without switching context.
- Modular internals: if you want a minimal, zero-network player, you can compile out all online dependencies and run a lean binary akin to `cmus`. If you want an all-in-one studio station, you can run the full flavor with all built-in plugins.

---

## Tech Stack

- **TUI Framework**: Rust (`ratatui`, `crossterm`)
- **Audio Output**: `rodio` (ALSA, PulseAudio, PipeWire)
- **ID3 & Audio Metadata**: `id3`
- **IPC & Media Keys**: `mpris-server`, `zbus` (desktop integration, playerctl)
- **Terminal Graphics**: Native Kitty Graphics Protocol with automatic ANSI half-block fallback
- **Helper Pipeline (Optional)**: Python 3 (`yt-dlp`, `pillow`, `mutagen`) for multi-source studio cover art scoring (iTunes, Deezer, Cover Art Archive), synced lyrics extraction, and audio conversion.

---

## Cover Art & Terminal Support

Lyra renders high-resolution album artwork directly in the terminal interface using the Kitty Graphics Protocol (falling back to ANSI half-block pixel rendering on other terminals).

Terminals with verified native high-res cover art support:
1. **Kitty** — Author and creator of the Kitty Graphics Protocol.
2. **Ghostty** — Mitchell Hashimoto's next-generation, high-performance terminal emulator.
3. **WezTerm** — Popular GPU-accelerated, cross-platform terminal emulator.

---

## Flavors & Releases

Because of its modular design, Lyra ships in different flavors:

### 1. Full TUI (Default / Recommended)
The standard build with all built-in extensions compiled in:
- Track list & day-list queue management
- File system browser with quick-import and folder-level queuing
- Integrated YouTube audio search and background downloader
- Multi-provider studio cover art search, preview, and tagger
- Real-time synchronized lyrics renderer (`.lrc`)
- MPRIS desktop integration (`playerctl`, media keys, lock screen display)
- Library-wide multiple-choice genre filtering (`f`) and inline tag editor (`t`)

### 2. Minimal TUI (`--no-default-features`)
Stripped-down, `cmus`-like terminal audio player:
- Pure local audio playback and library browsing
- Zero Python runtime dependency
- Zero network requests
- Instant startup and minimal memory footprint

### 3. CLI Only (Python script, not recommended for daily listening)
A standalone command-line pipeline (`main.py`) intended for shell scripts or headless servers:
- Audio ripping from URLs with automated ID3 tagging
- Headless artwork search and caching
- Lyric fetching without a terminal interface

---

## Installation & Building

### Prebuilt Binaries

Precompiled binaries for Linux (x86_64) are available on [GitHub Releases](https://github.com/vincentlalo-long/lyra/releases):

- **Full TUI** (`lyra-linux-x86_64.tar.gz`): Standard build with all built-in extensions enabled.
- **Minimal TUI** (`lyra-minimal-linux-x86_64.tar.gz`): Lean standalone binary, zero Python/network dependencies.
- **CLI Only** (`lyra-cli.tar.gz`): Headless audio pipeline and cover tagger.

Quick installation (extracts directly to `~/.local/bin`):
```bash
# Full TUI (Recommended)
curl -sSL https://github.com/vincentlalo-long/lyra/releases/latest/download/lyra-linux-x86_64.tar.gz | tar -xz -C ~/.local/bin/
chmod +x ~/.local/bin/lyra

# Or Minimal TUI
curl -sSL https://github.com/vincentlalo-long/lyra/releases/latest/download/lyra-minimal-linux-x86_64.tar.gz | tar -xz -C ~/.local/bin/
chmod +x ~/.local/bin/lyra
```
Ensure `~/.local/bin` is in your `$PATH`.

### Requirements (Source Build)
- **Rust toolchain** (1.80+)
- **ffmpeg** (required for audio conversion and thumbnail processing)
- **Python 3.10+** (only needed for Full TUI / download & cover features)
  - `pip install yt-dlp pillow mutagen`

### Building from Source

Clone the repository:
```bash
git clone https://github.com/vincentlalo-long/lyra.git
cd lyra
```

#### Build Full TUI (All features):
```bash
cargo install --path MusicPlayer --force
```

#### Build Minimal TUI (No network, no Python, pure player):
```bash
cargo install --path MusicPlayer --no-default-features --force
```

#### Custom Feature Selection:
You can cherry-pick specific features using Cargo flags:
```bash
# Example: Local player + Genre DB + MPRIS (no downloader)
cargo install --path MusicPlayer --no-default-features --features "genre,mpris" --force
```

Available cargo features:
- `download`: Background downloader and studio cover search engine
- `genre`: Multi-choice genre filtering and inline metadata tagger
- `mpris`: D-Bus media key and desktop widget bridge
- `notify`: Desktop notifications on track change

---

## Basic Controls

| Key | Action |
|---|---|
| `Tab` / `1-5` | Switch views: Playlist, Queue, Browser, Download, Plugins |
| `Space` | Play / Pause (or toggle select in modal pickers) |
| `k` / `j` or `↑` / `↓` | Move cursor up / down |
| `Enter` | Play track / Open directory / Confirm action |
| `n` / `p` | Next / Previous track |
| `←` / `→` or `[` / `]` | Seek backward / forward 5 seconds |
| `+` / `-` | Volume up / down |
| `m` | Toggle mute |
| `r` | Cycle repeat modes (Playlist -> Track -> Off) |
| `v` / `y` | Toggle synchronized lyrics view |
| `t` | Tag / Edit genre for selected song (stores in ID3 & sidecar) |
| `f` | Open Genre Filter modal (Space/Enter: select, f: apply) |
| `a` / `d` | Add track to queue / Remove from playlist or queue |
| `i` | Import folder/album from Browser into Playlist |
| `/` | Search in Playlist or Browser |
| `?` | Toggle keybindings help modal |
| `q` | Quit |

---

## Configuration & Data Paths

- Main Configuration: `~/.config/lyra/config.toml`
  ```toml
  music_directory = "~/Music"
  volume = 80
  repeat_mode = "playlist"
  ```
- Genre Sidecar: `~/.config/lyra/genres.json`
- Artwork Cache: `~/.cache/lyra/covers/`

---

## Contributing & Extensions

Lyra welcomes contributions, bug reports, and plugin pull requests.
See [CONTRIBUTING.md](CONTRIBUTING.md) for contribution guidelines, project layout, testing rules, and [MusicPlayer/plugins.md](MusicPlayer/plugins.md) for how to build or decouple modular compile-time plugins.

---

## License

MIT
