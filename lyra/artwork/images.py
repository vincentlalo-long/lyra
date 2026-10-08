"""Image download + normalization to 1000x1000 JPEG (stdlib net + Pillow)."""

import os
import urllib.request
from io import BytesIO
from typing import Optional

from PIL import Image

from . import cache as cache_mod

USER_AGENT = "Lyra/1.0 (cover-art search)"
MAX_BYTES = 15 * 1024 * 1024
TARGET_SIZE = 1000


def _fetch_bytes(url: str, timeout: float = 8.0) -> Optional[bytes]:
    if not url or not url.startswith(("http://", "https://")):
        return None
    # Reuse disk-cached image bytes when available.
    cached_path = cache_mod.image_path_for_url(url)
    if os.path.exists(cached_path) and os.path.getsize(cached_path) > 0:
        try:
            with open(cached_path, "rb") as f:
                return f.read()
        except OSError:
            pass
    try:
        req = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            ctype = resp.headers.get("Content-Type", "")
            if ctype and "image" not in ctype and "octet-stream" not in ctype:
                return None
            length = resp.headers.get("Content-Length")
            if length and int(length) > MAX_BYTES:
                return None
            data = resp.read(MAX_BYTES + 1)
            if not data or len(data) > MAX_BYTES + 1:
                return None
        try:
            with open(cached_path, "wb") as f:
                f.write(data)
        except OSError:
            pass
        return data
    except Exception:
        return None


def download_cover_image(
    url: str,
    output_path: str,
    timeout: float = 8.0,
    target_size: int = TARGET_SIZE,
) -> bool:
    """Download, validate, and normalize to a square JPEG.

    Returns True on success. Falls back to ``cover_url_fallback`` is left
    to the caller (it knows the candidate); this function takes one URL.
    """
    data = _fetch_bytes(url, timeout=timeout)
    if not data:
        return False
    try:
        with Image.open(BytesIO(data)) as img:
            img.load()  # fully decode (verify() alone leaves file unusable)
            img = img.convert("RGB")
            w, h = img.size
            if w <= 0 or h <= 0:
                return False
            side = min(w, h)
            if w != h:
                left = (w - side) // 2
                top = (h - side) // 2
                img = img.crop((left, top, left + side, top + side))
            if side != target_size:
                img = img.resize(
                    (target_size, target_size), Image.Resampling.LANCZOS
                )
            tmp = output_path + ".part"
            img.save(tmp, "JPEG", quality=92)
            os.replace(tmp, output_path)
            return True
    except Exception:
        for p in (output_path, output_path + ".part"):
            try:
                if os.path.exists(p):
                    os.remove(p)
            except OSError:
                pass
        return False
