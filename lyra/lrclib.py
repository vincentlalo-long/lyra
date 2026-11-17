"""LRCLIB client: canonical metadata + synced lyrics.

LRCLIB is a free, community-maintained lyrics database. We use it for
two things:

* ``best_match`` — resolve a track's canonical title/artist/album/duration
  and know whether synced (``.lrc``) lyrics exist before downloading.
* ``search`` — enumerate synced/plain lyric matches for the TUI lyric
  picker (already exposed through the ``lyric-search`` CLI command).

All helpers are network-tolerant: any failure returns ``None`` / ``[]``
instead of raising, so a missing LRCLIB must never block a download.
"""

import json
import urllib.parse
import urllib.request
from typing import Any, Dict, List, Optional

from .artwork.scoring import score_candidate

USER_AGENT = "Lyra/1.0 (https://github.com/vincentlalo-long/lyra)"
TIMEOUT_S = 8.0
BASE_URL = "https://lrclib.net/api"


def _get_json(url: str, timeout: float = TIMEOUT_S) -> Any:
    req = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
    with urllib.request.urlopen(req, timeout=timeout) as resp:
        return json.loads(resp.read().decode("utf-8", errors="ignore"))


def _record(raw: Dict[str, Any]) -> Dict[str, Any]:
    """Normalize LRCLIB's camelCase payload into our flat shape."""
    synced = (raw.get("syncedLyrics") or "").strip()
    plain = (raw.get("plainLyrics") or "").strip()
    return {
        "id": raw.get("id"),
        "artist": raw.get("artistName", ""),
        "title": raw.get("trackName", ""),
        "album": raw.get("albumName", ""),
        "duration": int(raw.get("duration") or 0),
        "synced_lyrics": synced,
        "plain_lyrics": plain,
        "instrumental": bool(raw.get("instrumental")),
        "has_synced": bool(synced),
        "has_plain": bool(plain),
    }


def get_exact(
    artist: str,
    title: str,
    album: str = "",
    duration: Optional[int] = None,
) -> Optional[Dict[str, Any]]:
    """Exact ``/api/get`` lookup; ``None`` on 404/network error."""
    if not artist or not title:
        return None
    params = {
        "artist_name": artist,
        "track_name": title,
    }
    if album:
        params["album_name"] = album
    if duration:
        params["duration"] = str(int(duration))
    url = f"{BASE_URL}/get?" + urllib.parse.urlencode(params)
    try:
        return _record(_get_json(url))
    except Exception:
        return None


def search(query: str, limit: int = 10) -> List[Dict[str, Any]]:
    """Free-text ``/api/search`` (``q``) lookup, best matches first."""
    query = (query or "").strip()
    if not query:
        return []
    url = f"{BASE_URL}/search?" + urllib.parse.urlencode({"q": query})
    try:
        data = _get_json(url)
    except Exception:
        return []
    if not isinstance(data, list):
        return []
    out = [_record(r) for r in data if isinstance(r, dict)]
    return out[:limit]


def _duration_delta(a: int, b: Optional[int]) -> Optional[int]:
    if not a or not b:
        return None
    return abs(int(a) - int(b))


def best_match(
    artist: str,
    title: str,
    album: str = "",
    duration: Optional[int] = None,
) -> Optional[Dict[str, Any]]:
    """Best LRCLIB record for a track.

    Tries the exact endpoint first (fast, 404 when absent), then falls
    back to free-text search and ranks results by title/artist relevance
    plus duration closeness.
    """
    artist = (artist or "").strip()
    title = (title or "").strip()
    if not title:
        return None

    exact = get_exact(artist, title, album, duration)
    if exact:
        delta = _duration_delta(exact.get("duration", 0), duration)
        # Trust an exact hit only when the duration agrees (LRCLIB's
        # /get is strict on title/artist, duration is advisory).
        if delta is None or delta <= 3:
            return exact
        exact_score = 100.0 - min(delta, 50)
    else:
        exact_score = 0.0

    candidates = search(f"{artist} {title}".strip(), limit=10)
    if not candidates:
        return exact

    best = None
    best_score = exact_score
    for cand in candidates:
        score = score_candidate(
            artist, title, cand.get("artist", ""), cand.get("title", "")
        )
        delta = _duration_delta(cand.get("duration", 0), duration)
        if delta is not None:
            if delta > 10:
                score -= 20.0
            elif delta > 3:
                score -= 8.0
            else:
                score += 5.0
        if cand.get("has_synced"):
            score += 3.0
        if score > best_score:
            best_score = score
            best = cand
    return best or exact


def probe(
    artist: str,
    title: str,
    album: str = "",
    duration: Optional[int] = None,
) -> Dict[str, Any]:
    """Availability probe shaped for the TUI's Lyrics badge.

    ``has_manual`` means synced lyrics (a real ``.lrc`` can be written);
    ``has_auto`` means only plain text exists. ``langs`` is kept empty
    because LRCLIB does not expose per-language variants.
    """
    rec = best_match(artist, title, album, duration)
    if not rec:
        return {"has_manual": False, "has_auto": False, "langs": []}
    return {
        "has_manual": bool(rec.get("has_synced")),
        "has_auto": bool(rec.get("has_synced") or rec.get("has_plain")),
        "langs": [],
        "album": rec.get("album", ""),
        "duration": rec.get("duration", 0),
    }


def to_lrc(record: Dict[str, Any]) -> str:
    """Return synced lyrics only; plain text is not a valid ``.lrc``."""
    return (record.get("synced_lyrics") or "").strip()
