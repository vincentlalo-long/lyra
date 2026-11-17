import re
import html
import json
import urllib.parse
import urllib.request
from typing import List, Tuple, Optional, Dict, Any


def _timestamp_to_seconds(ts_str: str) -> float:
    # Convert 'hh:mm:ss.xxx' or 'mm:ss.xxx' to seconds
    parts = ts_str.strip().replace(",", ".").split(":")
    if len(parts) == 3:
        return int(parts[0]) * 3600 + int(parts[1]) * 60 + float(parts[2])
    elif len(parts) == 2:
        return int(parts[0]) * 60 + float(parts[1])
    return 0.0


def _seconds_to_lrc_tag(seconds: float) -> str:
    # Convert seconds into [mm:ss.xx] using integer centiseconds to avoid 59.999 -> 60.00 rollover
    total_cs = int(round(max(0.0, seconds) * 100))
    mins = total_cs // 6000
    rem_cs = total_cs % 6000
    secs = rem_cs // 100
    cs = rem_cs % 100
    return f"[{mins:02d}:{secs:02d}.{cs:02d}]"


def vtt_to_lrc(vtt_content: str) -> str:
    # Parse WebVTT content into standard .lrc lines
    lines = vtt_content.splitlines()
    cues: List[Tuple[float, str]] = []

    # Matches standard cue timings: 00:01:23.456 --> 00:01:28.000 or 01:23.456 --> ...
    cue_pattern = re.compile(r"^((?:\d{2}:)?\d{2}:\d{2}[\.,]\d+)\s+-->")

    current_start: Optional[float] = None
    current_texts: List[str] = []

    def commit_cue():
        nonlocal current_start, current_texts
        if current_start is not None and current_texts:
            full_text = " ".join(current_texts).strip()
            if full_text:
                cues.append((current_start, full_text))
        current_start = None
        current_texts = []

    for line in lines:
        line = line.strip()
        if not line:
            # Blank line marks end of cue in WebVTT
            commit_cue()
            continue

        if line.startswith("WEBVTT") or line.startswith("NOTE") or line.startswith("Kind:") or line.startswith("Language:"):
            continue

        match = cue_pattern.match(line)
        if match:
            commit_cue()
            current_start = _timestamp_to_seconds(match.group(1))
        elif current_start is not None:
            # Strip tags like <c.color> or <00:00:01.000>
            clean_text = re.sub(r"<[^>]+>", "", line).strip()
            # Remove leading dialogue dashes
            clean_text = re.sub(r"^[-–—]\s*", "", clean_text)
            # Unescape HTML entities (&amp;, &#39;, etc.)
            clean_text = html.unescape(clean_text).strip()
            if clean_text:
                current_texts.append(clean_text)

    commit_cue()

    # Deduplicate consecutive lines
    deduped_cues: List[Tuple[float, str]] = []
    last_text = ""
    for ts, text in cues:
        if text.lower() != last_text.lower():
            deduped_cues.append((ts, text))
            last_text = text

    lrc_lines = []
    for ts, text in deduped_cues:
        tag = _seconds_to_lrc_tag(ts)
        lrc_lines.append(f"{tag} {text}")

    return "\n".join(lrc_lines)


def _fetch_vtt_text(url: str, depth: int = 0) -> Optional[str]:
    if depth > 2 or not url:
        return None
    try:
        req = urllib.request.Request(
            url,
            headers={"User-Agent": "Mozilla/5.0 (X11; Linux x86_64)"}
        )
        with urllib.request.urlopen(req, timeout=10) as resp:
            content = resp.read().decode("utf-8", errors="ignore")
            # If the response is an HLS m3u8 playlist, extract and follow the timedtext URL
            if content.startswith("#EXTM3U"):
                for line in content.splitlines():
                    line = line.strip()
                    if not line or line.startswith("#"):
                        continue
                    if line.startswith("http://") or line.startswith("https://"):
                        return _fetch_vtt_text(line, depth + 1)
                    else:
                        from urllib.parse import urljoin
                        return _fetch_vtt_text(urljoin(url, line), depth + 1)
            return content
    except Exception:
        return None


def fetch_all_vtt(
    info: dict,
    target_langs: Optional[List[str]] = None,
    allow_auto: bool = True,
    download_all: bool = True,
) -> Dict[str, str]:
    """
    Fetches WebVTT subtitles from YouTube info dict.
    Returns a dictionary mapping language_code -> vtt_content.
    If multiple manual subtitles exist and download_all=True, downloads all of them.
    If allow_auto=False, skips YouTube automatic captions completely.
    """
    if target_langs is None:
        target_langs = ["vi", "en"]

    subs = {k: v for k, v in (info.get("subtitles") or {}).items() if k.lower() != "live_chat"}
    auto = info.get("automatic_captions") or {}

    results: Dict[str, str] = {}

    def _extract_vtt_for_formats(formats) -> Optional[str]:
        if not formats:
            return None
        vtt_fmt = next((f for f in formats if f.get("ext") == "vtt"), None)
        if not vtt_fmt:
            vtt_fmt = next((f for f in formats if f.get("url") and "vtt" in f.get("url", "")), None)
        if not vtt_fmt and formats:
            vtt_fmt = formats[0]
        if vtt_fmt and vtt_fmt.get("url"):
            url = vtt_fmt["url"]
            # Ensure YouTube timedtext endpoint returns WebVTT format
            if "timedtext" in url:
                if "fmt=" not in url:
                    url += "&fmt=vtt"
                elif "fmt=vtt" not in url:
                    url = re.sub(r"fmt=[^&]+", "fmt=vtt", url)
            return _fetch_vtt_text(url)
        return None

    # 1. Check manual creator-uploaded subtitles first (highest quality)
    if subs:
        if download_all:
            # Order languages: priority target_langs first, followed by any remaining
            ordered_keys = []
            for lang in target_langs:
                for k in subs.keys():
                    if (k == lang or k.startswith(f"{lang}-")) and k not in ordered_keys:
                        ordered_keys.append(k)
            for k in subs.keys():
                if k not in ordered_keys:
                    ordered_keys.append(k)

            for k in ordered_keys:
                content = _extract_vtt_for_formats(subs[k])
                if content and ("WEBVTT" in content or "-->" in content):
                    results[k] = content
        else:
            for lang in target_langs:
                for k in subs.keys():
                    if k == lang or k.startswith(f"{lang}-"):
                        content = _extract_vtt_for_formats(subs[k])
                        if content and ("WEBVTT" in content or "-->" in content):
                            results[k] = content
                            break
                if results:
                    break
            if not results and subs:
                k = next(iter(subs.keys()))
                content = _extract_vtt_for_formats(subs[k])
                if content and ("WEBVTT" in content or "-->" in content):
                    results[k] = content

    # 2. Check automatic captions ONLY if no manual subtitles found and allow_auto is True
    if not results and allow_auto and auto:
        for lang in target_langs:
            for k in auto.keys():
                if k == lang or k.startswith(f"{lang}-"):
                    content = _extract_vtt_for_formats(auto[k])
                    if content and ("WEBVTT" in content or "-->" in content):
                        results[k] = content
                        break
            if results:
                break

    return results


