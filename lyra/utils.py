import re
from typing import Tuple


def clean_title_and_artist(raw_title: str, uploader: str = "") -> Tuple[str, str]:
    # Extract clean (artist, title) from raw video titles
    cleaned = raw_title

    # Remove common video tags
    patterns = [
        r"\[(official|mv|music video|audio|hd|4k|lyric video|lyrics|vietsub|karaoke).*?\]",
        r"\((official|mv|music video|audio|hd|4k|lyric video|lyrics|vietsub|karaoke|full).*?\)",
        r"【(mv|official|music video|audio).*?】",
        r"「(mv|official|music video|audio).*?」",
        r"\|\s*(official|mv|music video).*?$",
        r"-\s*(official|mv|music video).*?$",
    ]

    for pat in patterns:
        cleaned = re.sub(pat, "", cleaned, flags=re.IGNORECASE)

    cleaned = cleaned.strip()

    artist = ""
    title = cleaned

    # Split by hyphen if present: "Artist - Title"
    split_patterns = [" - ", " – ", " — ", " / "]
    for delim in split_patterns:
        if delim in cleaned:
            parts = cleaned.split(delim, 1)
            artist = parts[0].strip()
            title = parts[1].strip()
            break

    # Fallback to uploader channel if artist is missing
    if not artist and uploader:
        artist = re.sub(r"\s*-\s*Topic$", "", uploader, flags=re.IGNORECASE)
        artist = re.sub(r"\s*(Official|VEVO|Channel)$", "", artist, flags=re.IGNORECASE).strip()

    title = title.strip(" '\"`")
    artist = artist.strip(" '\"`")

    if not artist:
        artist = "Unknown Artist"
    if not title:
        title = raw_title

    return artist, title


def sanitize_filename(name: str) -> str:
    # Remove filesystem-illegal characters
    sanitized = re.sub(r'[\\/*?:"<>|]', "", name)
    sanitized = re.sub(r"\s+", " ", sanitized).strip()
    return sanitized or "untitled"
