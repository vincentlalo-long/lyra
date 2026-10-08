import re
import html
import urllib.request
from typing import List, Tuple, Optional, Dict


def _timestamp_to_seconds(ts_str: str) -> float:
    # Convert 'hh:mm:ss.xxx' or 'mm:ss.xxx' to seconds
    parts = ts_str.strip().replace(",", ".").split(":")
    if len(parts) == 3:
        return int(parts[0]) * 3600 + int(parts[1]) * 60 + float(parts[2])
    elif len(parts) == 2:
        return int(parts[0]) * 60 + float(parts[1])
    return 0.0


def _seconds_to_lrc_tag(seconds: float) -> str:
    # Convert seconds into [mm:ss.xx]
    mins = int(seconds // 60)
    secs = seconds % 60
    return f"[{mins:02d}:{secs:05.2f}]"


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
