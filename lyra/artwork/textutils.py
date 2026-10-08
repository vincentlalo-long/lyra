"""Shared text helpers for cover-art search.

Key difference vs the old ``image.clean_search_term``: we only strip
*video-suffix* tags (``(Official MV)``, ``[4K]`` ...) at the END of the
string and never touch hyphens inside artist names (``M-TP`` stays).
"""

import re
import unicodedata

# Suffix tags: only removed when they appear inside trailing (...) / [...]
# groups at the end of the title, never mid-string.
_SUFFIX_TAG_RE = re.compile(
    r"(?i)[\(\[][^()\[\]]*?"
    r"(official|music\s*video|\bmv\b|m/v|lyric|lyrics|audio|"
    r"vietsub|karaoke|full|hd|4k|8k|60fps|visualizer)"
    r"[^()\[\]]*?[\)\]]\s*$"
)
_TRAILING_PUNCT_RE = re.compile(r"[\s\-–—_|:;\"'`~!?.]+$")
_WS_RE = re.compile(r"\s+")


def strip_suffix_tags(text: str) -> str:
    """Remove trailing ``(Official MV)`` / ``[4K]`` style groups only."""
    if not text:
        return ""
    cleaned = text.strip()
    # Loop: "Song (Official) (4K)" has two stacked suffix groups.
    for _ in range(3):
        new = _SUFFIX_TAG_RE.sub("", cleaned).strip()
        new = _TRAILING_PUNCT_RE.sub("", new).strip()
        if new == cleaned:
            break
        cleaned = new
    return cleaned


def ascii_fold(text: str) -> str:
    """Lowercase + strip diacritics (``Sơn Tùng`` -> ``son tung``), stdlib only."""
    if not text:
        return ""
    norm = unicodedata.normalize("NFKD", text)
    ascii_only = "".join(c for c in norm if not unicodedata.combining(c))
    ascii_only = ascii_only.encode("ascii", "ignore").decode("ascii")
    return _WS_RE.sub(" ", ascii_only.lower()).strip()


def norm_key(text: str) -> str:
    """Normalized comparison key: suffix-strip then ascii-fold."""
    return ascii_fold(strip_suffix_tags(text))
