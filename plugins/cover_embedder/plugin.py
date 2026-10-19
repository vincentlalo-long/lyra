"""Cover art embedder for Lyra (cover-embedder).

Embeds album art (cover.jpg / cover.png next to the track) into the
track's ID3 APIC frame using mutagen. Designed to run after a download
completes so the cover travels with the audio file.

Actions:
  {"action": "embed", "file": "/path/to/song.mp3", "cover": "/path/to/cover.jpg"}
  {"action": "embed_dir", "dir": "/path/to/music/folder"} -> embeds cover.jpg/png
                                                      next to every .mp3 in dir
"""

from __future__ import annotations

import os
import sys
from typing import Any, Dict

try:
    from mutagen.id3 import ID3, APIC
    from mutagen.mp3 import MP3
except ImportError:
    ID3 = None  # type: ignore[assignment]
    APIC = None  # type: ignore[assignment]
    MP3 = None  # type: ignore[assignment]


VALID_IMAGE_EXTS = {".jpg", ".jpeg", ".png", ".webp"}


def _mime_type(cover_path: str) -> str:
    ext = os.path.splitext(cover_path)[1].lower()
    if ext == ".png":
        return "image/png"
    if ext == ".webp":
        return "image/webp"
    return "image/jpeg"


def _find_cover(song_path: str) -> str | None:
    """Look for cover.jpg / cover.png next to the song."""
    base = os.path.splitext(song_path)[0]
    parent = os.path.dirname(song_path)
    for ext in VALID_IMAGE_EXTS:
        candidate = base + ext
        if os.path.isfile(candidate):
            return candidate
        candidate = os.path.join(parent, "cover" + ext)
        if os.path.isfile(candidate):
            return candidate
    return None


def embed(file_path: str, cover_path: str | None = None) -> Dict[str, Any]:
    if ID3 is None or APIC is None or MP3 is None:
        return {"status": "error", "message": "mutagen is not installed (pip install mutagen)"}

    if not os.path.isfile(file_path):
        return {"status": "error", "message": f"File not found: {file_path}"}

    if cover_path is None:
        cover_path = _find_cover(file_path)
    if cover_path is None or not os.path.isfile(cover_path):
        return {"status": "error", "message": f"No cover art found next to {file_path}"}

    try:
        audio = MP3(file_path, ID3=ID3)
        try:
            tags = audio.tags
        except Exception:
            tags = ID3()
            audio.tags = tags

        # Remove existing APIC frames
        tags.delall("APIC")

        with open(cover_path, "rb") as f:
            cover_data = f.read()

        tags.add(
            APIC(
                encoding=3,
                mime=_mime_type(cover_path),
                type=3,  # Cover front
                desc="Cover",
                data=cover_data,
            )
        )
        audio.save(v2_version=3)
        return {
            "status": "success",
            "message": f"Embedded cover from {cover_path} into {file_path}",
            "data": {"file": file_path, "cover": cover_path},
        }
    except Exception as e:
        return {"status": "error", "message": f"Failed to embed cover: {e}"}


def run(payload: Dict[str, Any]) -> Dict[str, Any]:
    action = payload.get("action", "")

    if action == "embed":
        file_path = payload.get("file", "")
        cover_path = payload.get("cover")
        return embed(file_path, cover_path)

    if action == "embed_dir":
        dir_path = payload.get("dir", "")
        if not os.path.isdir(dir_path):
            return {"status": "error", "message": f"Directory not found: {dir_path}"}
        results = []
        for name in sorted(os.listdir(dir_path)):
            if name.lower().endswith(".mp3"):
                full = os.path.join(dir_path, name)
                results.append(embed(full))
        ok = sum(1 for r in results if r.get("status") == "success")
        fail = len(results) - ok
        return {
            "status": "success",
            "message": f"Processed {len(results)} MP3s: {ok} embedded, {fail} failed",
            "data": {"results": results},
        }

    return {"status": "error", "message": f"Unknown action: {action}"}


if __name__ == "__main__":
    import json

    if len(sys.argv) > 1:
        payload = json.loads(sys.argv[1])
    else:
        payload = {}
    print(json.dumps(run(payload)))
