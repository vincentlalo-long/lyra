"""Backward-compatible facade over :mod:`lyra.artwork`.

Old imports (``search_itunes_cover``, ``download_cover_image`` ...) keep
working; new code should import from ``lyra.artwork`` directly.
"""

from typing import Optional, Dict, Any, List

from .artwork import (
    search_candidates,
    search_best_cover,
    download_best_cover,
    process_cover_art,
    prepare_cover_art,
)
from .artwork.images import download_cover_image as _download_image
from .artwork.scoring import is_relevant as _is_relevant
from .artwork.textutils import strip_suffix_tags

__all__ = [
    "clean_search_term",
    "is_relevant_result",
    "search_itunes_cover",
    "search_itunes_tracks",
    "download_cover_image",
    "process_cover_art",
    "prepare_cover_art",
]


def clean_search_term(text: str) -> str:
    """Legacy helper kept for tests/CLI: suffix-strip + spaces.

    Note: unlike the original implementation this no longer mangles
    hyphens inside names (``M-TP`` stays intact).
    """
    if not text:
        return ""
    import re

    cleaned = strip_suffix_tags(text)
    # "Artist - Title" separator -> space, but keep inner hyphens (M-TP).
    cleaned = re.sub(r"\s+-\s+", " ", cleaned)
    cleaned = re.sub(r"[–—/\\()\[\]{}:\"'|]+", " ", cleaned)
    cleaned = re.sub(r"\s+", " ", cleaned).strip()
    return cleaned


def is_relevant_result(
    query_artist: str, query_title: str, res_artist: str, res_title: str
) -> bool:
    return _is_relevant(query_artist, query_title, res_artist, res_title)


def search_itunes_cover(
    artist: str,
    title: str,
    timeout: float = 5.0,
) -> Optional[Dict[str, Any]]:
    """Legacy single-best iTunes lookup (now scored + cached)."""
    _ = timeout  # per-provider timeout lives in artwork.providers
    return search_best_cover(artist, title, sources=("itunes",))


def search_itunes_tracks(
    query: str, limit: int = 15, country: str = "VN"
) -> List[Dict[str, Any]]:
    """Legacy multi-track search: treat query as title-only iTunes lookup."""
    _ = country  # global catalog covers VN now
    cands = search_candidates("", query, sources=("itunes",), limit=limit)
    results = []
    for c in cands:
        results.append({
            "id": None,
            "artist": c.get("artist", ""),
            "title": c.get("title", ""),
            "album": c.get("album", ""),
            "year": c.get("year", ""),
            "cover_url": c.get("cover_url", ""),
        })
    return results


def download_cover_image(url: str, output_path: str, timeout: float = 8.0) -> bool:
    return _download_image(url, output_path, timeout=timeout)