def fetch_best_vtt(
    info: dict,
    target_langs: Optional[List[str]] = None,
    allow_auto: bool = True,
) -> Optional[str]:
    """Helper returning a single primary VTT text if available."""
    vtt_map = fetch_all_vtt(info, target_langs=target_langs, allow_auto=allow_auto, download_all=False)
    if vtt_map:
        return next(iter(vtt_map.values()))
    return None


def _lrclib_request(url: str, timeout: float = 10.0) -> Any:
    req = urllib.request.Request(
        url,
        headers={"User-Agent": "LyraMusicPlayer/0.3 (https://github.com/vincentlalo-long/lyra)"},
    )
    with urllib.request.urlopen(req, timeout=timeout) as resp:
        return json.loads(resp.read().decode("utf-8", errors="replace"))


def normalize_lyric_query(artist: str, title: str) -> Tuple[str, str]:
    """Clean and split artist/title when a combined filename stem is passed."""
    from .utils import clean_title_and_artist
    from .artwork.textutils import strip_suffix_tags

    a = (artist or "").strip()
    t = (title or "").strip()

    # If caller passed the same filename stem for both artist and title, or left artist empty
    if a and t and a.lower() == t.lower():
        if " - " in t:
            a, t = clean_title_and_artist(t, "")
        else:
            a = ""
            t = strip_suffix_tags(t)
    elif not a and " - " in t:
        a, t = clean_title_and_artist(t, "")
    elif not t and " - " in a:
        a, t = clean_title_and_artist(a, "")
    else:
        t = strip_suffix_tags(t)
        a = strip_suffix_tags(a)

    return a.strip(), t.strip()


def search_lrclib(
    artist: str,
    title: str,
    album: str = "",
    duration: int = 0,
    limit: int = 10,
) -> List[Dict[str, Any]]:
    """Search LRCLIB for synced (preferred) and plain lyrics.

    Handles exact lookups, structured artist+track search, and free-text fallback
    when artist metadata is missing or noisy.
    """
    from .artwork.scoring import score_candidate
    from .artwork.textutils import norm_key

    clean_artist, clean_title = normalize_lyric_query(artist, title)
    if not clean_artist and not clean_title:
        return []

    raw_hits: List[Dict[str, Any]] = []

    def _collect(data: Any):
        if isinstance(data, dict):
            if data.get("syncedLyrics") or data.get("plainLyrics"):
                raw_hits.append(data)
        elif isinstance(data, list):
            for item in data:
                if isinstance(item, dict) and (item.get("syncedLyrics") or item.get("plainLyrics")):
                    raw_hits.append(item)

    # 1. Structured search by artist_name + track_name when both are known
    if clean_artist and clean_title:
        params: Dict[str, Any] = {
            "artist_name": clean_artist,
            "track_name": clean_title,
        }
        if album and album.strip():
            params["album_name"] = album.strip()
        try:
            url = f"https://lrclib.net/api/search?{urllib.parse.urlencode(params)}"
            _collect(_lrclib_request(url))
        except Exception:
            pass

    # 2. Combined free-text query fallback if structured search found no synced lyrics
    has_synced = any(bool((h.get("syncedLyrics") or "").strip()) for h in raw_hits)
    if not has_synced:
        q = f"{clean_artist} {clean_title}".strip() if clean_artist else clean_title
        if q:
            try:
                url = f"https://lrclib.net/api/search?{urllib.parse.urlencode({'q': q})}"
                _collect(_lrclib_request(url))
            except Exception:
                pass

    # 3. Title-only fallback ONLY when artist is unknown or a generic YouTube channel suffix
    has_synced = any(bool((h.get("syncedLyrics") or "").strip()) for h in raw_hits)
    artist_is_unknown = not clean_artist or clean_artist.lower() in ("unknown", "unknown artist")
    if not has_synced and artist_is_unknown and clean_title:
        try:
            url = f"https://lrclib.net/api/search?{urllib.parse.urlencode({'q': clean_title})}"
            _collect(_lrclib_request(url))
        except Exception:
            pass

    seen = set()
    scored: List[Tuple[Tuple[int, float, float], Dict[str, Any]]] = []

    for item in raw_hits:
        synced_text = (item.get("syncedLyrics") or "").strip()
        plain_text = (item.get("plainLyrics") or "").strip()
        lyrics_text = synced_text or plain_text
        if not lyrics_text:
            continue

        cand_artist = (item.get("artistName") or item.get("artist") or "").strip()
        cand_title = (item.get("trackName") or item.get("title") or "").strip()
        cand_album = (item.get("albumName") or item.get("album") or "").strip()
        try:
            cand_dur = int(round(float(item.get("duration") or 0)))
        except (ValueError, TypeError):
            cand_dur = 0
        is_synced = bool(synced_text)

        dedup_key = (
            norm_key(cand_artist),
            norm_key(cand_title),
            norm_key(cand_album),
            cand_dur,
            is_synced,
        )
        if dedup_key in seen:
            continue
        seen.add(dedup_key)

        query_artist_for_score = "" if artist_is_unknown else clean_artist
        rel = score_candidate(query_artist_for_score, clean_title, cand_artist, cand_title)
        if rel < 55.0:
            # Reject songs by a different artist that merely share a similar title
            continue

        dur_penalty = 0.0
        if duration > 0 and cand_dur > 0:
            dur_diff = abs(cand_dur - duration)
            # If duration differs by more than 25 seconds, heavily penalize so wrong versions don't win
            if dur_diff > 25:
                dur_penalty = 30.0 + (dur_diff - 25) * 0.5
            else:
                dur_penalty = dur_diff / 5.0

        final_score = round(rel - dur_penalty, 2)
        if final_score < 45.0:
            continue

        sort_key = (1 if is_synced else 0, final_score, -float(cand_dur == 0))
        scored.append((
            sort_key,
            {
                "artist": cand_artist or clean_artist,
                "title": cand_title or clean_title,
                "album": cand_album,
                "duration": cand_dur,
                "lyrics": lyrics_text,
                "synced": is_synced,
                "score": final_score,
            },
        ))

    scored.sort(key=lambda pair: pair[0], reverse=True)
    return [entry for _, entry in scored[:max(1, limit)]]


