"""Lyra Vietnamese Support Plugin (vi-vietnamese-support) v0.1.0.

Community plugin for Vietnamese (V-Pop) tracks. What it provides:

1. Synced lyrics lookup via LRCLIB (free, no API key). Tries three
   queries in order: accented "artist title" -> title only ->
   ascii-folded (unaccented) fallback such as "Son Tung Lac Troi".
   Best candidate is scored (synced bonus + length + duration).
2. Diacritics-insensitive matching for search/filter:
   "son tung" matches "Sơn Tùng M-TP".
3. NFC normalization so ratatui/unicode-width renders combined
   diacritics correctly (one precomposed char = one column, avoids
   NFD column breakage).
4. Language hint for ID3 USLT tagging: "vie" when Vietnamese
   diacritics are detected, otherwise "eng" (see lyra/tagger.py).

Standard Lyra interface: `run(payload) -> {status, message, data}`.
No Rust core recompile needed — registered via registry.json, read by
MusicPlayer/src/plugin.rs::available_community_plugins().

What this plugin does NOT do: full TUI translation, Zing/NCT
providers, or OCR for hardcoded (burned-in) video lyrics — those stay
with the YouTube caption pipeline (lyra/lyric.py) and ocr-video-lyrics.
"""

from __future__ import annotations

import re
import unicodedata
import urllib.parse
import urllib.request
import json
from typing import Any, Dict, List, Optional

PLUGIN_ID = "vi-vietnamese-support"
PLUGIN_VERSION = "v0.1.0"

LRCLIB_SEARCH_URL = "https://lrclib.net/api/search"
HTTP_TIMEOUT = 10

# Vietnamese diacritics set for fast language detection (lang="vie").
_VI_CHARS = set("àáạảãâầấậẩẫăằắặẳẵèéẹẻẽêềếệểễìíịỉĩòóọỏõôồốộổỗơờớợởỡùúụủũưừứựửữỳýỵỷỹđĐ")


def normalize_nfc(text: str) -> str:
    """Normalize to NFC (precomposed) so one char equals one terminal column."""
    return unicodedata.normalize("NFC", text or "")


def ascii_fold(text: str) -> str:
    """Lowercase + strip diacritics: 'Sơn Tùng' -> 'son tung'. For search."""
    if not text:
        return ""
    norm = unicodedata.normalize("NFKD", text)
    stripped = "".join(c for c in norm if not unicodedata.combining(c))
    # đ/Đ do not decompose via NFKD, map them manually.
    stripped = stripped.replace("đ", "d").replace("Đ", "D")
    ascii_only = stripped.encode("ascii", "ignore").decode("ascii")
    return re.sub(r"\s+", " ", ascii_only.lower()).strip()


def is_vietnamese(text: str) -> bool:
    """Return True when the text contains Vietnamese diacritics."""
    if not text:
        return False
    return any(c in _VI_CHARS for c in text)


def match_no_diacritics(query: str, target: str) -> bool:
    """Two-way accent-insensitive contains: 'son tung' matches 'Sơn Tùng M-TP'."""
    q, t = ascii_fold(query), ascii_fold(target)
    return bool(q) and q in t


def _http_get_json(url: str) -> Optional[Any]:
    try:
        req = urllib.request.Request(url, headers={"User-Agent": "lyra-vi-plugin/0.1.0"})
        with urllib.request.urlopen(req, timeout=HTTP_TIMEOUT) as resp:
            return json.loads(resp.read().decode("utf-8", errors="ignore"))
    except Exception:
        return None


def search_lrclib(artist: str, title: str) -> List[Dict[str, Any]]:
    """Try 3 queries: accented 'artist title' -> title -> unaccented version."""
    candidates: List[Dict[str, Any]] = []
    seen = set()
    queries = [
        f"{artist} {title}".strip(),
        title.strip(),
        f"{ascii_fold(artist)} {ascii_fold(title)}".strip(),
    ]
    for q in queries:
        if not q:
            continue
        url = f"{LRCLIB_SEARCH_URL}?{urllib.parse.urlencode({'q': q})}"
        data = _http_get_json(url)
        if isinstance(data, list):
            for item in data:
                key = (item.get("id") or item.get("trackName"), item.get("artistName"))
                if key not in seen:
                    seen.add(key)
                    candidates.append(item)
        if candidates:
            break
    return candidates


