# lyra

Minimal CLI tool to download audio from YouTube, generate 1:1 square album art, and extract synchronized lyrics (`.lrc`).

## Requirements

- Python >= 3.10
- `ffmpeg`
- Python packages: `yt-dlp`, `pillow`, `mutagen`

## Usage

```bash
# Download to default directory (./output)
python3 main.py get "<url>"

# Specify output directory
python3 main.py get "<url>" -o ~/Music

# Optional: select cover art mode ('blur_pad' by default, or 'center_crop')
python3 main.py get "<url>" --cover-mode center_crop
```

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
