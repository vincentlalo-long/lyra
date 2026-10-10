"""Image download + normalization to 1000x1000 JPEG (stdlib net + Pillow)."""

import os
import urllib.request
from io import BytesIO
from typing import Optional

from PIL import Image, ImageFilter, ImageEnhance

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


def process_cover_art(
    input_path: str,
    output_path: str,
    mode: str = "blur_pad",
    target_size: int = TARGET_SIZE,
    focus: Optional[tuple] = None,
) -> bool:
    """Normalize local image to 1000x1000 square JPEG with blur_pad or center_crop.

    ``focus`` is an optional (x, y) pair in 0..1 units selecting the
    center of the crop window for ``center_crop`` (default: image center).
    """
    try:
        with Image.open(input_path) as img:
            img = img.convert("RGB")
            w, h = img.size

            if w == h and w >= target_size:
                img.save(output_path, "JPEG", quality=95)
                return True

            if mode == "center_crop":
                min_dim = min(w, h)
                fx, fy = 0.5, 0.5
                if focus:
                    try:
                        fx = min(1.0, max(0.0, float(focus[0])))
                        fy = min(1.0, max(0.0, float(focus[1])))
                    except (TypeError, ValueError, IndexError):
                        pass
                max_left = w - min_dim
                max_top = h - min_dim
                left = int(round(max_left * fx))
                top = int(round(max_top * fy))
                cropped = img.crop((left, top, left + min_dim, top + min_dim))
                resized = cropped.resize((target_size, target_size), Image.Resampling.LANCZOS)
                resized.save(output_path, "JPEG", quality=95)
                return True

            # Default: blur_pad
            bg = img.resize((target_size, target_size), Image.Resampling.BILINEAR)
            bg = bg.filter(ImageFilter.GaussianBlur(radius=35))
            enhancer = ImageEnhance.Brightness(bg)
            bg = enhancer.enhance(0.65)

            ratio = min(target_size / w, target_size / h)
            new_w = int(w * ratio)
            new_h = int(h * ratio)
            fg = img.resize((new_w, new_h), Image.Resampling.LANCZOS)

            pos_x = (target_size - new_w) // 2
            pos_y = (target_size - new_h) // 2
            bg.paste(fg, (pos_x, pos_y))

            bg.save(output_path, "JPEG", quality=95)
            return True
    except Exception as e:
        import sys
        print(f"warning: failed to process cover art: {e}", file=sys.stderr)
        return False


def prepare_cover_art(
    source: str,
    output_path: str,
    mode: str = "blur_pad",
    target_size: int = TARGET_SIZE,
    focus: Optional[tuple] = None,
) -> bool:
    """Prepare a square 1:1 cover art from either a local file or a remote URL."""
    if not source:
        return False
    source = source.strip()
    if source.startswith(("http://", "https://")):
        return download_cover_image(source, output_path, target_size=target_size)

    local_path = source[7:] if source.startswith("file://") else source
    local_path = os.path.abspath(os.path.expanduser(local_path))
    if os.path.isfile(local_path):
        return process_cover_art(local_path, output_path, mode=mode, target_size=target_size, focus=focus)
    return False
