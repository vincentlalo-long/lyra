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
- **Helper Pipeline (Optional)**: Python 3 (`yt-dlp`, `pillow`, `mutagen`) for metadata-first music search, multi-source studio cover art scoring (iTunes, Deezer, Cover Art Archive), LRCLIB lyrics, and audio conversion.

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
- Metadata-first music search (Spotify/MusicBrainz/Deezer/iTunes) with YouTube fallback and background downloader
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

### Runtime Dependencies (Prebuilt Binaries)

The prebuilt binaries are **dynamically linked** and do **not** bundle external
tools. Install these before running Lyra, otherwise startup or downloads will
fail with missing-library / `os error` messages:

| Dependency | Needed by | Why |
|---|---|---|
| `ffmpeg` | Full TUI | Audio conversion and thumbnail processing |
| `python3` | Full TUI | Runs the download / cover / lyrics pipeline |
| `yt-dlp`, `pillow`, `mutagen` | Full TUI | Python packages for downloading and tagging |
| `alsa-lib` (`libasound`) | All builds | Audio output |
| `libudev` | All builds | Device detection |

The **Minimal TUI** (`lyra-minimal-linux-x86_64.tar.gz`) has no Python or
network dependency; it only needs the system audio libraries.

Install per distro:

```bash
# Arch Linux
sudo pacman -S ffmpeg python python-pip python-pillow python-mutagen yt-dlp alsa-lib

# Debian / Ubuntu
sudo apt install ffmpeg python3 python3-pip python3-pil python3-mutagen yt-dlp libasound2

# Fedora (ffmpeg is in RPM Fusion)
sudo dnf install ffmpeg python3 python3-pip python3-pillow python3-mutagen yt-dlp alsa-lib
```

If your distro packages are older, install the Python packages with pip:

```bash
python3 -m pip install --user yt-dlp pillow mutagen
```

Check what is missing at any time:

```bash
lyra --check-deps
```

> The Linux binaries are built on Ubuntu and link against its `glibc`. On
> older distros, build from source (see below) instead of using the prebuilt
> archive.

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
- `update`: GitHub Releases update check and self-update for tarball installs

---

## Updating

The Full TUI includes a built-in updater plugin (visible in `[5] Plugins`).
It reads the `latest.json` manifest published with each GitHub release, checks
at most once per day, and shows a toast when a newer version is available.

```bash
lyra --check-update   # check now and print the result
lyra --update         # download, verify and install the latest release
```

Press `U` in the Plugins view to check or install interactively.

Self-update only runs for the Linux x86_64 **tarball** builds installed in a
writable user directory (for example `~/.local/bin/lyra`). Binaries managed by
`pacman`, `apt`, `nix`, or `cargo install` are never overwritten; Lyra asks you
to update through that package manager instead. Downloads are SHA-256 verified
against the release manifest before the atomic replacement. Set
`LYRA_NO_UPDATE_CHECK=1` to disable background checks.

---

## Music Search, Metadata & Lyrics

Search is **metadata-first**:

1. **Official APIs by default** — Spotify (when credentials are configured),
   then MusicBrainz, Deezer and iTunes. These provide canonical artist,
   title, album, year, duration, and cover art. MusicBrainz needs no API key
   and is rate-limited to one request per second.
2. **YouTube is a fallback** — it is only used when the official sources
   return no matches, or when you explicitly select pure YouTube search.
3. **LRCLIB fills in lyrics + metadata** — synced `.lrc` lyrics are fetched
   before download, and the canonical album/metadata is merged into the ID3
   tags. YouTube captions are used only when LRCLIB has no synced lyrics.

When an official result is selected, Lyra resolves it to a matching
YouTube video only at download time, ranked by title/artist relevance and
duration proximity. Search therefore stays fast and YouTube never
outranks the official catalog.

### Choosing a search source

The search bar always shows the active source, e.g. `[Auto (official)]`.
Cycle it with:

- `s` in the Download view (normal mode)
- `Ctrl+S` while typing a query
- `F2` while the search input is focused

Available sources: `auto` (recommended), `spotify`, `musicbrainz`, `deezer`,
`itunes`, `youtube` (pure YouTube search), and `all` (official + YouTube
merged).
The choice is saved to `config.toml` as `search_source`.

### Spotify credentials

Spotify's Web API requires a free developer app (Client Credentials flow):

```bash
export SPOTIFY_CLIENT_ID="your-client-id"
export SPOTIFY_CLIENT_SECRET="your-client-secret"
```

or create `~/.config/lyra/spotify.json`:

```json
{ "client_id": "your-client-id", "client_secret": "your-client-secret" }
```

Without credentials, `auto` uses MusicBrainz + Deezer + iTunes (no keys
needed); choosing `spotify` explicitly shows a setup hint.

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
| `s` | Download view: cycle search source (auto/spotify/musicbrainz/deezer/itunes/youtube/all) |
| `Ctrl+S` | While typing a download query: cycle search source |
| `?` | Toggle keybindings help modal |
| `X` | Cycle theme (mocha → gruvbox → tokyo-night → dracula → nord → latte; saved to config.toml) |
| `Z` | Reload theme from config.toml (use after changing theme via theme-packs plugin) |
| `O` | Sleep timer: off → 15 → 30 → 60 → 90 → 120 → off (pauses playback on expiry) |
| `K` | Playlist import/export modal (M3U; Tab toggles import/export) |
| `b` | Audio visualizer: ON/OFF |
| `Ctrl+V` | Visualizer color: accent → cyan → green → magenta → yellow |
| `Ctrl+B` | Visualizer style: bars → wave → dual → spectrum → dots |
| `q` | Quit |

---

## Configuration & Data Paths

- Main Configuration: `~/.config/lyra/config.toml`
  ```toml
  music_directory = "~/Music"
  volume = 80
  repeat_mode = "playlist"
  theme = "gruvbox"
  cover_embed = true
  visualizer = true
  visualizer_color = "cyan"
  visualizer_style = "wave"
  search_limit = 15
  search_source = "auto"
  lyric_download = true
  ```
- Genre Sidecar: `~/.config/lyra/genres.json`
- Artwork Cache: `~/.cache/lyra/covers/`

**Config validation**: On startup, lyra warns (stderr) if `theme` is not a known preset or if `music_folder`/`download_dir` don't exist. Fix `config.toml` or unset `LYRA_THEME` to clear the warning.

**Visualizer**: Press `b` to toggle. Cycle style with `Ctrl+B` (bars/wave/dual/spectrum/dots) and color with `Ctrl+V` (accent/cyan/green/magenta/yellow). The visualizer appears below lyrics in Karaoke mode, or below spotlight in Studio mode. It is also a toggleable plugin in the Plugins menu (key `5`).

---

## Contributing & Extensions

Lyra welcomes contributions, bug reports, and plugin pull requests.
See [CONTRIBUTING.md](CONTRIBUTING.md) for contribution guidelines, project layout, and testing rules. See [MusicPlayer/plugins.md](MusicPlayer/plugins.md) for compile-time Cargo plugins, and [plugins/](plugins/) with [registry.json](registry.json) for community script add-ons.

---

## License

MIT