def _iter_config_paths() -> List[str]:
    """Return candidate Lyra config.toml file paths in lookup priority order."""
    import os

    candidates: List[str] = []
    xdg = os.environ.get("XDG_CONFIG_HOME")
    if xdg:
        candidates.append(os.path.join(xdg, "lyra", "config.toml"))
    candidates.append(os.path.join(os.path.expanduser("~"), ".config", "lyra", "config.toml"))
    return [p for p in candidates if os.path.isfile(p)]


def configured_music_roots() -> List[str]:
    """Return existing music/download directories from config, env, or ~/Music."""
    import os

    music_folders: List[str] = []
    download_dirs: List[str] = []
    for path in _iter_config_paths():
        try:
            with open(path, "r", encoding="utf-8", errors="ignore") as f:
                for line in f:
                    m = re.match(r'^\s*(music_folder|download_dir)\s*=\s*["\']([^"\']+)["\']', line)
                    if m:
                        p = os.path.expanduser(m.group(2).strip())
                        if os.path.isdir(p):
                            target = music_folders if m.group(1) == "music_folder" else download_dirs
                            if p not in target:
                                target.append(p)
        except Exception:
            pass

    roots: List[str] = []
    for p in (*music_folders, *download_dirs):
        if p not in roots:
            roots.append(p)

    env_dir = (os.environ.get("LYRA_MUSIC_DIR") or "").strip()
    if env_dir:
        p = os.path.expanduser(env_dir)
        if os.path.isdir(p) and p not in roots:
            roots.append(p)

    default_music = os.path.join(os.path.expanduser("~"), "Music")
    if os.path.isdir(default_music) and default_music not in roots:
        roots.append(default_music)
    return roots


def get_groq_api_key() -> str:
    """Resolve Groq API key from environment or ~/.config/lyra/config.toml."""
    import os

    for env_name in ("GROQ_API_KEY", "LYRA_GROQ_API_KEY"):
        val = (os.environ.get(env_name) or "").strip()
        if val:
            return val

    for path in _iter_config_paths():
        try:
            with open(path, "r", encoding="utf-8", errors="ignore") as f:
                for line in f:
                    m = re.match(r'^\s*groq_api_key\s*=\s*["\']([^"\']+)["\']', line)
                    if m and m.group(1).strip():
                        return m.group(1).strip()
        except Exception:
            pass
    return ""


def find_local_audio_file(artist: str, title: str) -> Optional[str]:
    """Locate a local .mp3 file in configured music/download folders matching artist/title."""
    import os
    from .artwork.textutils import norm_key

    def _tokens(text: str) -> List[str]:
        cleaned = re.sub(r"[^a-z0-9\s]", " ", norm_key(text))
        return [t for t in cleaned.split() if t]

    clean_a, clean_t = normalize_lyric_query(artist, title)
    query_tokens = _tokens(f"{clean_a} {clean_t}".strip() or f"{artist} {title}")
    title_tokens = _tokens(clean_t or title)

    roots = configured_music_roots()

    best_path: Optional[str] = None
    best_score = 0.0
    newest_mp3: Optional[Tuple[float, str]] = None

    for root in roots:
        for dirpath, _, filenames in os.walk(root):
            for fn in filenames:
                if not fn.lower().endswith(".mp3"):
                    continue
                full_p = os.path.join(dirpath, fn)
                try:
                    mtime = os.path.getmtime(full_p)
                    if newest_mp3 is None or mtime > newest_mp3[0]:
                        newest_mp3 = (mtime, full_p)
                except Exception:
                    pass

                stem = os.path.splitext(fn)[0]
                file_tokens = _tokens(stem)
                if not file_tokens:
                    continue

                if query_tokens and file_tokens == query_tokens:
                    return full_p
                if title_tokens and file_tokens == title_tokens:
                    best_path = full_p
                    best_score = 95.0
                    continue

                if query_tokens:
                    q_set = set(query_tokens)
                    f_set = set(file_tokens)
                    overlap = len(q_set & f_set)
                    if overlap > 0:
                        score = (overlap / max(len(q_set), 1)) * 60.0 + (overlap / max(len(f_set), 1)) * 40.0
                        if score > best_score:
                            best_score = score
                            best_path = full_p

    if best_score >= 40.0 and best_path:
        return best_path
    # If user just typed "ai:" with empty query, fall back to most recent MP3
    if not query_tokens and newest_mp3:
        return newest_mp3[1]
    return None


