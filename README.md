# lyra

Minimal CLI tool to download audio from YouTube, fetch official studio cover art, and extract synchronized lyrics (`.lrc`).

## Requirements

- Python >= 3.10
- `ffmpeg`
- Python packages: `yt-dlp`, `pillow`, `mutagen` (stdlib only beyond that)

## Usage

```bash
# Download to default directory (./output)
python3 main.py get "<url>"

# Specify output directory
python3 main.py get "<url>" -o ~/Music

# Cover art: studio search is on by default (iTunes + Deezer + Cover Art Archive),
# YouTube thumbnail is the last-resort fallback
python3 main.py get "<url>" --cover-source deezer
python3 main.py get "<url>" --cover-mode center_crop   # YouTube fallback shape
python3 main.py get "<url>" --cover-url "https://..."  # pin a chosen candidate
python3 main.py get "<url>" --no-cover-search          # YouTube thumbnail only

# Search studio covers without downloading (termusic tag-editor style)
python3 main.py cover --artist "Sơn Tùng M-TP" --title "Chúng Ta Của Hiện Tại"
python3 main.py cover "Cruel Summer" --source itunes --limit 3 --json
```

Search results are scored 0-100 (title 62% + artist 33%, diacritics-insensitive);
the best candidate is auto-picked, cached under `~/.cache/lyra/covers/`
(metadata 7 days, images permanently), and normalized to 1000x1000 JPEG.

In the TUI Download view, the metadata form shows cover candidates while you
type (`Auto: <source> <score> [i/n]`); use `←/→` on the Cover row to pick one,
`Enter` keeps Auto (best score wins).

## Output Structure

```
<output>/
├── <Artist> - <Title>.mp3
└── <Artist> - <Title>.lrc
```

- **Audio**: MP3 tagged with ID3v2 (Title, Artist, Album, and embedded 1:1 Front Cover).
- **Lyrics**: Standard `.lrc` file (`[mm:ss.xx]`) placed alongside the audio file for players that support synced lyrics (MPD, Amberol, ncmpcpp, etc.).

## Testing

```bash
python3 -m unittest discover -s tests
```

## License

MIT
