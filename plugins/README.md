# Lyra Community & Sample Plugins

This directory contains standalone, modular plugin extensions for Lyra.

Community plugins are optional extensions that can be maintained independently of the core TUI player.

---

## Directory Structure

```text
plugins/
├── README.md                 # This documentation
├── ocr_video_lyrics/         # OCR hardcoded subtitle to LRC extractor (In development)
│   ├── README.md
│   └── plugin.py
└── template_plugin/          # Minimal boilerplate template for new plugins
    ├── README.md
    └── plugin.py
```

---

## Plugin Manifest (`registry.json`)

All community plugins available for discovery and download in the Lyra Plugin Store must be registered in [`registry.json`](../registry.json) at the root of the repository:

```json
[
  {
    "id": "ocr-video-lyrics",
    "name": "OCR Video HardCode Lyric to lrc",
    "version": "Coming Soon",
    "author": "@vincentlalo-long",
    "description": "Extract hardcoded subtitles from music videos via OCR into synced .lrc lyrics (Coming Soon)",
    "entry": "plugins/ocr_video_lyrics",
    "status": "planned"
  }
]
```

### Manifest Fields

| Field | Type | Description |
|---|---|---|
| `id` | `string` | Unique identifier (lowercase alphanumeric with hyphens) |
| `name` | `string` | Display name in the TUI store modal |
| `version` | `string` | Semantic version (e.g. `v1.0.0`) or `Coming Soon` |
| `author` | `string` | Author GitHub username or handle (e.g. `@author`) |
| `description` | `string` | Brief description of functionality |
| `entry` | `string` | Relative path to plugin directory or script |
| `status` | `string` | `active`, `beta`, or `planned` |

---

## Creating a New Plugin

1. Copy `plugins/template_plugin/` to `plugins/<your_plugin_id>/`.
2. Implement your logic in `plugin.py`.
3. Add your plugin entry to `registry.json`.
4. Submit a Pull Request following the guidelines in [`CONTRIBUTING.md`](../CONTRIBUTING.md).