_PROMO_LINE_RE = re.compile(
    r"(?i)(https?://|www\.|@[a-z0-9_]+|#[a-z0-9_]+|"
    r"\b(follow|instagram|facebook|tiktok|soundcloud|spotify|youtube|twitter|"
    r"prod\.?\s*by|produced\s+by|mixed\s+by|mastered\s+by|beat\s+by|"
    r"contact|booking|business|email|subscribe|copyright|all\s+rights\s+reserved)\b)"
)

_HALLUCINATION_RE = re.compile(
    r"(?i)(subtitles\s+by|amara\.org|transcribed\s+by|hãy\s+subscribe|đăng\s+ký\s+kênh|"
    r"subscribe\s+cho\s+k\b|cảm\s+ơn\s+các\s+bạn\s+đã\s+(?:xem|theo\s+dõi)|thank(?:s|\s+you)\s+for\s+watching|"
    r"ghiền\s*mì(?:\s*gõ)?|ghi\s+m\s+g|la\s*la\s*school|lalaschool|không\s+bỏ\s+lỡ\s+những\s+video\s+hấp\s+dẫn|"
    r"please\s+subscribe|like\s+và\s+share|🎵|"
    r"优优独播剧场|YoYo\s+Television\s+Series|z\s*ither\s+har\s*p|請不吝點讚|感謝收看)"
)

_HALLUCINATION_STRIP_RE = re.compile(
    r"(?i)(?:[a-zÀ-ỹ]*\s*subscribe\s+(?:cho\s+kênh\s+|cho\s+k\b\s*)?(?:ghiền\s*mì(?:\s*gõ)?|ghi\s*m\s*g|la\s*la\s*school|lalaschool)?(?:\s*(?:để\s*)?kh(?:ông)?\s*b(?:ỏ)?(?:\s*l(?:ỡ)?(?:\s*nh(?:ững)?\s*video(?:\s*h(?:ấp)?\s*d(?:ẫn)?)?)?)?)?|"
    r"(?:gõ\s+)?để\s+không\s+bỏ\s+lỡ(?:\s+những\s+video\s+hấp\s+dẫn)?|"
    r"(?:lỡ\s+)?những\s+video\s+hấp\s+dẫn|"
    r"đăng\s+ký\s+kênh(?:\s+để\s+[^.!?\n]+)?|"
    r"cảm\s+ơn\s+các\s+bạn\s+đã\s+(?:xem|theo\s+dõi)[^.!?\n]*|"
    r"ghiền\s*mì\s*gõ|la\s*la\s*school|lalaschool|"
    r"subtitles?\s+by\s+[^.!?\n]+|amara\.org|transcribed\s+by\s+[^.!?\n]+|"
    r"thank(?:s|\s+you)\s+for\s+watching[^.!?\n]*|please\s+subscribe[^.!?\n]*|"
    r"优优独播剧场[^\n]*|YoYo\s+Television\s+Series[^\n]*|z\s*ither\s+har\s*p|"
    r"\[\s*(?:âm\s+nhạc|music)\s*\]|🎵)"
)

_STANDALONE_NOISE_RE = re.compile(
    r"(?i)^(?:\s*(?:tập\s*\d+|bye|thank\s+you|âm\s+nhạc|music)[\s.!?,…]*)+$"
)

_VERIFY_STOPWORDS = {
    "the", "and", "you", "for", "that", "this", "with", "are", "was", "have",
    "but", "not", "all", "can", "just", "from", "they", "what", "when", "your",
    "she", "him", "her", "his", "its", "our", "out", "now", "how", "who",
    "cho", "cua", "nhu", "nhung", "cac", "mot", "ngay", "khi", "khong", "thoi",
    "nhau", "minh", "anh", "cung", "nguoi", "duoc", "roi", "het", "lai", "nay",
}

_VALID_SHORT_WORDS = {
    "a", "i", "im", "oh", "ah", "uh", "ya", "yo", "ok", "no", "so", "to", "do", "go",
    "in", "on", "at", "of", "or", "by", "as", "is", "it", "be", "we", "me", "my", "he",
    "up", "if", "us", "em", "ta", "la", "là", "và", "có", "đi", "ra", "về", "với",
    "ai", "gì", "cô", "chú", "mẹ", "cha", "ba", "bà", "ông", "bé", "đó", "đây",
    "kia", "này", "thì", "mà", "đã", "sẽ", "vì", "từ", "lại", "rồi", "hết", "như",
    "khi", "cho", "của", "nếu", "hay", "nhà", "đời", "tình", "người", "nào", "sao",
    "đau", "yêu", "em", "anh", "cứ", "mơ", "xa", "gần", "lâu", "đêm", "ngày", "ơi",
    "hỡi", "nên", "mới", "còn", "hết", "vẫn", "đều", "đến", "bên", "trên", "dưới",
}


