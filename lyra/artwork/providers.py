"""Online cover-art providers: iTunes, Deezer, MusicBrainz/CAA.

All providers return plain ``Candidate`` dicts and never raise.
Network budget: each provider gets its own short timeout; callers run
them in parallel via ``search_candidates``.
"""

import json
import urllib.parse
import urllib.request
from concurrent.futures import ThreadPoolExecutor
from typing import Any, Dict, List

from . import cache as cache_mod
from .scoring import score_candidate

USER_AGENT = "Lyra/1.0 (cover-art search; contact: lyra-player)"
TIMEOUT_S = 4.0

import os
import re

# Small static prior so a slightly worse iTunes hit still beats a random
# Deezer/CAA hit when scores tie. Kept tiny on purpose.
_PROVIDER_BONUS = {"itunes": 2.0, "deezer": 1.0, "caa": 0.0, "web": 0.5}


def _get_json(url: str, timeout: float = TIMEOUT_S) -> Any:
    req = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
    with urllib.request.urlopen(req, timeout=timeout) as resp:
        ctype = resp.headers.get("Content-Type", "")
        if "json" not in ctype and "javascript" not in ctype and "text" not in ctype:
            # Deezer/MB always answer JSON; be lenient anyway and try parsing.
            pass
        return json.loads(resp.read().decode("utf-8", errors="ignore"))


# ---------------------------------------------------------------- iTunes

def _itunes_raw(term: str, country: str = "", limit: int = 10) -> List[Dict[str, Any]]:
    if not term:
        return []
    params = {
        "term": term,
        "entity": "song",
        "limit": str(max(limit, 5)),
    }
    if country:
        params["country"] = country
    url = "https://itunes.apple.com/search?" + urllib.parse.urlencode(params)
    try:
        data = _get_json(url)
        results = data.get("results", [])
        return results[:limit] if isinstance(results, list) else []
    except Exception:
        return []


def _itunes_candidates(artist: str, title: str) -> List[Dict[str, Any]]:
    out: List[Dict[str, Any]] = []
    seen = set()
    # One broad query (global store) + one title-only fallback. No more
    # sequential VN->US->global chains: the global catalog covers VN.
    queries = []
    if artist and title:
        queries.append(f"{artist} {title}")
    if title:
        queries.append(title)
    for term in queries:
        for r in _itunes_raw(term, country="", limit=10):
            tid = r.get("trackId") or r.get("collectionId")
            if tid is not None:
                if tid in seen:
                    continue
                seen.add(tid)
            raw_art = r.get("artworkUrl100", "")
            if not raw_art:
                continue
            out.append({
                "source": "itunes",
                "artist": r.get("artistName", ""),
                "title": r.get("trackName", ""),
                "album": r.get("collectionName", ""),
                "year": (r.get("releaseDate", "") or "")[:4],
                "genre": r.get("primaryGenreName", ""),
                "cover_url": raw_art.replace("100x100bb", "1000x1000bb"),
                "cover_url_fallback": raw_art.replace("100x100bb", "600x600bb"),
            })
        if out:
            break  # broad query hit: skip the narrower fallback
    return out


# ---------------------------------------------------------------- Deezer

def _deezer_raw(query: str, limit: int = 10) -> List[Dict[str, Any]]:
    if not query:
        return []
    url = "https://api.deezer.com/search/track?" + urllib.parse.urlencode(
        {"q": query, "limit": str(limit)}
    )
    try:
        data = _get_json(url)
        items = data.get("data", [])
        return items[:limit] if isinstance(items, list) else []
    except Exception:
        return []


