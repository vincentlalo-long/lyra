# Vietnamese Support Plugin for Lyra

First community plugin: support for Vietnamese (V-Pop) tracks — synced
`.lrc` lookup via LRCLIB, two-way accent-insensitive matching, NFC
normalization, and `lang="vie"` tagging hints.

## What it provides

| Action | Input | Output |
|---|---|---|
| `lyrics` | `artist` + `title` (accented or plain) | `{found, lrc_text, lang, source}` via LRCLIB |
| `match` | `query` + `target` | accent-insensitive contains (`son tung` ↔ `Sơn Tùng M-TP`) |
| `normalize` | `text` | NFC text + ascii fold + `is_vietnamese` |
| `detect` | `text` | `suggest_lang: vie \| eng` for ID3 USLT |

What it does NOT do: full TUI translation, Zing/NCT providers, or OCR
for burned-in video lyrics (see `lyra/lyric.py` captions and
`ocr-video-lyrics`).

## Install

No Rust rebuild needed. Python stdlib only, no extra dependencies:

```bash
python3 -c "from plugins.vi_vietnamese_support.plugin import run; print(run({'action':'detect','text':'Son Tung'}))"
```

## Usage

```python
from plugins.vi_vietnamese_support.plugin import run

# 1. Find lyrics for a Vietnamese track (accented or plain both work)
run({"action": "lyrics", "artist": "Son Tung M-TP", "title": "Lac Troi"})
run({"action": "lyrics", "artist": "Son Tung", "title": "Lac Troi"})  # folded fallback

# 2. Accent-insensitive matching for TUI search/filter
run({"action": "match", "query": "son tung", "target": "Son Tung M-TP"})

# 3. Display normalization (NFC) + Vietnamese detection
run({"action": "normalize", "text": "Son Tung"})
run({"action": "detect", "text": "Nau An Cho Em"})  # -> suggest_lang vie
```

A `lyrics` result looks like:

```json
{"status": "success", "data": {"found": true, "lrc_text": "[00:12.00] ...", "lang": "vie", "source": "lrclib"}}
```

When `lang == "vie"`, the tagging pipeline should use `USLT(lang="vie")`
instead of hardcoded `"eng"` — see `suggest_lyrics_lang()` in
`lyra/tagger.py`.

## Registration

Registered in `registry.json`:

```json
{
  "id": "vi-vietnamese-support",
  "name": "Vietnamese Lyrics & Search",
  "version": "v0.1.0",
  "author": "@vincentlalo-long",
  "description": "Synced Vietnamese lyrics via LRCLIB + diacritics-insensitive search + vie tagging",
  "entry": "plugins/vi_vietnamese_support",
  "status": "active"
}
```

The TUI reads it in `MusicPlayer/src/plugin.rs::available_community_plugins()`
and shows it in the Plugin Store (tab `5`).

## Limits

- LRCLIB misses some indie V-Pop tracks → falls back to YouTube captions
  (`lyra/lyric.py:fetch_all_vtt(["vi","en"])`).
- Hardcoded (burned-in) lyric videos need OCR — reserved for `ocr-video-lyrics`.