def _has_word_repetition_loop(text: str) -> bool:
    """Detect degenerate Whisper decoding loops like 'tóc, tóc, tóc, tóc, tóc'."""
    toks = [t.lower() for t in re.findall(r"[0-9A-Za-zÀ-ỹ]+", text or "")]
    if len(toks) < 5:
        return False
    run = 1
    for i in range(1, len(toks)):
        if toks[i] == toks[i - 1]:
            run += 1
            if run >= 4:
                return True
        else:
            run = 1
    # Low lexical diversity on long strings
    if len(toks) >= 10 and len(set(toks)) <= 2:
        return True
    return False


def _has_corrupt_fragment_run(text: str) -> bool:
    """Detect Whisper BPE-truncated consonant gibberish like 'kh b l nh video h d'."""
    toks = re.findall(r"[0-9A-Za-zÀ-ỹ]+", text or "")
    if len(toks) < 7:
        return False
    broken = sum(1 for t in toks if len(t) <= 2 and t.lower() not in _VALID_SHORT_WORDS)
    return (broken / len(toks)) >= 0.45


def _strip_whisper_noise(text: str) -> str:
    cleaned = _HALLUCINATION_STRIP_RE.sub(" ", text or "")
    cleaned = re.sub(r"\s+", " ", cleaned).strip(" \t\r\n.,;:-")
    if (
        not cleaned
        or _STANDALONE_NOISE_RE.match(cleaned)
        or _HALLUCINATION_RE.search(cleaned)
        or _has_word_repetition_loop(cleaned)
        or _has_corrupt_fragment_run(cleaned)
    ):
        return ""
    return cleaned


def extract_lyrics_from_description(description: str) -> str:
    """Extract clean lyric lines from a SoundCloud or YouTube track description."""
    if not description or not description.strip():
        return ""

    raw_lines = description.splitlines()
    # If description has an explicit "Lyrics:" / "Lời bài hát:" section header, start after it
    header_re = re.compile(r"(?i)^\s*(lyrics|lời\s*bài\s*hát|lyric)\s*[:\-–—]?\s*$")
    start_idx = 0
    for idx, line in enumerate(raw_lines):
        if header_re.match(line.strip()):
            start_idx = idx + 1
            break

    kept: List[str] = []
    for line in raw_lines[start_idx:]:
        s = line.strip()
        if not s:
            continue
        if len(s) > 120 or _PROMO_LINE_RE.search(s):
            continue
        if re.match(r"^[\-=*_~#•·\s]+$", s):
            continue
        kept.append(s)

    if len(kept) < 4:
        return ""
    return "\n".join(kept)


def _segments_to_lrc(segments: List[Dict[str, Any]], words: Optional[List[Dict[str, Any]]] = None) -> str:
    """Convert Whisper verbose_json segments (and optional word timings) into synced .lrc text."""
    cues: List[Tuple[float, str]] = []

    for seg in segments:
        raw_text = (seg.get("text") or "").strip()
        if not raw_text:
            continue
        no_speech = float(seg.get("no_speech_prob") or 0.0)
        avg_logprob = float(seg.get("avg_logprob") or 0.0)
        comp_ratio = float(seg.get("compression_ratio") or 0.0)
        if comp_ratio > 2.4 or avg_logprob < -1.15 or (no_speech > 0.72 and avg_logprob < -0.55):
            continue
        text = _strip_whisper_noise(raw_text)
        if not text:
            continue

        seg_start = max(0.0, float(seg.get("start") or 0.0))
        seg_end = max(seg_start, float(seg.get("end") or seg_start))
        seg_words = seg.get("words")

        # Only split into sub-phrases if a single segment is unusually long (> 16 words)
        if isinstance(seg_words, list) and len(seg_words) > 16:
            chunk_words: List[str] = []
            chunk_start: Optional[float] = None
            prev_end: Optional[float] = None
            for w_obj in seg_words:
                w_txt = (w_obj.get("word") or "").strip()
                if not w_txt:
                    continue
                w_start = max(0.0, float(w_obj.get("start") or seg_start))
                w_end = max(w_start, float(w_obj.get("end") or w_start))
                if chunk_start is None:
                    chunk_start = w_start
                gap = (w_start - prev_end) if prev_end is not None else 0.0
                if chunk_words and (len(chunk_words) >= 12 or (len(chunk_words) >= 6 and gap >= 0.45)):
                    phrase = _strip_whisper_noise(" ".join(chunk_words))
                    if phrase:
                        cues.append((chunk_start, phrase))
                    chunk_words = [w_txt]
                    chunk_start = w_start
                else:
                    chunk_words.append(w_txt)
                prev_end = w_end
            if chunk_words and chunk_start is not None:
                phrase = _strip_whisper_noise(" ".join(chunk_words))
                if phrase:
                    cues.append((chunk_start, phrase))
        else:
            cues.append((seg_start, text))
        _ = seg_end

    if not cues:
        return ""

    # Sort monotonically so word-vs-segment timestamps never go backwards
    cues.sort(key=lambda item: item[0])

    # Deduplicate consecutive identical lines and add instrumental marker on long gaps
    lrc_lines: List[str] = []
    last_text = ""
    last_ts = 0.0
    for ts, text in cues:
        clean = _strip_whisper_noise(text)
        if not clean:
            continue
        if clean.lower() == last_text.lower() and (ts - last_ts) < 6.0:
            continue
        if lrc_lines and (ts - last_ts) >= 10.0:
            lrc_lines.append(f"{_seconds_to_lrc_tag(last_ts + 4.0)} ♪")
        lrc_lines.append(f"{_seconds_to_lrc_tag(ts)} {clean}")
        last_text = clean
        last_ts = ts

    return "\n".join(lrc_lines)


