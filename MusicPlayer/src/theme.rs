#![allow(dead_code)]

//! Theme system: raw palettes + semantic roles.
//!
//! Design (borrowed from lazygit/k9s/neovim theme engines):
//! - A **palette** is raw colors (Catppuccin Mocha, Gruvbox, Tokyo Night).
//! - A **Theme** maps each UI *role* to one palette color.
//! - All `ui/*.rs` code must use roles (`app.theme.accent`), never raw
//!   palette constants directly. That ends the "two mixed-up colors"
//!   problem: changing one role updates every widget consistently.
//!
//! Role table:
//! | Role | Used for |
//! |---|---|
//! | `accent` | Active tab, progress fill, key widget borders, selection caret |
//! | `accent_dim` | Secondary highlights |
//! | `border` / `border_focused` | Inactive / active panel borders |
//! | `text` / `muted` / `faint` | Primary / secondary / placeholder text |
//! | `success` / `warning` / `danger` / `info` / `hint` | Status semantics |
//! | `ink` | Dark text on bright pills (Enabled/Installed badges) |
//! | `selected_bg` / `selected_fg` | Selected row pill |
//! | `progress_track` | Seekbar/volume empty track background |
//!
//! Switching: `LYRA_THEME` env or `theme = "gruvbox"` in
//! `~/.config/lyra/config.toml`. Unknown names fall back to Mocha.
//! Ready-made TOML packs live in `plugins/theme_packs/`.

use ratatui::style::Color;

// ---------------------------------------------------------------------------
// Raw palettes (kept as constants for backward compatibility)
// ---------------------------------------------------------------------------

// Catppuccin Mocha Palette
pub const MAUVE: Color = Color::Rgb(203, 166, 247);
pub const BLUE: Color = Color::Rgb(137, 180, 250);
pub const YELLOW: Color = Color::Rgb(249, 226, 175);
pub const GREEN: Color = Color::Rgb(166, 227, 161);
pub const RED: Color = Color::Rgb(243, 139, 168);
pub const ROSE: Color = Color::Rgb(245, 194, 231);
pub const PEACH: Color = Color::Rgb(250, 179, 135);
pub const PURPLE: Color = Color::Rgb(203, 166, 247); // Alias of MAUVE
pub const TEXT: Color = Color::Rgb(205, 214, 244);
pub const SUBTEXT0: Color = Color::Rgb(166, 173, 200);
pub const OVERLAY0: Color = Color::Rgb(108, 112, 134);
pub const SURFACE2: Color = Color::Rgb(88, 91, 112);
pub const SURFACE1: Color = Color::Rgb(69, 71, 90);
pub const SURFACE0: Color = Color::Rgb(49, 50, 68);
pub const BASE: Color = Color::Rgb(30, 30, 46);

// Gruvbox Dark Palette (subset)
pub const GRUVBOX_PURPLE: Color = Color::Rgb(211, 134, 155);
pub const GRUVBOX_BLUE: Color = Color::Rgb(131, 165, 152);
pub const GRUVBOX_YELLOW: Color = Color::Rgb(250, 189, 47);
pub const GRUVBOX_GREEN: Color = Color::Rgb(184, 187, 38);
pub const GRUVBOX_RED: Color = Color::Rgb(251, 73, 52);
pub const GRUVBOX_ORANGE: Color = Color::Rgb(254, 128, 25);
pub const GRUVBOX_FG: Color = Color::Rgb(235, 219, 178);
pub const GRUVBOX_GRAY: Color = Color::Rgb(146, 131, 116);
pub const GRUVBOX_DARK: Color = Color::Rgb(60, 56, 54);
pub const GRUVBOX_BG: Color = Color::Rgb(40, 40, 40);

// Tokyo Night Palette (subset)
pub const TOKYO_PURPLE: Color = Color::Rgb(187, 154, 247);
pub const TOKYO_BLUE: Color = Color::Rgb(122, 162, 247);
pub const TOKYO_YELLOW: Color = Color::Rgb(224, 175, 104);
pub const TOKYO_GREEN: Color = Color::Rgb(158, 206, 106);
pub const TOKYO_RED: Color = Color::Rgb(247, 118, 142);
pub const TOKYO_CYAN: Color = Color::Rgb(125, 207, 255);
pub const TOKYO_FG: Color = Color::Rgb(192, 202, 245);
pub const TOKYO_GRAY: Color = Color::Rgb(89, 95, 119);
pub const TOKYO_DARK: Color = Color::Rgb(41, 46, 66);
pub const TOKYO_BG: Color = Color::Rgb(26, 27, 38);

// Dracula Palette (subset)
pub const DRACULA_PINK: Color = Color::Rgb(255, 121, 198);
pub const DRACULA_CYAN: Color = Color::Rgb(139, 233, 253);
pub const DRACULA_GREEN: Color = Color::Rgb(80, 250, 123);
pub const DRACULA_YELLOW: Color = Color::Rgb(241, 250, 140);
pub const DRACULA_RED: Color = Color::Rgb(255, 85, 85);
pub const DRACULA_ORANGE: Color = Color::Rgb(255, 184, 108);
pub const DRACULA_PURPLE: Color = Color::Rgb(189, 147, 249);
pub const DRACULA_FG: Color = Color::Rgb(248, 248, 242);
pub const DRACULA_GRAY: Color = Color::Rgb(98, 114, 164);
pub const DRACULA_DARK: Color = Color::Rgb(68, 71, 90);
pub const DRACULA_BG: Color = Color::Rgb(40, 42, 54);

