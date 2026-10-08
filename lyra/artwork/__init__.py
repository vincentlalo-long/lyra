"""Public API for cover-art search (termusic songtag-style multi-provider)."""

from typing import Any, Dict, List, Optional

from . import cache as cache_mod
from .providers import search_candidates, prefetch_candidates
from .images import download_cover_image, process_cover_art, prepare_cover_art, TARGET_SIZE
from .scoring import ACCEPT_THRESHOLD, AUTO_PICK_THRESHOLD

__all__ = [
    "search_candidates",
    "search_best_cover",
    "prefetch_candidates",
    "download_cover_image",
    "download_best_cover",
    "process_cover_art",
    "prepare_cover_art",
    "ACCEPT_THRESHOLD",
    "AUTO_PICK_THRESHOLD",
    "TARGET_SIZE",
]


def search_best_cover(
    artist: str,
    title: str,
    sources: tuple = ("itunes", "deezer", "caa"),
    min_score: float = ACCEPT_THRESHOLD,
    use_cache: bool = True,
) -> Optional[Dict[str, Any]]:
    """Return the single best candidate, or None below ``min_score``."""
    cands = search_candidates(
        artist, title, sources=sources, limit=5, use_cache=use_cache
    )
    if not cands or cands[0].get("score", 0) < min_score:
        return None
    return cands[0]


def download_best_cover(
    artist: str,
    title: str,
    output_path: str,
    sources: tuple = ("itunes", "deezer", "caa"),
    use_cache: bool = True,
) -> Optional[Dict[str, Any]]:
    """Search, download and normalize the best cover.

    Tries candidates in score order, including each candidate's
    ``cover_url_fallback`` (e.g. iTunes 600px when 1000px 404s).
    Returns the winning candidate dict on success, else None.
    """
    cands = search_candidates(
        artist, title, sources=sources, limit=5, use_cache=use_cache
    )
    for c in cands:
        for url_key in ("cover_url", "cover_url_fallback"):
            url = c.get(url_key)
            if url and download_cover_image(url, output_path):
                return c
    return None