def _deezer_candidates(artist: str, title: str) -> List[Dict[str, Any]]:
    out: List[Dict[str, Any]] = []
    seen = set()
    queries = []
    if artist and title:
        queries.append(f'artist:"{artist}" track:"{title}"')
        queries.append(f"{artist} {title}")
    if title:
        queries.append(title)
    for q in queries:
        for t in _deezer_raw(q, limit=10):
            did = t.get("id")
            if did is not None:
                if did in seen:
                    continue
                seen.add(did)
            album = t.get("album") or {}
            cover_xl = album.get("cover_xl") or album.get("cover_big") or ""
            if not cover_xl:
                continue
            out.append({
                "source": "deezer",
                "artist": (t.get("artist") or {}).get("name", ""),
                "title": t.get("title", ""),
                "album": album.get("title", ""),
                "year": "",
                "genre": "",
                "cover_url": cover_xl,
                "cover_url_fallback": album.get("cover_big", ""),
            })
        if out:
            break
    return out


# ------------------------------------------------------- MusicBrainz + CAA

def _mb_release_ids(artist: str, title: str, limit: int = 4) -> List[tuple]:
    """Best-effort (release MBID, MB score) pairs for the recording.

    MusicBrainz returns its own match ``score`` (0-100); we blend it into
    the final candidate score so a fuzzy MB match can't hide behind the
    echoed query text (respects 1 req/s).
    """
    if not title:
        return []
    q = f'recording:"{title}"'
    if artist:
        q += f' AND artist:"{artist}"'
    url = "https://musicbrainz.org/ws/2/recording/?" + urllib.parse.urlencode(
        {"query": q, "fmt": "json", "limit": str(limit)}
    )
    try:
        data = _get_json(url, timeout=6.0)
        out: List[tuple] = []
        seen = set()
        for rec in data.get("recordings", [])[:limit]:
            try:
                mb_score = float(rec.get("score", 0))
            except (TypeError, ValueError):
                mb_score = 0.0
            for rel in rec.get("releases", [])[:2]:
                mbid = rel.get("id")
                if mbid and mbid not in seen:
                    seen.add(mbid)
                    out.append((mbid, mb_score))
            if len(out) >= limit:
                break
        return out[:limit]
    except Exception:
        return []


def _caa_front_url(release_mbid: str) -> str:
    url = f"https://coverartarchive.org/release/{release_mbid}"
    try:
        data = _get_json(url, timeout=6.0)
        for img in data.get("images", []):
            if img.get("front"):
                thumbs = img.get("thumbnails") or {}
                # Prefer the ~1200px thumbnail; "500" is the floor (upscaling
                # it to 1000x1000 looks soft next to iTunes 1000px art).
                return (
                    thumbs.get("1200")
                    or thumbs.get("large")
                    or thumbs.get("500")
                    or img.get("image", "")
                )
        return ""
    except Exception:
        return ""


def _caa_candidates(artist: str, title: str) -> List[Dict[str, Any]]:
    out: List[Dict[str, Any]] = []
    for mbid, mb_score in _mb_release_ids(artist, title):
        front = _caa_front_url(mbid)
        if front:
            out.append({
                "source": "caa",
                "artist": artist,
                "title": title,
                "album": "",
                "year": "",
                "genre": "",
                "cover_url": front,
                "cover_url_fallback": "",
                "mbid": mbid,
                # Blended into the final score in search_candidates().
                "mb_score": mb_score,
            })
            break  # one good community cover is enough
    return out


# ---------------------------------------------------------------- Web Search

def _web_candidates(artist: str, title: str) -> List[Dict[str, Any]]:
    out: List[Dict[str, Any]] = []
    seen = set()
    queries = []
    if artist and title:
        queries.append(f"{artist} {title} album cover")
    if title:
        queries.append(f"{title} album cover")
    for q in queries:
        url = f"https://www.bing.com/images/search?q={urllib.parse.quote(q)}&form=HDRSC2&first=1"
        req = urllib.request.Request(
            url,
            headers={
                "User-Agent": (
                    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) "
                    "AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
                )
            },
        )
        try:
            with urllib.request.urlopen(req, timeout=5.0) as resp:
                html = resp.read().decode("utf-8", errors="ignore")
            matches = re.findall(r'm="({.*?})"', html)
            for m in matches[:10]:
                try:
                    data = json.loads(m.replace("&quot;", '"'))
                    murl = data.get("murl")
                    if not murl or murl in seen:
                        continue
                    seen.add(murl)
                    desc = data.get("t") or data.get("desc") or ""
                    desc = re.sub(r"[\ue000\ue001]", "", desc).strip()
                    out.append({
                        "source": "web",
                        "artist": artist,
                        "title": title,
                        "album": desc[:40] if desc else "Web Image",
                        "year": "",
                        "genre": "",
                        "cover_url": murl,
                        "cover_url_fallback": data.get("turl", ""),
                    })
                except Exception:
                    continue
            if out:
                break
        except Exception:
            continue
    return out


