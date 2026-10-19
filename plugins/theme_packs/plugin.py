"""Theme packs for Lyra (theme-packs).

Lists the built-in semantic theme presets and applies one by writing it
to the user config. The Rust core owns the actual rendering roles
(MusicPlayer/src/theme.rs::Theme); this plugin is the discoverable
store entry + helper for switching without hand-editing TOML.

Actions:
  {"action": "list"}            -> available presets with roles
  {"action": "apply", "name": "gruvbox"} -> writes theme to config.toml
"""

from __future__ import annotations

import os
from typing import Any, Dict, List

PRESETS: List[Dict[str, Any]] = [
    {
        "name": "mocha",
        "palette": "Catppuccin Mocha",
        "style": "dark, pastel purple accent",
        "roles": {"accent": "mauve #cba6f7", "info": "blue #89b4fa",
                  "success": "green #a6e3a1", "warning": "yellow #f9e2af",
                  "danger": "red #f38ba8", "hint": "peach #fab387",
                  "text": "text #cdd6f4", "muted": "subtext #a6adc8",
                  "faint": "overlay #6c7086", "border": "surface #45475a",
                  "selected": "mauve on surface #313244"},
    },
    {
        "name": "gruvbox",
        "palette": "Gruvbox Dark",
        "style": "dark, warm retro orange accent",
        "roles": {"accent": "orange #fe8019", "info": "aqua #83a598",
                  "success": "green #b8bb26", "warning": "yellow #fabd2f",
                  "danger": "red #fb4934", "hint": "yellow #fabd2f",
                  "text": "fg #ebdbb2", "muted": "gray #928374",
                  "faint": "dark-gray #665c54", "border": "dark #3c3836",
                  "selected": "orange on dark #3c3836"},
    },
    {
        "name": "tokyo-night",
        "palette": "Tokyo Night",
        "style": "dark, neon cyan accent on deep navy",
        "roles": {"accent": "cyan #7dcfff", "info": "blue #7aa2f7",
                  "success": "green #9ece6a", "warning": "yellow #e0af68",
                  "danger": "red #f7768e", "hint": "purple #bb9af7",
                  "text": "fg #c0caf5", "muted": "comment #565f89",
                  "faint": "dark #3b4261", "border": "dark #292e42",
                  "selected": "cyan on dark #292e42"},
    },
    {
        "name": "dracula",
        "palette": "Dracula",
        "style": "dark, hot-pink accent on purple-gray",
        "roles": {"accent": "pink #ff79c6", "info": "cyan #8be9fd",
                  "success": "green #50fa7b", "warning": "yellow #f1fa8c",
                  "danger": "red #ff5555", "hint": "orange #ffb86c",
                  "text": "fg #f8f8f2", "muted": "comment #6272a4",
                  "faint": "selection #44475a", "border": "selection #44475a",
                  "selected": "pink on selection #44475a"},
    },
    {
        "name": "nord",
        "palette": "Nord",
        "style": "dark, calm frost-blue accent, low contrast",
        "roles": {"accent": "frost #88c0d0", "info": "blue #81a1c1",
                  "success": "green #a3be8c", "warning": "yellow #ebcb8b",
                  "danger": "red #bf616a", "hint": "orange #d08770",
                  "text": "snow #eceff4", "muted": "storm #b9c2d0",
                  "faint": "polar #4c566a", "border": "night #3b4252",
                  "selected": "frost on night #3b4252"},
    },
    {
        "name": "latte",
        "palette": "Catppuccin Latte",
        "style": "light, dark-ink text on paper, purple accent",
        "roles": {"accent": "mauve #8839ef", "info": "blue #1e66f5",
                  "success": "green #40a02b", "warning": "yellow #df8e1d",
                  "danger": "red #d20f39", "hint": "peach #fe640b",
                  "text": "ink #4c4f69", "muted": "subtext #6c6f85",
                  "faint": "overlay #9ca0b0", "border": "surface #ccd0da",
                  "selected": "mauve on surface #ccd0da"},
    },
]

VALID = {p["name"] for p in PRESETS}


def _config_path() -> str:
    base = os.environ.get("XDG_CONFIG_HOME") or os.path.join(
        os.path.expanduser("~"), ".config")
    return os.path.join(base, "lyra", "config.toml")


def run(payload: Dict[str, Any]) -> Dict[str, Any]:
    action = (payload or {}).get("action", "list")
    try:
        if action == "apply":
            name = str((payload or {}).get("name", "")).lower().replace("_", "-")
            if name not in VALID:
                return {"status": "error",
                        "message": f"Unknown theme '{name}'. Available: {sorted(VALID)}",
                        "data": {"available": sorted(VALID)}}
            path = _config_path()
            try:
                os.makedirs(os.path.dirname(path), exist_ok=True)
                existing = ""
                if os.path.exists(path):
                    with open(path, encoding="utf-8") as f:
                        existing = f.read()
                lines = [l for l in existing.splitlines()
                         if not l.strip().startswith("theme")]
                lines.append(f'theme = "{name}"')
                with open(path, "w", encoding="utf-8") as f:
                    f.write("\n".join(lines) + "\n")
            except Exception as e:
                return {"status": "error",
                        "message": f"Could not write {path}: {e}", "data": {}}
            return {"status": "success",
                    "message": f"Theme '{name}' saved to {path}. "
                               "Press 'Z' in lyra to reload without restarting, "
                               "or restart lyra / set LYRA_THEME to apply.",
                    "data": {"name": name, "config": path}}
        # default: list
        return {"status": "success",
                "message": "Available theme presets (semantic roles, see theme.rs)",
                "data": {"themes": PRESETS}}
    except Exception as e:  # Never crash the host.
        return {"status": "error", "message": f"theme-packs error: {e}", "data": {}}