// Nord Palette (subset)
pub const NORD_FROST_LIGHT: Color = Color::Rgb(136, 192, 208);
pub const NORD_FROST: Color = Color::Rgb(129, 161, 193);
pub const NORD_FROST_DIM: Color = Color::Rgb(143, 188, 187);
pub const NORD_GREEN: Color = Color::Rgb(163, 190, 140);
pub const NORD_YELLOW: Color = Color::Rgb(235, 203, 139);
pub const NORD_RED: Color = Color::Rgb(191, 97, 106);
pub const NORD_ORANGE: Color = Color::Rgb(208, 135, 112);
pub const NORD_FG: Color = Color::Rgb(236, 239, 244);
pub const NORD_GRAY: Color = Color::Rgb(76, 86, 106);
pub const NORD_DARK: Color = Color::Rgb(59, 66, 82);
pub const NORD_BG: Color = Color::Rgb(46, 52, 64);

// Catppuccin Latte Palette (light subset)
pub const LATTE_MAUVE: Color = Color::Rgb(136, 57, 239);
pub const LATTE_PINK: Color = Color::Rgb(234, 118, 203);
pub const LATTE_BLUE: Color = Color::Rgb(30, 102, 245);
pub const LATTE_YELLOW: Color = Color::Rgb(223, 142, 29);
pub const LATTE_GREEN: Color = Color::Rgb(64, 160, 43);
pub const LATTE_RED: Color = Color::Rgb(210, 15, 57);
pub const LATTE_PEACH: Color = Color::Rgb(254, 100, 11);
pub const LATTE_FG: Color = Color::Rgb(76, 79, 105);
pub const LATTE_GRAY: Color = Color::Rgb(108, 111, 133);
pub const LATTE_FAINT: Color = Color::Rgb(156, 160, 176);
pub const LATTE_DARK: Color = Color::Rgb(204, 208, 218);
pub const LATTE_BG: Color = Color::Rgb(239, 241, 245);

// ---------------------------------------------------------------------------
// Semantic theme
// ---------------------------------------------------------------------------

/// Semantic UI theme. Construct via `Theme::mocha()` (default),
/// `Theme::gruvbox()`, `Theme::tokyo_night()`, or `Theme::from_name()`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Theme {
    pub name: &'static str,
    pub accent: Color,
    pub accent_dim: Color,
    pub border: Color,
    pub border_focused: Color,
    pub text: Color,
    pub muted: Color,
    pub faint: Color,
    pub success: Color,
    pub warning: Color,
    pub danger: Color,
    pub info: Color,
    pub hint: Color,
    /// Dark foreground for text drawn on top of bright pills
    /// (e.g. "Enabled", "Installed"). Always dark across presets.
    pub ink: Color,
    pub selected_bg: Color,
    pub selected_fg: Color,
    pub progress_fill: Color,
    pub progress_track: Color,
}

impl Theme {
    pub fn mocha() -> Self {
        Self {
            name: "mocha",
            accent: MAUVE,
            accent_dim: ROSE,
            border: SURFACE1,
            border_focused: MAUVE,
            text: TEXT,
            muted: SUBTEXT0,
            faint: OVERLAY0,
            success: GREEN,
            warning: YELLOW,
            danger: RED,
            info: BLUE,
            hint: PEACH,
            ink: BASE,
            selected_bg: SURFACE0,
            selected_fg: MAUVE,
            progress_fill: MAUVE,
            progress_track: SURFACE0,
        }
    }

    pub fn gruvbox() -> Self {
        Self {
            name: "gruvbox",
            accent: GRUVBOX_ORANGE,
            accent_dim: GRUVBOX_YELLOW,
            border: GRUVBOX_DARK,
            border_focused: GRUVBOX_ORANGE,
            text: GRUVBOX_FG,
            muted: GRUVBOX_GRAY,
            faint: GRUVBOX_DARK,
            success: GRUVBOX_GREEN,
            warning: GRUVBOX_YELLOW,
            danger: GRUVBOX_RED,
            info: GRUVBOX_BLUE,
            hint: GRUVBOX_YELLOW,
            ink: GRUVBOX_BG,
            selected_bg: GRUVBOX_DARK,
            selected_fg: GRUVBOX_ORANGE,
            progress_fill: GRUVBOX_ORANGE,
            progress_track: GRUVBOX_DARK,
        }
    }

