"""Hand-curated genre tags (sidecar) + one-shot iTunes backfill.

The sidecar (`~/.config/lyra/genres.json`, `{abs_path: [genres]}`) covers the
existing library and personal mood tags, where ID3 TCON (written for new
downloads) is missing or too coarse. Consumers merge with sidecar winning.
"""

import json
import os
from typing import Dict, List, Optional


def sidecar_path() -> str:
    base = os.environ.get("XDG_CONFIG_HOME") or os.path.join(
        os.path.expanduser("~"), ".config"
    )
    d = os.path.join(base, "lyra")
    os.makedirs(d, exist_ok=True)
    return os.path.join(d, "genres.json")


def load_sidecar(path: Optional[str] = None) -> Dict[str, List[str]]:
    p = path or sidecar_path()
    try:
        with open(p, encoding="utf-8") as f:
            data = json.load(f)
        if not isinstance(data, dict):
            return {}
        out: Dict[str, List[str]] = {}
        for k, v in data.items():
            if isinstance(v, list):
                out[str(k)] = [str(g).strip() for g in v if str(g).strip()]
            elif isinstance(v, str) and v.strip():
                out[str(k)] = [v.strip()]
        return out
    except (OSError, ValueError):
        return {}


def save_sidecar(mapping: Dict[str, List[str]], path: Optional[str] = None) -> str:
    p = path or sidecar_path()
    with open(p, "w", encoding="utf-8") as f:
        json.dump(mapping, f, ensure_ascii=False, indent=2, sort_keys=True)
    return p


def set_genres(mp3_path: str, genres: List[str], path: Optional[str] = None) -> List[str]:
    """Replace the genre list for one file. Empty list removes the entry."""
    mapping = load_sidecar(path)
    key = os.path.abspath(os.path.expanduser(mp3_path))
    cleaned = [g.strip() for g in genres if g.strip()]
    if cleaned:
        mapping[key] = cleaned
    else:
        mapping.pop(key, None)
    save_sidecar(mapping, path)
    return cleaned


def get_genres(mp3_path: str, path: Optional[str] = None) -> List[str]:
    key = os.path.abspath(os.path.expanduser(mp3_path))
    return load_sidecar(path).get(key, [])


def split_artist_title(filename: str):
    """Best-effort 'Artist - Title.mp3' split for backfill queries."""
    stem = os.path.splitext(os.path.basename(filename))[0]
    if " - " in stem:
        artist, title = stem.split(" - ", 1)
        return artist.strip(), title.strip()
    return "", stem.strip()


def backfill_directory(
    music_dir: str,
    limit: Optional[int] = None,
    path: Optional[str] = None,
    progress=None,
) -> Dict[str, int]:
    """One-shot: query iTunes for mp3s lacking a sidecar entry and store the
    returned genre. Never modifies audio files. Returns stats."""
    from .image import search_itunes_cover

    stats = {"scanned": 0, "filled": 0, "skipped": 0, "missed": 0}
    mapping = load_sidecar(path)
    targets = []
    for root, _dirs, files in os.walk(os.path.abspath(os.path.expanduser(music_dir))):
        for fn in sorted(files):
            if not fn.lower().endswith(".mp3"):
                continue
            full = os.path.join(root, fn)
            stats["scanned"] += 1
            if full in mapping and mapping[full]:
                stats["skipped"] += 1
                continue
            targets.append(full)
            if limit is not None and len(targets) >= limit:
                break
        if limit is not None and len(targets) >= limit:
            break

    for full in targets:
        artist, title = split_artist_title(full)
        try:
            info = search_itunes_cover(artist, title) if title else None
        except Exception:
            info = None
        genre = (info or {}).get("genre", "").strip()
        if genre:
            mapping[full] = [genre]
            stats["filled"] += 1
        else:
            stats["missed"] += 1
        if progress:
            progress(full, genre)
    save_sidecar(mapping, path)
    return stats
