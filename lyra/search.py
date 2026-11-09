"""Multi-source music search.

Search is metadata-first:

* ``spotify`` — official Spotify Web API (needs ``client_id``/``secret``)
* ``deezer``  — official Deezer API (no credentials)
* ``itunes``  — official Apple iTunes Search API (no credentials)
* ``musicbrainz`` — MusicBrainz recording search (no credentials; canonical metadata)
* ``youtube`` — yt-dlp flat search (playable/downloadable URL included)
* ``auto``    — all available official sources, YouTube only when they
  return nothing.

Official results carry canonical metadata (artist, title, album, year,
cover, duration) but no playable stream, so :func:`resolve` maps the
chosen track to a matching YouTube video only when the user actually
downloads it. Lyrics + final metadata come from LRCLIB.
"""

import base64
import json
import os
import threading
import time
import urllib.parse
import urllib.request
from concurrent.futures import ThreadPoolExecutor
from typing import Any, Dict, List, Optional

from . import lrclib
from .artwork.scoring import score_candidate

USER_AGENT = "Lyra/1.0 (https://github.com/vincentlalo-long/lyra)"
TIMEOUT_S = 8.0

SOURCES = ("auto", "spotify", "musicbrainz", "deezer", "itunes", "youtube", "all")
OFFICIAL_SOURCES = ("spotify", "musicbrainz", "deezer", "itunes")

#: Larger bonus = stronger priority when merging official providers.
_PROVIDER_BONUS = {"spotify": 18.0, "musicbrainz": 10.0, "deezer": 8.0, "itunes": 2.0}


def _get_json(url: str, timeout: float = TIMEOUT_S, headers: Optional[dict] = None) -> Any:
    req = urllib.request.Request(url, headers={"User-Agent": USER_AGENT, **(headers or {})})
    with urllib.request.urlopen(req, timeout=timeout) as resp:
        return json.loads(resp.read().decode("utf-8", errors="ignore"))


def _fmt_duration(seconds: Any) -> str:
    try:
        total = int(seconds or 0)
    except (TypeError, ValueError):
        total = 0
    return f"{total // 60:02d}:{total % 60:02d}"


def _item(
    *,
    source: str,
    item_id: str,
    title: str,
    artist: str,
    album: str = "",
    year: str = "",
    duration_secs: int = 0,
    cover_url: str = "",
    url: str = "",
    uploader: str = "",
    isrc: str = "",
) -> Dict[str, Any]:
    return {
        "id": item_id,
        "url": url,
        "raw_title": f"{artist} - {title}".strip(" -"),
        "title": title,
        "artist": artist,
        "uploader": uploader or album or source.title(),
        "album": album,
        "year": year,
        "duration": _fmt_duration(duration_secs),
        "duration_secs": duration_secs,
        "cover_url": cover_url,
        "isrc": isrc,
        "source": source,
    }


# ------------------------------------------------------------- Spotify

_spotify_cache: Dict[str, Any] = {"token": None, "expires_at": 0.0}


def _spotify_credentials() -> Optional[tuple]:
    cid = os.environ.get("SPOTIFY_CLIENT_ID") or os.environ.get("LYRA_SPOTIFY_CLIENT_ID")
    secret = os.environ.get("SPOTIFY_CLIENT_SECRET") or os.environ.get("LYRA_SPOTIFY_CLIENT_SECRET")
    if cid and secret:
        return cid, secret

    home = os.path.expanduser("~")
    candidates = [
        os.path.join(os.environ.get("XDG_CONFIG_HOME", os.path.join(home, ".config")), "lyra", "spotify.json"),
        os.path.join(home, ".config", "lyra", "spotify.json"),
    ]
    for path in candidates:
        try:
            with open(path, "r", encoding="utf-8") as f:
                data = json.load(f)
            cid = data.get("client_id") or data.get("clientId")
            secret = data.get("client_secret") or data.get("clientSecret")
            if cid and secret:
                return cid, secret
        except Exception:
            continue

    # Optional: read from config.toml when Python has tomllib (3.11+).
    try:
        import tomllib

        config_path = os.path.join(
            os.environ.get("XDG_CONFIG_HOME", os.path.join(home, ".config")),
            "lyra",
            "config.toml",
        )
        with open(config_path, "rb") as f:
            data = tomllib.load(f)
        cid = data.get("spotify_client_id")
        secret = data.get("spotify_client_secret")
        if cid and secret:
            return cid, secret
    except Exception:
        pass
    return None