# ---------------------------------------------------------------- fan-out

_PROVIDERS = {
    "itunes": _itunes_candidates,
    "deezer": _deezer_candidates,
    "caa": _caa_candidates,
    "web": _web_candidates,
}


def search_candidates(
    artist: str,
    title: str,
    sources: tuple = ("itunes", "deezer", "caa", "web"),
    limit: int = 16,
    use_cache: bool = True,
) -> List[Dict[str, Any]]:
    """Query providers in parallel, score, and return top candidates.

    Each item: ``source/artist/title/album/year/genre/cover_url/cached_path/score``.
    Sorted by score desc. Empty list = nothing relevant found.
    """
    artist = (artist or "").strip()
    title = (title or "").strip()
    if not title:
        return []

    active = [s for s in sources if s in _PROVIDERS]
    if use_cache:
        cached = cache_mod.load_meta(artist, title, tuple(active))
        if cached:
            for c in cached:
                if not c.get("cached_path") and c.get("cover_url"):
                    c["cached_path"] = cache_mod.image_path_for_url(c["cover_url"])
            return cached[:limit]

    raw: List[Dict[str, Any]] = []
    if active:
        with ThreadPoolExecutor(max_workers=len(active)) as pool:
            futures = {
                pool.submit(_PROVIDERS[s], artist, title): s for s in active
            }
            for fut, src in futures.items():
                try:
                    raw.extend(fut.result())
                except Exception:
                    continue

    scored: List[Dict[str, Any]] = []
    for c in raw:
        if not c.get("cover_url"):
            continue
        text_score = score_candidate(
            artist, title, c.get("artist", ""), c.get("title", ""),
            provider_bonus=_PROVIDER_BONUS.get(c.get("source", ""), 0.0),
        )
        if "mb_score" in c:
            # CAA echoes the query as artist/title, so the text score is
            # meaningless alone: blend 50/50 with MusicBrainz' own match
            # score to demote fuzzy compilation/live matches.
            try:
                mb_score = float(c.pop("mb_score"))
            except (TypeError, ValueError):
                mb_score = 0.0
            text_score = 0.5 * text_score + 0.5 * mb_score
        c["score"] = round(text_score, 1)
        c["cached_path"] = cache_mod.image_path_for_url(c["cover_url"])
        if c["score"] > 0:
            scored.append(c)
    scored.sort(key=lambda c: c["score"], reverse=True)
    top = scored[:limit]
    if use_cache and top:
        cache_mod.save_meta(artist, title, top, tuple(active))
    return top


def prefetch_candidates(candidates: List[Dict[str, Any]], max_workers: int = 4) -> None:
    """Download candidate images in background so local preview files exist immediately."""
    from .images import download_cover_image
    to_fetch = []
    for c in candidates:
        url = c.get("cover_url")
        cpath = c.get("cached_path") or (cache_mod.image_path_for_url(url) if url else None)
        if cpath:
            c["cached_path"] = cpath
            if url and (not os.path.exists(cpath) or os.path.getsize(cpath) == 0):
                to_fetch.append((url, cpath))
    if to_fetch:
        with ThreadPoolExecutor(max_workers=min(max_workers, len(to_fetch))) as pool:
            futures = [pool.submit(download_cover_image, u, p) for u, p in to_fetch]
            for fut in futures:
                try:
                    fut.result()
                except Exception:
                    pass