def pick_best_lrc(candidates: List[Dict[str, Any]]) -> Optional[Dict[str, Any]]:
    """Prefer entries with syncedLyrics, longer content, and valid duration."""
    if not candidates:
        return None
    synced = [c for c in candidates if c.get("syncedLyrics")]
    pool = synced or candidates

    def score(c: Dict[str, Any]) -> int:
        s = 0
        txt = c.get("syncedLyrics") or c.get("plainLyrics") or ""
        if c.get("syncedLyrics"):
            s += 10
        s += min(txt.count("\n"), 20)
        if c.get("duration"):
            s += 2
        return s

    return max(pool, key=score)


def fetch_lyrics(artist: str, title: str) -> Dict[str, Any]:
    """Main pipeline: LRCLIB search -> best LRC text + language hint."""
    artist_n = normalize_nfc(artist or "")
    title_n = normalize_nfc(title or "")
    candidates = search_lrclib(artist_n, title_n)
    best = pick_best_lrc(candidates)
    if not best:
        return {"found": False, "lrc_text": None, "lang": None, "source": "lrclib",
                "candidates": len(candidates)}
    lrc = best.get("syncedLyrics") or best.get("plainLyrics")
    lang = "vie" if (is_vietnamese(f"{artist_n} {title_n} {lrc or ''}")) else "eng"
    return {
        "found": bool(lrc),
        "lrc_text": lrc,
        "lang": lang,
        "source": "lrclib",
        "track": best.get("trackName"),
        "artist": best.get("artistName"),
        "duration": best.get("duration"),
        "candidates": len(candidates),
    }


# ---- Lyra plugin entrypoint ----

def run(payload: Dict[str, Any]) -> Dict[str, Any]:
    """Execute a plugin action.

    Supported payloads:
      {"action": "lyrics", "artist": "...", "title": "..."}  (default)
      {"action": "match", "query": "...", "target": "..."}
      {"action": "normalize", "text": "..."}
      {"action": "detect", "text": "..."}
      {"action": "list"}  (capability probe used by the TUI Store runner)
    """
    action = (payload or {}).get("action", "lyrics")
    try:
        if action == "list":
            return {"status": "success",
                    "message": "vi-vietnamese-support v0.1.0: lyrics/match/normalize/detect for Vietnamese tracks (LRCLIB, no key)",
                    "data": {"id": PLUGIN_ID, "version": PLUGIN_VERSION,
                             "actions": ["lyrics", "match", "normalize", "detect", "list"]}}
        if action == "match":
            ok = match_no_diacritics(payload.get("query", ""), payload.get("target", ""))
            return {"status": "success",
                    "message": "match" if ok else "no-match",
                    "data": {"match": ok,
                             "query_fold": ascii_fold(payload.get("query", "")),
                             "target_fold": ascii_fold(payload.get("target", ""))}}
        if action == "normalize":
            text = normalize_nfc(payload.get("text", ""))
            return {"status": "success", "message": "normalized",
                    "data": {"text": text, "fold": ascii_fold(text),
                             "is_vietnamese": is_vietnamese(text)}}
        if action == "detect":
            text = payload.get("text", "")
            return {"status": "success", "message": "detected",
                    "data": {"is_vietnamese": is_vietnamese(text),
                             "suggest_lang": "vie" if is_vietnamese(text) else "eng"}}
        # default: lyrics lookup
        artist = (payload or {}).get("artist", "")
        title = (payload or {}).get("title", "")
        if not artist and not title and (payload or {}).get("query"):
            # Shorthand: {"query": "Son Tung Lac Troi"}
            title = payload.get("query", "")
        result = fetch_lyrics(artist, title)
        if result["found"]:
            return {"status": "success",
                    "message": f"Found lyrics via LRCLIB ({result['lang']})",
                    "data": result}
        return {"status": "not_found",
                "message": "No synced lyrics on LRCLIB; try YouTube captions "
                           "(lyra/lyric.py fetch_all_vtt vi->en) or OCR for burned-in video lyrics",
                "data": result}
    except Exception as e:  # Never crash the host.
        return {"status": "error", "message": f"vi-plugin error: {e}", "data": {}}