def spotify_available() -> bool:
    return _spotify_credentials() is not None


def _spotify_token() -> str:
    creds = _spotify_credentials()
    if not creds:
        raise RuntimeError(
            "Spotify needs API credentials. Set SPOTIFY_CLIENT_ID / SPOTIFY_CLIENT_SECRET "
            "or create ~/.config/lyra/spotify.json with {\"client_id\": ..., \"client_secret\": ...}."
        )
    now = time.time()
    if _spotify_cache["token"] and now < float(_spotify_cache["expires_at"]) - 30:
        return str(_spotify_cache["token"])

    client_id, client_secret = creds
    auth = base64.b64encode(f"{client_id}:{client_secret}".encode()).decode()
    body = urllib.parse.urlencode({"grant_type": "client_credentials"}).encode()
    req = urllib.request.Request(
        "https://accounts.spotify.com/api/token",
        data=body,
        headers={
            "Authorization": f"Basic {auth}",
            "Content-Type": "application/x-www-form-urlencoded",
            "User-Agent": USER_AGENT,
        },
        method="POST",
    )
    with urllib.request.urlopen(req, timeout=TIMEOUT_S) as resp:
        data = json.loads(resp.read().decode("utf-8", errors="ignore"))
    _spotify_cache["token"] = data.get("access_token")
    _spotify_cache["expires_at"] = now + float(data.get("expires_in", 3600))
    if not _spotify_cache["token"]:
        raise RuntimeError("Spotify token request failed")
    return str(_spotify_cache["token"])


