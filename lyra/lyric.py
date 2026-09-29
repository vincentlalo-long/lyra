import re
import urllib.request
from typing import List, Tuple, Optional


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

    cue_pattern = re.compile(r"^((?:\d{2}:)?\d{2}:\d{2}[\.,]\d{3})\s+-->")

    current_start = None
    current_texts = []

    for line in lines:
        line = line.strip()
        if not line or line.startswith("WEBVTT") or line.startswith("NOTE"):
            continue

        match = cue_pattern.match(line)
        if match:
            if current_start is not None and current_texts:
                full_text = " ".join(current_texts).strip()
                if full_text:
                    cues.append((current_start, full_text))
                current_texts = []

            current_start = _timestamp_to_seconds(match.group(1))
        elif current_start is not None:
            clean_text = re.sub(r"<[^>]+>", "", line).strip()
            clean_text = re.sub(r"^[-–—]\s*", "", clean_text)
            if clean_text:
                current_texts.append(clean_text)

    if current_start is not None and current_texts:
        full_text = " ".join(current_texts).strip()
        if full_text:
            cues.append((current_start, full_text))

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


def fetch_best_vtt(info: dict, target_langs: Optional[List[str]] = None) -> Optional[str]:
    # Check if target languages exist in subtitles or automatic captions
    if target_langs is None:
        target_langs = ["en", "vi"]

    subs = info.get("subtitles") or {}
    auto = info.get("automatic_captions") or {}

    selected_formats = None

    # 1. Check manual subtitles first
    for lang in target_langs:
        for k in subs.keys():
            if k == lang or k.startswith(f"{lang}-"):
                selected_formats = subs[k]
                break
        if selected_formats:
            break

    # 2. Check automatic captions next
    if not selected_formats:
        for lang in target_langs:
            for k in auto.keys():
                if k == lang or k.startswith(f"{lang}-"):
                    selected_formats = auto[k]
                    break
            if selected_formats:
                break

    # Early exit: no matching subtitles available
    if not selected_formats:
        return None

    # Find VTT format
    vtt_fmt = next((f for f in selected_formats if f.get("ext") == "vtt"), None)
    if not vtt_fmt or not vtt_fmt.get("url"):
        return None

    # Safely fetch VTT text, suppressing 429/network errors
    try:
        req = urllib.request.Request(
            vtt_fmt["url"],
            headers={"User-Agent": "Mozilla/5.0 (X11; Linux x86_64)"}
        )
        with urllib.request.urlopen(req, timeout=8) as resp:
            return resp.read().decode("utf-8", errors="ignore")
    except Exception:
        return None