def transcribe_lyrics_groq(
    audio_path: str,
    artist: str = "",
    title: str = "",
    description_hint: str = "",
    lang: Optional[str] = None,
    api_key: Optional[str] = None,
) -> Optional[str]:
    """Generate synced .lrc lyrics from an audio file using Groq Whisper API.

    Compresses audio to a lightweight 16kHz mono stream via ffmpeg before upload
    and injects artist/title/description lyric hints into the Whisper prompt.
    """
    import os
    import shutil
    import subprocess
    import tempfile
    import uuid

    key = (api_key or get_groq_api_key()).strip()
    if not key or not audio_path or not os.path.isfile(audio_path):
        return None

    with tempfile.TemporaryDirectory(prefix="lyra_groq_") as tmp_dir:
        upload_path = audio_path
        if shutil.which("ffmpeg"):
            small_mp3 = os.path.join(tmp_dir, "speech_16k.mp3")
            try:
                subprocess.run(
                    [
                        "ffmpeg",
                        "-y",
                        "-i",
                        audio_path,
                        "-vn",
                        "-af",
                        "highpass=f=180,lowpass=f=3800",
                        "-ac",
                        "1",
                        "-ar",
                        "16000",
                        "-b:a",
                        "24k",
                        small_mp3,
                    ],
                    stdout=subprocess.DEVNULL,
                    stderr=subprocess.DEVNULL,
                    timeout=60,
                    check=False,
                )
                if os.path.isfile(small_mp3) and os.path.getsize(small_mp3) > 1024:
                    upload_path = small_mp3
            except Exception:
                pass

        # Groq Whisper API has a 25MB request limit
        try:
            if os.path.getsize(upload_path) > 24 * 1024 * 1024:
                return None
            with open(upload_path, "rb") as f:
                audio_bytes = f.read()
        except Exception:
            return None

    # Only pass prompt when actual description lyric hints exist (passing bare song titles
    # causes Whisper to echo the title on instrumental 30s windows).
    prompt_str = ""
    if description_hint:
        hint_clean = re.sub(r"\s+", " ", description_hint).strip()
        if hint_clean:
            prompt_str = hint_clean[:600]

    for model_name in ("whisper-large-v3", "whisper-large-v3-turbo"):
        boundary = f"----LyraGroqBoundary{uuid.uuid4().hex}"
        body = bytearray()

        def _add_field(name: str, value: str):
            body.extend(f"--{boundary}\r\n".encode("utf-8"))
            body.extend(f'Content-Disposition: form-data; name="{name}"\r\n\r\n'.encode("utf-8"))
            body.extend(f"{value}\r\n".encode("utf-8"))

        _add_field("model", model_name)
        _add_field("response_format", "verbose_json")
        _add_field("temperature", "0")
        _add_field("timestamp_granularities[]", "segment")
        _add_field("timestamp_granularities[]", "word")
        if prompt_str:
            _add_field("prompt", prompt_str)
        if lang and len(lang.strip()) == 2:
            _add_field("language", lang.strip().lower())

        body.extend(f"--{boundary}\r\n".encode("utf-8"))
        body.extend(
            b'Content-Disposition: form-data; name="file"; filename="track.mp3"\r\n'
            b"Content-Type: audio/mpeg\r\n\r\n"
        )
        body.extend(audio_bytes)
        body.extend(f"\r\n--{boundary}--\r\n".encode("utf-8"))

        req = urllib.request.Request(
            "https://api.groq.com/openai/v1/audio/transcriptions",
            data=bytes(body),
            headers={
                "Authorization": f"Bearer {key}",
                "Content-Type": f"multipart/form-data; boundary={boundary}",
                "User-Agent": "LyraMusicPlayer/0.3",
            },
            method="POST",
        )
        try:
            with urllib.request.urlopen(req, timeout=90) as resp:
                payload = json.loads(resp.read().decode("utf-8", errors="replace"))
            segments = payload.get("segments") or []
            words = payload.get("words") or []
            # Attach top-level words to segments if not nested inside segments
            if words and segments and not any(s.get("words") for s in segments):
                for seg in segments:
                    s_start = float(seg.get("start") or 0.0)
                    s_end = float(seg.get("end") or s_start)
                    seg["words"] = [
                        w for w in words
                        if s_start <= (float(w.get("start") or 0.0) + float(w.get("end") or 0.0)) / 2.0 < s_end
                    ]
            lrc = _segments_to_lrc(segments, words)
            if lrc.strip():
                return lrc
        except Exception:
            continue

    return None


def _probe_audio_duration(audio_path: str) -> float:
    """Return audio duration in seconds via ffprobe, or 0.0 on failure."""
    import shutil
    import subprocess

    if not shutil.which("ffprobe"):
        return 0.0
    try:
        out = subprocess.check_output(
            [
                "ffprobe",
                "-v",
                "error",
                "-show_entries",
                "format=duration",
                "-of",
                "default=noprint_wrappers=1:nokey=1",
                audio_path,
            ],
            stderr=subprocess.DEVNULL,
            timeout=10,
        )
        return max(0.0, float(out.decode().strip() or 0.0))
    except Exception:
        return 0.0


def find_existing_lrc_for_audio(audio_path: str) -> Optional[str]:
    """Return the first existing sidecar .lrc path matching `audio_path`."""
    import glob
    import os

    if not audio_path:
        return None
    base = os.path.splitext(audio_path)[0]
    direct = f"{base}.lrc"
    if os.path.isfile(direct):
        return direct
    for lang in ("vi", "en", "ja", "ko", "zh", "lrclib"):
        cand = f"{base}.{lang}.lrc"
        if os.path.isfile(cand):
            return cand
    for cand in sorted(glob.glob(f"{glob.escape(base)}.*.lrc")):
        if os.path.isfile(cand):
            return cand
    return None