def _search_spotify(query: str, limit: int) -> List[Dict[str, Any]]:
    token = _spotify_token()
    url = "https://api.spotify.com/v1/search?" + urllib.parse.urlencode(
        {"q": query, "type": "track", "limit": max(1, min(limit, 50))}
    )
    data = _get_json(url, headers={"Authorization": f"Bearer {token}"})
    tracks = ((data.get("tracks") or {}).get("items")) or []
    out: List[Dict[str, Any]] = []
    for t in tracks:
        if not isinstance(t, dict):
            continue
        album = t.get("album") or {}
        images = album.get("images") or []
        cover = ""
        if images:
            # Largest image first (Spotify returns desc by size).
            cover = max(images, key=lambda i: i.get("width") or 0).get("url", "")
        artists = ", ".join(a.get("name", "") for a in (t.get("artists") or []) if a.get("name"))
        out.append(
            _item(
                source="spotify",
                item_id=f"spotify:{t.get('id', '')}",
                title=t.get("name", ""),
                artist=artists,
                album=album.get("name", ""),
                year=(album.get("release_date") or "")[:4],
                duration_secs=int((t.get("duration_ms") or 0) // 1000),
                cover_url=cover,
                url=t.get("external_urls", {}).get("spotify", ""),
                isrc=(t.get("external_ids") or {}).get("isrc", ""),
            )
        )
    return out


# -------------------------------------------------------------- Deezer

def _search_deezer(query: str, limit: int) -> List[Dict[str, Any]]:
    url = "https://api.deezer.com/search/track?" + urllib.parse.urlencode(
        {"q": query, "limit": max(1, min(limit, 50))}
    )
    data = _get_json(url)
    out: List[Dict[str, Any]] = []
    for t in data.get("data", []) or []:
        if not isinstance(t, dict):
            continue
        album = t.get("album") or {}
        cover = album.get("cover_xl") or album.get("cover_big") or album.get("cover_medium") or ""
        out.append(
            _item(
                source="deezer",
                item_id=f"deezer:{t.get('id', '')}",
                title=t.get("title", ""),
                artist=(t.get("artist") or {}).get("name", ""),
                album=album.get("title", ""),
                duration_secs=int(t.get("duration") or 0),
                cover_url=cover,
                url=t.get("link", ""),
                isrc=t.get("isrc", ""),
            )
        )
    return out


# -------------------------------------------------------------- iTunes

def _search_itunes(query: str, limit: int) -> List[Dict[str, Any]]:
    url = "https://itunes.apple.com/search?" + urllib.parse.urlencode(
        {"term": query, "entity": "song", "limit": max(1, min(limit, 50))}
    )
    data = _get_json(url)
    out: List[Dict[str, Any]] = []
    for t in data.get("results", []) or []:
        if not isinstance(t, dict):
            continue
        art = t.get("artworkUrl100") or ""
        cover = art.replace("100x100bb", "1000x1000bb") if art else ""
        out.append(
            _item(
                source="itunes",
                item_id=f"itunes:{t.get('trackId') or t.get('collectionId') or ''}",
                title=t.get("trackName", ""),
                artist=t.get("artistName", ""),
                album=t.get("collectionName", ""),
                year=(t.get("releaseDate") or "")[:4],
                duration_secs=int((t.get("trackTimeMillis") or 0) // 1000),
                cover_url=cover,
                url=t.get("trackViewUrl", ""),
                isrc=t.get("isrc", ""),
            )
        )
    return out


# --------------------------------------------------------- MusicBrainz

#: MusicBrainz asks clients to stay at or below one request per second.
_MB_LOCK = threading.Lock()
_MB_LAST_REQUEST = 0.0


def _musicbrainz_get(url: str):
    """GET MusicBrainz with its mandatory User-Agent and 1 req/s throttle."""
    global _MB_LAST_REQUEST
    with _MB_LOCK:
        wait = 1.0 - (time.monotonic() - _MB_LAST_REQUEST)
        if wait > 0:
            time.sleep(wait)
        try:
            return _get_json(url, timeout=10.0, headers={"Accept": "application/json"})
        finally:
            _MB_LAST_REQUEST = time.monotonic()


def _mb_artist_credit(credits: Any) -> str:
    if not isinstance(credits, list):
        return ""
    parts: List[str] = []
    for credit in credits:
        if not isinstance(credit, dict):
            continue
        name = credit.get("name") or (credit.get("artist") or {}).get("name") or ""
        parts.append(str(name))
        parts.append(str(credit.get("joinphrase") or ""))
    return "".join(parts).strip()


def _mb_release(releases: Any) -> Dict[str, Any]:
    if not isinstance(releases, list):
        return {}
    valid = [r for r in releases if isinstance(r, dict) and r.get("id")]
    if not valid:
        return {}
    # Prefer the earliest official release so the album/year reflect the
    # original release instead of a later compilation or reissue.
    dated = [r for r in valid if r.get("status") == "Official" and r.get("date")]
    pool = dated or valid
    return min(pool, key=lambda r: str(r.get("date") or "9999"))


def _search_musicbrainz(query: str, limit: int) -> List[Dict[str, Any]]:
    url = "https://musicbrainz.org/ws/2/recording?" + urllib.parse.urlencode(
        {"query": query, "fmt": "json", "limit": max(1, min(limit, 50))}
    )
    data = _musicbrainz_get(url)
    out: List[Dict[str, Any]] = []
    for rec in data.get("recordings", []) or []:
        if not isinstance(rec, dict) or rec.get("video"):
            continue
        recording_id = rec.get("id") or ""
        if not recording_id:
            continue
        release = _mb_release(rec.get("releases"))
        isrcs = rec.get("isrcs") or []
        out.append(
            _item(
                source="musicbrainz",
                item_id=f"musicbrainz:{recording_id}",
                title=rec.get("title", ""),
                artist=_mb_artist_credit(rec.get("artist-credit")),
                album=release.get("title", ""),
                year=str(release.get("date") or "")[:4],
                duration_secs=int((rec.get("length") or 0) // 1000),
                url=f"https://musicbrainz.org/recording/{recording_id}",
                isrc=isrcs[0] if isrcs else "",
            )
        )
    return out


# ------------------------------------------------------------- YouTube

def _search_youtube(query: str, limit: int) -> List[Dict[str, Any]]:
    from .core import search_youtube

    out: List[Dict[str, Any]] = []
    for r in search_youtube(query, limit=limit):
        out.append(
            _item(
                source="youtube",
                item_id=r.get("id", ""),
                title=r.get("title", ""),
                artist=r.get("artist", ""),
                duration_secs=int(r.get("duration_secs") or 0),
                url=r.get("url", ""),
                uploader=r.get("uploader", ""),
            )
        )
    return out


# ---------------------------------------------------------- fan-out/rank

_PROVIDERS = {
    "spotify": _search_spotify,
    "musicbrainz": _search_musicbrainz,
    "deezer": _search_deezer,
    "itunes": _search_itunes,
    "youtube": _search_youtube,
}


def _score(query: str, item: Dict[str, Any]) -> float:
    score = score_candidate("", query, item.get("artist", ""), item.get("title", ""))
    return score + _PROVIDER_BONUS.get(item.get("source", ""), 0.0)


def _dedupe(items: List[Dict[str, Any]]) -> List[Dict[str, Any]]:
    seen = set()
    out = []
    for item in items:
        key = (
            (item.get("artist") or "").strip().lower(),
            (item.get("title") or "").strip().lower(),
        )
        if key in seen:
            continue
        seen.add(key)
        out.append(item)
    return out


def _query_official(query: str, limit: int, sources: tuple) -> List[Dict[str, Any]]:
    active = [s for s in sources if s in _PROVIDERS and (s != "spotify" or spotify_available())]
    if not active:
        return []
    # Fetch a little extra per provider so ranking has room to breathe.
    per_source = max(limit, 5)
    raw_by_source: Dict[str, List[Dict[str, Any]]] = {}
    if len(active) == 1:
        raw_by_source[active[0]] = _PROVIDERS[active[0]](query, per_source)
    else:
        with ThreadPoolExecutor(max_workers=len(active)) as pool:
            futures = {pool.submit(_PROVIDERS[s], query, per_source): s for s in active}
            for fut, src in futures.items():
                try:
                    raw_by_source[src] = fut.result()
                except Exception:
                    raw_by_source[src] = []

    merged: List[Dict[str, Any]] = []
    # Preserve provider priority order, then let the score refine within it.
    for src in active:
        merged.extend(raw_by_source.get(src, []))
    merged = _dedupe(merged)

    scored: List[Dict[str, Any]] = []
    for item in merged:
        score = _score(query, item)
        if score > 0:
            item = dict(item)
            item["score"] = round(score, 1)
            scored.append(item)
    scored.sort(key=lambda i: i.get("score", 0), reverse=True)
    return scored[:limit]


def search_music(query: str, limit: int = 15, source: str = "auto") -> List[Dict[str, Any]]:
    """Search music with the requested provider policy.

    Args:
        query: free-text user query.
        limit: max results.
        source: ``auto`` (default), ``spotify``, ``musicbrainz``, ``deezer``,
            ``itunes``, ``youtube`` or ``all``.

    Raises:
        RuntimeError: explicit source requested but unusable (e.g. missing
            Spotify credentials).
    """
    query = (query or "").strip()
    if not query:
        return []
    source = (source or "auto").lower()
    if source not in SOURCES:
        source = "auto"

    if source == "youtube":
        return _search_youtube(query, limit)[:limit]

    if source == "all":
        official = _query_official(query, limit, OFFICIAL_SOURCES)
        youtube = _search_youtube(query, limit)
        combined = _dedupe(official + youtube)
        combined.sort(key=lambda i: _score(query, i), reverse=True)
        return combined[:limit]

    if source == "auto":
        official = _query_official(query, limit, OFFICIAL_SOURCES)
        if official:
            return official
        # YouTube is only a fallback when official sources have nothing.
        return _search_youtube(query, limit)[:limit]

    # Explicit official provider.
    if source == "spotify" and not spotify_available():
        raise RuntimeError(
            "Spotify search is not configured. Set SPOTIFY_CLIENT_ID / SPOTIFY_CLIENT_SECRET "
            "or create ~/.config/lyra/spotify.json. Deezer or Auto work without keys."
        )
    items = _PROVIDERS[source](query, limit)
    scored = []
    for item in items:
        score = _score(query, item)
        if score > 0:
            item = dict(item)
            item["score"] = round(score, 1)
            scored.append(item)
    scored.sort(key=lambda i: i.get("score", 0), reverse=True)
    return scored[:limit]


# --------------------------------------------------------------- resolve

def _pick_youtube_match(
    artist: str,
    title: str,
    duration: Optional[int],
) -> Optional[Dict[str, Any]]:
    from .core import search_youtube

    query = f"{artist} {title}".strip()
    try:
        candidates = search_youtube(query, limit=8)
    except Exception:
        return None
    if not candidates:
        return None

    best = None
    best_score = -1.0
    for cand in candidates:
        score = score_candidate(
            artist, title, cand.get("artist", ""), cand.get("title", "")
        )
        cand_dur = int(cand.get("duration_secs") or 0)
        if duration and cand_dur:
            delta = abs(cand_dur - int(duration))
            if delta <= 3:
                score += 15.0
            elif delta <= 8:
                score += 6.0
            elif delta > 30:
                score -= 25.0
            elif delta > 12:
                score -= 10.0
        if score > best_score:
            best_score = score
            best = cand
    if best and best_score <= 0:
        best = candidates[0]
    return best or candidates[0]


def resolve(
    artist: str,
    title: str,
    album: str = "",
    duration: Optional[int] = None,
    source: str = "",
    cover_url: str = "",
    year: str = "",
) -> Dict[str, Any]:
    """Map an official metadata result to a downloadable YouTube video.

    Returns the same ``info`` payload the TUI expects, enriched with
    LRCLIB's canonical album + lyric availability. Raises ``RuntimeError``
    when no playable match exists.
    """
    artist = (artist or "").strip()
    title = (title or "").strip()
    if not title:
        raise RuntimeError("Missing track title for resolve")

    match = _pick_youtube_match(artist, title, duration)
    if not match:
        raise RuntimeError(
            f"No downloadable audio found for '{artist} - {title}'. "
            "Try searching YouTube directly."

        )

    rec = lrclib.best_match(artist, title, album, duration)
    final_album = album or (rec.get("album", "") if rec else "")
    final_title = title or (rec.get("title", "") if rec else "")
    final_artist = artist or (rec.get("artist", "") if rec else "")

    return {
        "id": match.get("id", ""),
        "url": match.get("url", ""),
        "raw_title": match.get("raw_title", ""),
        "title": final_title,
        "artist": final_artist,
        "album": final_album,
        "year": year,
        "uploader": match.get("uploader", ""),
        "duration": match.get("duration", _fmt_duration(duration)),
        "duration_secs": int(match.get("duration_secs") or duration or 0),
        "cover_url": cover_url,
        "source": source or "auto",
        "has_lyrics": bool(rec and rec.get("has_synced")),
        "lyrics_langs": [],
    }
