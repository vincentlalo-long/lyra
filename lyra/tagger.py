import os
from typing import Optional
from mutagen.mp3 import MP3
from mutagen.id3 import (
    ID3,
    TIT2,
    TPE1,
    TALB,
    TCON,
    APIC,
    USLT,
    ID3NoHeaderError,
)


def suggest_lyrics_lang(title: str = "", artist: str = "", lrc_text: str = "") -> str:
    """Suggest the USLT lang code: 'vie' for Vietnamese diacritics, else 'eng'.

    Shared by the download pipeline and the vi-vietnamese-support plugin,
    replacing the previously hardcoded 'eng' value.
    """
    try:
        from plugins.vi_vietnamese_support.plugin import is_vietnamese
    except Exception:
        # Lightweight fallback for standalone runs (plugin not on PYTHONPATH).
        def is_vietnamese(text: str) -> bool:  # type: ignore[misc]
            vi = set("àáạảãâầấậẩẫăằắặẳẵèéẹẻẽêềếệểễìíịỉĩòóọỏõôồốộổỗơờớợởỡùúụủũưừứựửữỳýỵỷỹđĐ")
            return any(c in vi for c in (text or ""))

    blob = f"{title} {artist} {lrc_text or ''}"
    return "vie" if is_vietnamese(blob) else "eng"


def tag_audio_file(
    file_path: str,
    title: str,
    artist: str,
    album: Optional[str] = None,
    cover_path: Optional[str] = None,
    lrc_text: Optional[str] = None,
    genre: Optional[str] = None,
) -> bool:
    if not os.path.exists(file_path):
        return False

    try:
        try:
            tags = ID3(file_path)
        except ID3NoHeaderError:
            tags = ID3()

        # ID3 text tags
        tags.add(TIT2(encoding=3, text=title))
        tags.add(TPE1(encoding=3, text=artist))
        tags.add(TALB(encoding=3, text=album or f"{title} - Single"))
        if genre and genre.strip():
            tags.add(TCON(encoding=3, text=genre.strip()))

        # Embed front cover art
        if cover_path and os.path.exists(cover_path):
            with open(cover_path, "rb") as img_file:
                tags.add(
                    APIC(
                        encoding=3,
                        mime="image/jpeg",
                        type=3,
                        desc="Cover",
                        data=img_file.read(),
                    )
                )

        # Embed unsynced lyrics tag for fallback player support
        if lrc_text:
            import re
            plain_lyrics = re.sub(r"\[\d{2}:\d{2}\.\d{2}\]\s*", "", lrc_text)
            tags.add(
                USLT(
                    encoding=3,
                    lang=suggest_lyrics_lang(title, artist, lrc_text),
                    desc="Lyrics",
                    text=plain_lyrics,
                )
            )

        tags.save(file_path, v2_version=3)
        return True
    except Exception as e:
        import sys
        print(f"warning: failed to tag audio file: {e}", file=sys.stderr)
        return False