def verify_lrc_with_audio(
    audio_path: str,
    lrc_text: str,
    api_key: Optional[str] = None,
) -> Tuple[bool, float, str]:
    """Cross-check whether `lrc_text` actually matches the vocals in `audio_path`.

    Selects the densest active vocal 35s window in `lrc_text` (or ~30% into the track
    for plain lyrics), transcribes that audio slice with Groq Whisper, and scores word overlap.
    Returns `(is_match, confidence_score, heard_sample)`.
    """
    import os
    import shutil
    import subprocess
    import tempfile
    import uuid
    from difflib import SequenceMatcher
    from .artwork.textutils import ascii_fold

    key = (api_key or get_groq_api_key()).strip()

    timed_lines: List[Tuple[float, str]] = []
    single_tag_times: List[float] = []
    hallucinated_or_corrupt = 0
    total_lyric_lines = 0
    tag_re = re.compile(r"\[(\d{2}:\d{2}(?:[\.,]\d+)?)\]")
    meta_re = re.compile(r"^\[[a-zA-Z]+:.*\]$")

    for raw in (lrc_text or "").splitlines():
        raw = raw.strip()
        if not raw:
            continue
        tags = tag_re.findall(raw)
        body = tag_re.sub("", raw).strip()
        if not body or body == "♪":
            continue
        if meta_re.match(raw) and not tags:
            continue
        total_lyric_lines += 1
        if _HALLUCINATION_RE.search(body) or _has_word_repetition_loop(body) or _has_corrupt_fragment_run(body):
            hallucinated_or_corrupt += 1
        if tags:
            ts = _timestamp_to_seconds(tags[0])
            timed_lines.append((ts, body))
            if len(tags) == 1:
                single_tag_times.append(ts)

    # Pre-audio structural integrity check (catches corrupted/hallucinated/out-of-order .lrc immediately)
    if total_lyric_lines == 0:
        return False, 0.0, "Empty lyrics"
    if hallucinated_or_corrupt >= 1 and (hallucinated_or_corrupt * 1.0 / total_lyric_lines) >= 0.05:
        return False, 0.0, "Contains hallucinated or corrupted text"
    if len(single_tag_times) >= 4:
        reversed_jumps = sum(
            1 for i in range(1, len(single_tag_times))
            if single_tag_times[i] + 2.0 < single_tag_times[i - 1]
        )
        if reversed_jumps >= 1:
            return False, 0.0, "Corrupted out-of-order timestamps"

    if not key or not audio_path or not os.path.isfile(audio_path) or not shutil.which("ffmpeg"):
        return True, 1.0, ""

    def _word_list(s: str, drop_stopwords: bool = False) -> List[str]:
        folded = ascii_fold(s)
        cleaned = re.sub(r"[^a-z0-9\s]", " ", folded)
        words = [w for w in cleaned.split() if len(w) >= 2]
        if drop_stopwords:
            content = [w for w in words if len(w) >= 3 and w not in _VERIFY_STOPWORDS]
            if len(content) >= 4:
                return content
        return words

    candidate_windows: List[Tuple[float, List[str]]] = []
    if len(timed_lines) >= 2:
        # Score candidate 35s vocal windows across first half & second half of the track
        first_ts = timed_lines[0][0]
        last_ts = timed_lines[-1][0]
        span = last_ts - first_ts
        mid_ts = first_ts + span * 0.5
        half1_wins: List[Tuple[float, float, List[str]]] = []
        half2_wins: List[Tuple[float, float, List[str]]] = []

        for idx, (ts, _) in enumerate(timed_lines):
            w_start = max(0.0, ts - 1.0)
            w_end = w_start + 35.0
            texts = [
                b for t2, b in timed_lines[idx:]
                if t2 <= w_end
            ]
            word_cnt = sum(len(t.split()) for t in texts)
            if word_cnt < 8:
                continue
            intro_bonus = 20.0 if (span > 60.0 and w_start >= 12.0) else 0.0
            entry = (word_cnt + intro_bonus, w_start, texts)
            if span > 90.0 and ts >= mid_ts:
                half2_wins.append(entry)
            else:
                half1_wins.append(entry)

        half1_wins.sort(key=lambda x: x[0], reverse=True)
        half2_wins.sort(key=lambda x: x[0], reverse=True)
        if half1_wins:
            candidate_windows.append((half1_wins[0][1], half1_wins[0][2]))
        if half2_wins:
            candidate_windows.append((half2_wins[0][1], half2_wins[0][2]))
        elif len(half1_wins) > 1:
            candidate_windows.append((half1_wins[1][1], half1_wins[1][2]))
        if not candidate_windows:
            ws = max(0.0, timed_lines[0][0] - 1.0)
            candidate_windows.append((ws, [b for _, b in timed_lines[:8]]))
    else:
        plain_lines = [
            ln.strip() for ln in (lrc_text or "").splitlines()
            if ln.strip() and not tag_re.match(ln.strip()) and not meta_re.match(ln.strip()) and not _HALLUCINATION_RE.search(ln)
        ]
        if len(plain_lines) < 3:
            return False, 0.0, ""
        dur = _probe_audio_duration(audio_path)
        w1 = max(15.0, min(dur * 0.25, dur - 40.0)) if dur > 60.0 else 15.0
        candidate_windows.append((w1, plain_lines))
        if dur > 120.0:
            candidate_windows.append((max(30.0, dur * 0.55), plain_lines))

    def _transcribe_slice(start_sec: float) -> Optional[str]:
        with tempfile.TemporaryDirectory(prefix="lyra_verify_") as tmp_dir:
            snippet_mp3 = os.path.join(tmp_dir, "verify_35s.mp3")
            try:
                subprocess.run(
                    [
                        "ffmpeg",
                        "-y",
                        "-ss",
                        f"{start_sec:.2f}",
                        "-t",
                        "35",
                        "-i",
                        audio_path,
                        "-vn",
                        "-af",
                        "highpass=f=180,lowpass=f=3800",
                        "-ac",
                        "1",
                        "-ar",
                        "16000",
                        "-b:a",
                        "24k",
                        snippet_mp3,
                    ],
                    stdout=subprocess.DEVNULL,
                    stderr=subprocess.DEVNULL,
                    timeout=30,
                    check=False,
                )
                if not os.path.isfile(snippet_mp3) or os.path.getsize(snippet_mp3) < 512:
                    return None
                with open(snippet_mp3, "rb") as f:
                    audio_bytes = f.read()
            except Exception:
                return None

        boundary = f"----LyraVerifyBoundary{uuid.uuid4().hex}"
        body = bytearray()

        def _add_field(name: str, value: str):
            body.extend(f"--{boundary}\r\n".encode("utf-8"))
            body.extend(f'Content-Disposition: form-data; name="{name}"\r\n\r\n'.encode("utf-8"))
            body.extend(f"{value}\r\n".encode("utf-8"))

        _add_field("model", "whisper-large-v3-turbo")
        _add_field("response_format", "json")
        _add_field("temperature", "0")
        body.extend(f"--{boundary}\r\n".encode("utf-8"))
        body.extend(
            b'Content-Disposition: form-data; name="file"; filename="snippet.mp3"\r\n'
            b"Content-Type: audio/mpeg\r\n\r\n"
        )
        body.extend(audio_bytes)
        body.extend(f"\r\n--{boundary}--\r\n".encode("utf-8"))

        req = urllib.request.Request(
            "https://api.groq.com/openai/v1/audio/transcriptions",
            data=bytes(body),
            headers={
                "Authorization": f"Bearer {key}",
                "Content-Type": f"multipart/form-data; boundary={boundary}",
                "User-Agent": "LyraMusicPlayer/0.3",
            },
            method="POST",
        )
        try:
            with urllib.request.urlopen(req, timeout=25) as resp:
                payload = json.loads(resp.read().decode("utf-8", errors="replace"))
            return (payload.get("text") or "").strip()
        except Exception:
            return None

    is_plain_mode = len(timed_lines) < 2
    scores: List[float] = []
    best_heard = ""
    api_failed = True

    for win_start, win_texts in candidate_windows:
        lrc_words = _word_list(" ".join(win_texts), drop_stopwords=is_plain_mode)
        if len(lrc_words) < 4:
            continue

        heard_raw = _transcribe_slice(win_start)
        if heard_raw is None:
            continue
        api_failed = False

        heard_text = _strip_whisper_noise(heard_raw)
        if not heard_text:
            continue

        heard_words = _word_list(heard_text, drop_stopwords=is_plain_mode)
        if len(heard_words) < 3:
            if not best_heard:
                best_heard = heard_text
            continue

        lrc_set = set(lrc_words)
        heard_set = set(heard_words)
        matched = 0
        for hw in heard_set:
            if hw in lrc_set or any(SequenceMatcher(None, hw, lw).ratio() >= 0.82 for lw in lrc_set):
                matched += 1

        denom = max(min(len(heard_set), len(lrc_set)), 1)
        overlap_ratio = matched / denom
        seq_ratio = SequenceMatcher(None, " ".join(heard_words), " ".join(lrc_words)).ratio()
        confidence = round(max(overlap_ratio, seq_ratio), 2)
        scores.append(confidence)
        if confidence >= max(scores) or not best_heard:
            best_heard = heard_text

        # Early pass only when first window strongly matches (>= 65%)
        if confidence >= 0.65:
            return True, confidence, heard_text

    if api_failed:
        return True, 1.0, ""
    if not scores:
        # Whisper heard no clear singing across checked windows
        return (not is_plain_mode), (0.0 if is_plain_mode else 1.0), ""
    if not is_plain_mode and len(scores) >= 2 and min(scores) < 0.18:
        # Reject shifted or half-mismatched timed .lrc (e.g., wrong remix body despite shared intro)
        return False, min(scores), best_heard
    best_conf = max(scores)
    return (best_conf >= 0.28), best_conf, best_heard


