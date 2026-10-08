"""Backward-compatible facade over :mod:`lyra.artwork`.

Old imports (``search_itunes_cover``, ``download_cover_image`` ...) keep
working; new code should import from ``lyra.artwork`` directly.
"""

from typing import Optional, Dict, Any, List

from PIL import Image, ImageFilter, ImageEnhance

from .artwork import (
    search_candidates,
    search_best_cover,
    download_best_cover,
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
    query: str, limit: int = 5, country: str = "VN"
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


def process_cover_art(
    input_path: str,
    output_path: str,
    mode: str = "blur_pad",
    target_size: int = 1000
) -> bool:
    try:
        with Image.open(input_path) as img:
            img = img.convert("RGB")
            w, h = img.size

            if w == h and w >= target_size:
                # Already square
                img.save(output_path, "JPEG", quality=95)
                return True

            if mode == "center_crop":
                min_dim = min(w, h)
                left = (w - min_dim) // 2
                top = (h - min_dim) // 2
                cropped = img.crop((left, top, left + min_dim, top + min_dim))
                resized = cropped.resize((target_size, target_size), Image.Resampling.LANCZOS)
                resized.save(output_path, "JPEG", quality=95)
                return True

            # Default: blur_pad
            # 1. Background: Cover the square canvas, blur and dim slightly
            bg = img.resize((target_size, target_size), Image.Resampling.BILINEAR)
            bg = bg.filter(ImageFilter.GaussianBlur(radius=35))
            enhancer = ImageEnhance.Brightness(bg)
            bg = enhancer.enhance(0.65)  # Dim background by 35% for contrast

            # 2. Foreground: Fit original image into the square canvas
            ratio = min(target_size / w, target_size / h)
            new_w = int(w * ratio)
            new_h = int(h * ratio)
            fg = img.resize((new_w, new_h), Image.Resampling.LANCZOS)

            # 3. Paste foreground into center of background
            pos_x = (target_size - new_w) // 2
            pos_y = (target_size - new_h) // 2
            bg.paste(fg, (pos_x, pos_y))

            bg.save(output_path, "JPEG", quality=95)
            return True
    except Exception as e:
        import sys
        print(f"warning: failed to process cover art: {e}", file=sys.stderr)
        return False
