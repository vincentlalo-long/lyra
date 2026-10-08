"""Disk + memory cache for cover-art search results and images."""

import hashlib
import json
import os
import time
from typing import Any, Dict, List, Optional

META_TTL_SECONDS = 7 * 24 * 3600  # search metadata lives 7 days

#: Bump when candidate schema/scoring changes so stale entries are ignored.
CACHE_VERSION = 2


def _base_dir() -> str:
    root = os.environ.get("XDG_CACHE_HOME") or os.path.expanduser("~/.cache")
    path = os.path.join(root, "lyra", "covers")
    os.makedirs(path, exist_ok=True)
    return path


def cache_key(artist: str, title: str, sources: tuple = ()) -> str:
    norm = (
        f"v{CACHE_VERSION}|{(artist or '').strip().lower()}"
        f"|{(title or '').strip().lower()}|{','.join(sorted(sources))}"
    )
    return hashlib.sha1(norm.encode("utf-8")).hexdigest()


def meta_path(artist: str, title: str, sources: tuple = ()) -> str:
    return os.path.join(_base_dir(), f"{cache_key(artist, title, sources)}.json")


def image_path_for_url(url: str) -> str:
    digest = hashlib.sha1(url.encode("utf-8")).hexdigest()
    return os.path.join(_base_dir(), f"img_{digest}.jpg")


def load_meta(
    artist: str, title: str, sources: tuple = ()
) -> Optional[List[Dict[str, Any]]]:
    path = meta_path(artist, title, sources)
    try:
        if not os.path.exists(path):
            return None
        if time.time() - os.path.getmtime(path) > META_TTL_SECONDS:
            return None
        with open(path, "r", encoding="utf-8") as f:
            data = json.load(f)
        items = data.get("items")
        return items if isinstance(items, list) else None
    except Exception:
        return None


def save_meta(
    artist: str, title: str, items: List[Dict[str, Any]], sources: tuple = ()
) -> None:
    try:
        with open(meta_path(artist, title, sources), "w", encoding="utf-8") as f:
            json.dump({"items": items}, f, ensure_ascii=False)
    except Exception:
        pass