def verify_existing_lrc(
    audio_path: str,
    lrc_path: Optional[str] = None,
    api_key: Optional[str] = None,
) -> Dict[str, Any]:
    """Verify the sidecar .lrc file of `audio_path` against actual vocals.

    Returns dict with keys:
      status: "ok" | "mismatch" | "plain" | "missing"
      confidence: float (0.0..1.0)
      has_timing: bool
      heard: str
      lrc_path: str
    """
    import os

    resolved_lrc = lrc_path if (lrc_path and os.path.isfile(lrc_path)) else find_existing_lrc_for_audio(audio_path)
    if not resolved_lrc or not os.path.isfile(resolved_lrc):
        return {
            "type": "lyric_verify",
            "status": "missing",
            "confidence": 0.0,
            "has_timing": False,
            "heard": "",
            "lrc_path": "",
        }

    try:
        with open(resolved_lrc, "r", encoding="utf-8", errors="ignore") as f:
            text = f.read()
    except Exception:
        return {
            "type": "lyric_verify",
            "status": "missing",
            "confidence": 0.0,
            "has_timing": False,
            "heard": "",
            "lrc_path": resolved_lrc,
        }

    has_timing = bool(re.search(r"\[\d{2}:\d{2}(?:[\.,]\d+)?\]", text))
    is_ok, conf, heard = verify_lrc_with_audio(audio_path, text, api_key=api_key)
    if not is_ok:
        status = "mismatch"
    elif not has_timing:
        status = "plain"
    else:
        status = "ok"

    return {
        "type": "lyric_verify",
        "status": status,
        "confidence": conf,
        "has_timing": has_timing,
        "heard": heard[:120] if heard else "",
        "lrc_path": resolved_lrc,
    }