    pub fn tokyo_night() -> Self {
        // Signature neon-cyan accent (deliberately NOT purple, so it never
        // reads as "mocha again"). Pairs with deep-navy surfaces.
        Self {
            name: "tokyo-night",
            accent: TOKYO_CYAN,
            accent_dim: TOKYO_BLUE,
            border: TOKYO_DARK,
            border_focused: TOKYO_CYAN,
            text: TOKYO_FG,
            muted: TOKYO_GRAY,
            faint: TOKYO_GRAY,
            success: TOKYO_GREEN,
            warning: TOKYO_YELLOW,
            danger: TOKYO_RED,
            info: TOKYO_BLUE,
            hint: TOKYO_PURPLE,
            ink: TOKYO_BG,
            selected_bg: TOKYO_DARK,
            selected_fg: TOKYO_CYAN,
            progress_fill: TOKYO_CYAN,
            progress_track: TOKYO_DARK,
        }
    }

    pub fn dracula() -> Self {
        // Signature hot-pink accent on a vivid dark-purple background.
        Self {
            name: "dracula",
            accent: DRACULA_PINK,
            accent_dim: DRACULA_PURPLE,
            border: DRACULA_DARK,
            border_focused: DRACULA_PINK,
            text: DRACULA_FG,
            muted: DRACULA_GRAY,
            faint: DRACULA_GRAY,
            success: DRACULA_GREEN,
            warning: DRACULA_YELLOW,
            danger: DRACULA_RED,
            info: DRACULA_CYAN,
            hint: DRACULA_ORANGE,
            ink: DRACULA_BG,
            selected_bg: DRACULA_DARK,
            selected_fg: DRACULA_PINK,
            progress_fill: DRACULA_PINK,
            progress_track: DRACULA_DARK,
        }
    }

    pub fn nord() -> Self {
        // Signature frost-blue accent, calm low-contrast polar palette.
        Self {
            name: "nord",
            accent: NORD_FROST_LIGHT,
            accent_dim: NORD_FROST_DIM,
            border: NORD_DARK,
            border_focused: NORD_FROST_LIGHT,
            text: NORD_FG,
            muted: NORD_GRAY,
            faint: NORD_GRAY,
            success: NORD_GREEN,
            warning: NORD_YELLOW,
            danger: NORD_RED,
            info: NORD_FROST,
            hint: NORD_ORANGE,
            ink: NORD_BG,
            selected_bg: NORD_DARK,
            selected_fg: NORD_FROST_LIGHT,
            progress_fill: NORD_FROST_LIGHT,
            progress_track: NORD_DARK,
        }
    }

    pub fn latte() -> Self {
        // The only light theme: dark ink text on a paper background.
        Self {
            name: "latte",
            accent: LATTE_MAUVE,
            accent_dim: LATTE_PINK,
            border: LATTE_DARK,
            border_focused: LATTE_MAUVE,
            text: LATTE_FG,
            muted: LATTE_GRAY,
            faint: LATTE_FAINT,
            success: LATTE_GREEN,
            warning: LATTE_YELLOW,
            danger: LATTE_RED,
            info: LATTE_BLUE,
            hint: LATTE_PEACH,
            ink: LATTE_BG,
            selected_bg: LATTE_DARK,
            selected_fg: LATTE_MAUVE,
            progress_fill: LATTE_MAUVE,
            progress_track: LATTE_DARK,
        }
    }

    /// Resolve by name (`mocha` default). Case/dash-insensitive:
    /// `TokyoNight`, `tokyo-night`, `gruvbox` all work.
    pub fn from_name(name: &str) -> Self {
        let key: String = name.to_ascii_lowercase().replace(['_', ' '], "-");
        match key.as_str() {
            "gruvbox" => Self::gruvbox(),
            "tokyo-night" | "tokyonight" | "tokyo" => Self::tokyo_night(),
            "dracula" => Self::dracula(),
            "nord" => Self::nord(),
            "latte" | "catppuccin-latte" => Self::latte(),
            _ => Self::mocha(),
        }
    }

    pub fn available() -> Vec<&'static str> {
        vec!["mocha", "gruvbox", "tokyo-night", "dracula", "nord", "latte"]
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::mocha()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_name_known_and_fallback() {
        assert_eq!(Theme::from_name("mocha").name, "mocha");
        assert_eq!(Theme::from_name("gruvbox").name, "gruvbox");
        assert_eq!(Theme::from_name("TokyoNight").name, "tokyo-night");
        assert_eq!(Theme::from_name("DRACULA").name, "dracula");
        assert_eq!(Theme::from_name("nord").name, "nord");
        assert_eq!(Theme::from_name("catppuccin-latte").name, "latte");
        assert_eq!(Theme::from_name("nope").name, "mocha");
    }

    #[test]
    fn test_roles_differ_per_preset() {
        let accents: Vec<_> = [
            Theme::mocha(),
            Theme::gruvbox(),
            Theme::tokyo_night(),
            Theme::dracula(),
            Theme::nord(),
            Theme::latte(),
        ]
        .iter()
        .map(|t| t.accent)
        .collect();
        let mut unique = accents.clone();
        unique.sort_by_key(|c| format!("{c:?}"));
        unique.dedup();
        // Every preset must have a visually distinct signature accent.
        assert_eq!(unique.len(), accents.len());
        assert_eq!(Theme::available().len(), 6);
    }
}
