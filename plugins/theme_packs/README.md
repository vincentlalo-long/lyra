# Theme Packs for Lyra

Ready-made color presets for Lyra's semantic theme system. The Rust core
defines UI **roles** (`MusicPlayer/src/theme.rs::Theme`); every widget
should use roles (`app.theme.accent`) instead of raw palette colors.
Switching a preset recolors the whole TUI consistently — no more two
mixed-up colors.

## Presets

| Name | Palette | Style |
|---|---|---|
| `mocha` (default) | Catppuccin Mocha | dark, pastel purple accent |
| `gruvbox` | Gruvbox Dark | dark, warm retro orange accent |
| `tokyo-night` | Tokyo Night | dark, neon cyan accent on deep navy |
| `dracula` | Dracula | dark, hot-pink accent on purple-gray |
| `nord` | Nord | dark, calm frost-blue accent, low contrast |
| `latte` | Catppuccin Latte | light, dark-ink text on paper |

## Switching

Fastest: press `X` in the TUI (cycles mocha → gruvbox → tokyo-night → dracula → nord → latte).

Persistent options:

```toml
# ~/.config/lyra/config.toml
theme = "gruvbox"
```

```bash
LYRA_THEME=tokyo-night lyra ~/Music
```

```python
from plugins.theme_packs.plugin import run
run({"action": "list"})
run({"action": "apply", "name": "gruvbox"})
```

`list` (v0.1.1+) returns harmonious text colors per theme too — each
preset carries `text` / `muted` / `faint` / `border` / `selected` roles
with exact hex values sampled from its palette, e.g. dracula
`text #f8f8f2 / muted #6272a4`, nord `text #eceff4 / muted #b9c2d0`,
latte `text #4c4f69 / muted #6c6f85`. Use them when styling custom
widgets so text stays readable on every theme.

## Role table (for theme authors)

| Role | Used for |
|---|---|
| `accent` | Active tab, progress fill, focused borders, selection caret |
| `accent_dim` | Secondary highlights |
| `border` / `border_focused` | Inactive / active panel borders |
| `text` / `muted` / `faint` | Primary / secondary / placeholder text |
| `success` / `warning` / `danger` / `info` / `hint` | Status semantics |
| `selected_bg` / `selected_fg` | Selected row pill |
| `progress_fill` / `progress_track` | Seekbar fill and empty track |

Migration rule: new UI code must use `app.theme.<role>`; raw
`theme::MAUVE/BLUE/...` constants remain only for backward compatibility
until each `ui/*.rs` file is migrated (controls + top bar done, rest
tracked in `theme.rs` docs).
