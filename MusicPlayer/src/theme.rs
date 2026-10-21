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

// Catppuccin Frappê Palette (muted medium subset)
pub const FRAPPE_ROSE: Color = Color::Rgb(235, 160, 172);
pub const FRAPPE_MAUVE: Color = Color::Rgb(152, 199, 243);
pub const FRAPPE_PINK: Color = Color::Rgb(232, 166, 229);
pub const FRAPPE_RED: Color = Color::Rgb(232, 102, 136);
pub const FRAPPE_MAROON: Color = Color::Rgb(229, 128, 128);
pub const FRAPPE_PEACH: Color = Color::Rgb(235, 188, 138);
pub const FRAPPE_YELLOW: Color = Color::Rgb(241, 227, 137);
pub const FRAPPE_GREEN: Color = Color::Rgb(133, 211, 153);
pub const FRAPPE_TEAL: Color = Color::Rgb(113, 203, 200);
pub const FRAPPE_SKY: Color = Color::Rgb(142, 213, 240);
pub const FRAPPE_SAPPHIRE: Color = Color::Rgb(103, 166, 229);
pub const FRAPPE_BLUE: Color = Color::Rgb(137, 180, 250);
pub const FRAPVE_LAVENDER: Color = Color::Rgb(194, 190, 248);
pub const FRAPPE_TEXT: Color = Color::Rgb(216, 222, 233);
pub const FRAPPE_SUBTEXT0: Color = Color::Rgb(186, 194, 222);
pub const FRAPPE_SUBTEXT1: Color = Color::Rgb(156, 164, 205);
pub const FRAPPE_OVERLAY0: Color = Color::Rgb(116, 124, 148);
pub const FRAPPE_OVERLAY1: Color = Color::Rgb(86, 94, 118);
pub const FRAPPE_OVERLAY2: Color = Color::Rgb(74, 77, 89);
pub const FRAPPE_SURFACE0: Color = Color::Rgb(62, 68, 83);
pub const FRAPPE_SURFACE1: Color = Color::Rgb(49, 53, 65);
pub const FRAPPE_SURFACE2: Color = Color::Rgb(36, 39, 46);
pub const FRAPPE_BASE: Color = Color::Rgb(30, 30, 46);
pub const FRAPPE_MANTLE: Color = Color::Rgb(24, 24, 37);
pub const FRAPPE_CRUST: Color = Color::Rgb(17, 17, 27);

// Catppuccin Macchiato Palette (light-medium subset)
pub const MACCHIATO_ROSE: Color = Color::Rgb(232, 156, 172);
pub const MACCHIATO_MAUVE: Color = Color::Rgb(177, 199, 252);
pub const MACCHIATO_PINK: Color = Color::Rgb(235, 154, 221);
pub const MACCHIATO_RED: Color = Color::Rgb(229, 116, 148);
pub const MACCHIATO_MAROON: Color = Color::Rgb(231, 120, 120);
pub const MACCHIATO_PEACH: Color = Color::Rgb(238, 176, 131);
pub const MACCHIATO_YELLOW: Color = Color::Rgb(245, 228, 140);
pub const MACCHIATO_GREEN: Color = Color::Rgb(143, 215, 161);
pub const MACCHIATO_TEAL: Color = Color::Rgb(127, 211, 207);
pub const MACCHIATO_SKY: Color = Color::Rgb(154, 219, 246);
pub const MACCHIATO_SAPPHIRE: Color = Color::Rgb(122, 172, 233);
pub const MACCHIATO_BLUE: Color = Color::Rgb(137, 220, 252);
pub const MACCHIATO_LAVENDER: Color = Color::Rgb(203, 193, 247);
pub const MACCHIATO_TEXT: Color = Color::Rgb(223, 229, 238);
pub const MACCHIATO_SUBTEXT0: Color = Color::Rgb(194, 202, 213);
pub const MACCHIATO_SUBTEXT1: Color = Color::Rgb(166, 174, 190);
pub const MACCHIATO_OVERLAY0: Color = Color::Rgb(138, 146, 164);
pub const MACCHIATO_OVERLAY1: Color = Color::Rgb(110, 118, 137);
pub const MACCHIATO_OVERLAY2: Color = Color::Rgb(126, 129, 152);
pub const MACCHIATO_SURFACE0: Color = Color::Rgb(104, 113, 129);
pub const MACCHIATO_SURFACE1: Color = Color::Rgb(82, 89, 104);
pub const MACCHIATO_SURFACE2: Color = Color::Rgb(59, 64, 77);
pub const MACCHIATO_BASE: Color = Color::Rgb(47, 50, 62);
pub const MACCHIATO_MANTLE: Color = Color::Rgb(39, 41, 51);
pub const MACCHIATO_CRUST: Color = Color::Rgb(32, 34, 43);

// Solarized Dark Palette
pub const SOLARIZED_BASE03: Color = Color::Rgb(0, 43, 54);
pub const SOLARIZED_BASE02: Color = Color::Rgb(7, 54, 66);
pub const SOLARIZED_BASE01: Color = Color::Rgb(88, 110, 117);
pub const SOLARIZED_BASE00: Color = Color::Rgb(101, 123, 131);
pub const SOLARIZED_BASE0: Color = Color::Rgb(131, 148, 150);
pub const SOLARIZED_BASE1: Color = Color::Rgb(147, 161, 161);
pub const SOLARIZED_BASE2: Color = Color::Rgb(214, 222, 224);
pub const SOLARIZED_BASE3: Color = Color::Rgb(230, 235, 235);
pub const SOLARIZED_YELLOW: Color = Color::Rgb(181, 137, 0);
pub const SOLARIZED_ORANGE: Color = Color::Rgb(203, 75, 22);
pub const SOLARIZED_RED: Color = Color::Rgb(220, 50, 47);
pub const SOLARIZED_MAGENTA: Color = Color::Rgb(211, 54, 130);
pub const SOLARIZED_VIOLET: Color = Color::Rgb(108, 113, 196);
pub const SOLARIZED_BLUE: Color = Color::Rgb(38, 139, 210);
pub const SOLARIZED_CYAN: Color = Color::Rgb(42, 161, 152);
pub const SOLARIZED_GREEN: Color = Color::Rgb(133, 153, 0);

// Solarized Light Palette
pub const SOLARIZED_LIGHT_YELLOW: Color = Color::Rgb(181, 137, 0);
pub const SOLARIZED_LIGHT_ORANGE: Color = Color::Rgb(203, 75, 22);
pub const SOLARIZED_LIGHT_RED: Color = Color::Rgb(220, 50, 47);
pub const SOLARIZED_LIGHT_MAGENTA: Color = Color::Rgb(211, 54, 130);
pub const SOLARIZED_LIGHT_VIOLET: Color = Color::Rgb(108, 113, 196);
pub const SOLARIZED_LIGHT_BLUE: Color = Color::Rgb(38, 139, 210);
pub const SOLARIZED_LIGHT_CYAN: Color = Color::Rgb(42, 161, 152);
pub const SOLARIZED_LIGHT_GREEN: Color = Color::Rgb(133, 153, 0);

// Rose Pine Palette
pub const ROSEPINE_BASE: Color = Color::Rgb(19, 17, 24);
pub const ROSEPINE_SURFACE: Color = Color::Rgb(33, 30, 45);
pub const ROSEPINE_OVERLAY: Color = Color::Rgb(49, 46, 64);
pub const ROSEPINE_MUTED: Color = Color::Rgb(65, 60, 81);
pub const ROSEPINE_TEXT: Color = Color::Rgb(220, 215, 223);
pub const ROSEPINE_SUBTEXT: Color = Color::Rgb(176, 170, 183);
pub const ROSEPINE_PINE: Color = Color::Rgb(105, 143, 119);
pub const ROSEPINE_FOAM: Color = Color::Rgb(132, 197, 182);
pub const ROSEPINE_IRIS: Color = Color::Rgb(176, 156, 222);
pub const ROSEPINE_GOLD: Color = Color::Rgb(210, 182, 111);
pub const ROSEPINE_ROSE: Color = Color::Rgb(225, 135, 159);
pub const ROSEPINE_LOVE: Color = Color::Rgb(232, 124, 143);
pub const ROSEPINE_GOLD_RGB: Color = Color::Rgb(204, 172, 103);

// One Dark Palette
pub const ONEDARK_BG: Color = Color::Rgb(40, 44, 52);
pub const ONEDARK_FG: Color = Color::Rgb(171, 178, 191);
pub const ONEDARK_RED: Color = Color::Rgb(224, 108, 117);
pub const ONEDARK_GREEN: Color = Color::Rgb(152, 195, 121);
pub const ONEDARK_YELLOW: Color = Color::Rgb(229, 192, 117);
pub const ONEDARK_BLUE: Color = Color::Rgb(97, 175, 239);
pub const ONEDARK_PURPLE: Color = Color::Rgb(198, 120, 221);
pub const ONEDARK_CYAN: Color = Color::Rgb(86, 182, 194);
pub const ONEDARK_GREY: Color = Color::Rgb(86, 95, 108);
pub const ONEDARK_LIGHT_GREY: Color = Color::Rgb(127, 137, 152);
pub const ONEDARK_DARKER_GREY: Color = Color::Rgb(62, 69, 82);
pub const ONEDARK_LINE_GREY: Color = Color::Rgb(59, 65, 77);

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

    pub fn frappe() -> Self {
        // Catppuccin Frappe: muted medium-dark, soft pastel accents.
        Self {
            name: "frappe",
            accent: FRAPPE_MAUVE,
            accent_dim: FRAPPE_ROSE,
            border: FRAPPE_SURFACE1,
            border_focused: FRAPPE_MAUVE,
            text: FRAPPE_TEXT,
            muted: FRAPPE_SUBTEXT0,
            faint: FRAPPE_OVERLAY0,
            success: FRAPPE_GREEN,
            warning: FRAPPE_YELLOW,
            danger: FRAPPE_RED,
            info: FRAPPE_BLUE,
            hint: FRAPPE_PEACH,
            ink: FRAPPE_BASE,
            selected_bg: FRAPPE_SURFACE0,
            selected_fg: FRAPPE_MAUVE,
            progress_fill: FRAPPE_MAUVE,
            progress_track: FRAPPE_SURFACE0,
        }
    }

    pub fn macchiato() -> Self {
        // Catppuccin Macchiato: light-medium with soft rose accents.
        Self {
            name: "macchiato",
            accent: MACCHIATO_MAUVE,
            accent_dim: MACCHIATO_ROSE,
            border: MACCHIATO_SURFACE1,
            border_focused: MACCHIATO_MAUVE,
            text: MACCHIATO_TEXT,
            muted: MACCHIATO_SUBTEXT0,
            faint: MACCHIATO_OVERLAY0,
            success: MACCHIATO_GREEN,
            warning: MACCHIATO_YELLOW,
            danger: MACCHIATO_RED,
            info: MACCHIATO_BLUE,
            hint: MACCHIATO_PEACH,
            ink: MACCHIATO_BASE,
            selected_bg: MACCHIATO_SURFACE0,
            selected_fg: MACCHIATO_MAUVE,
            progress_fill: MACCHIATO_MAUVE,
            progress_track: MACCHIATO_SURFACE0,
        }
    }

    pub fn solarized_dark() -> Self {
        // Solarized Dark: low-contrast, blue-gray base with cyan accent.
        Self {
            name: "solarized-dark",
            accent: SOLARIZED_CYAN,
            accent_dim: SOLARIZED_BASE0,
            border: SOLARIZED_BASE02,
            border_focused: SOLARIZED_CYAN,
            text: SOLARIZED_BASE2,
            muted: SOLARIZED_BASE01,
            faint: SOLARIZED_BASE02,
            success: SOLARIZED_GREEN,
            warning: SOLARIZED_YELLOW,
            danger: SOLARIZED_RED,
            info: SOLARIZED_BLUE,
            hint: SOLARIZED_MAGENTA,
            ink: SOLARIZED_BASE03,
            selected_bg: SOLARIZED_BASE01,
            selected_fg: SOLARIZED_CYAN,
            progress_fill: SOLARIZED_CYAN,
            progress_track: SOLARIZED_BASE01,
        }
    }

    pub fn solarized_light() -> Self {
        // Solarized Light: inverted dark, paper background with cyan accent.
        Self {
            name: "solarized-light",
            accent: SOLARIZED_LIGHT_VIOLET,
            accent_dim: SOLARIZED_BASE1,
            border: SOLARIZED_BASE1,
            border_focused: SOLARIZED_LIGHT_VIOLET,
            text: SOLARIZED_BASE03,
            muted: SOLARIZED_BASE00,
            faint: SOLARIZED_BASE1,
            success: SOLARIZED_LIGHT_GREEN,
            warning: SOLARIZED_LIGHT_YELLOW,
            danger: SOLARIZED_LIGHT_RED,
            info: SOLARIZED_LIGHT_BLUE,
            hint: SOLARIZED_LIGHT_MAGENTA,
            ink: SOLARIZED_BASE3,
            selected_bg: SOLARIZED_BASE00,
            selected_fg: SOLARIZED_LIGHT_VIOLET,
            progress_fill: SOLARIZED_LIGHT_VIOLET,
            progress_track: SOLARIZED_BASE00,
        }
    }

    pub fn rosepine() -> Self {
        // Rose Pine: warm dark pink/purple palette, very cozy.
        Self {
            name: "rosepine",
            accent: ROSEPINE_IRIS,
            accent_dim: ROSEPINE_ROSE,
            border: ROSEPINE_SURFACE,
            border_focused: ROSEPINE_IRIS,
            text: ROSEPINE_TEXT,
            muted: ROSEPINE_SUBTEXT,
            faint: ROSEPINE_MUTED,
            success: ROSEPINE_PINE,
            warning: ROSEPINE_GOLD,
            danger: ROSEPINE_LOVE,
            info: ROSEPINE_FOAM,
            hint: ROSEPINE_ROSE,
            ink: ROSEPINE_BASE,
            selected_bg: ROSEPINE_SURFACE,
            selected_fg: ROSEPINE_IRIS,
            progress_fill: ROSEPINE_IRIS,
            progress_track: ROSEPINE_SURFACE,
        }
    }

    pub fn onedark() -> Self {
        // One Dark: Atom's classic dark theme, blue-gray accents.
        Self {
            name: "onedark",
            accent: ONEDARK_BLUE,
            accent_dim: ONEDARK_PURPLE,
            border: ONEDARK_DARKER_GREY,
            border_focused: ONEDARK_BLUE,
            text: ONEDARK_FG,
            muted: ONEDARK_GREY,
            faint: ONEDARK_DARKER_GREY,
            success: ONEDARK_GREEN,
            warning: ONEDARK_YELLOW,
            danger: ONEDARK_RED,
            info: ONEDARK_CYAN,
            hint: ONEDARK_PURPLE,
            ink: ONEDARK_BG,
            selected_bg: ONEDARK_DARKER_GREY,
            selected_fg: ONEDARK_BLUE,
            progress_fill: ONEDARK_BLUE,
            progress_track: ONEDARK_DARKER_GREY,
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
            "frappe" | "catppuccin-frappe" => Self::frappe(),
            "macchiato" | "catppuccin-macchiato" => Self::macchiato(),
            "solarized-dark" | "solarizeddark" => Self::solarized_dark(),
            "solarized-light" | "solarizedlight" => Self::solarized_light(),
            "rosepine" | "rose-pine" => Self::rosepine(),
            "onedark" | "one-dark" => Self::onedark(),
            _ => Self::mocha(),
        }
    }

    pub fn available() -> Vec<&'static str> {
        vec![
            "mocha",
            "gruvbox",
            "tokyo-night",
            "dracula",
            "nord",
            "latte",
            "frappe",
            "macchiato",
            "solarized-dark",
            "solarized-light",
            "rosepine",
            "onedark",
        ]
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
        assert_eq!(Theme::from_name("catppuccin-frappe").name, "frappe");
        assert_eq!(Theme::from_name("catppuccin-macchiato").name, "macchiato");
        assert_eq!(Theme::from_name("solarized-dark").name, "solarized-dark");
        assert_eq!(Theme::from_name("solarized-light").name, "solarized-light");
        assert_eq!(Theme::from_name("rosepine").name, "rosepine");
        assert_eq!(Theme::from_name("onedark").name, "onedark");
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
            Theme::frappe(),
            Theme::macchiato(),
            Theme::solarized_dark(),
            Theme::solarized_light(),
            Theme::rosepine(),
            Theme::onedark(),
        ]
        .iter()
        .map(|t| t.accent)
        .collect();
        let mut unique = accents.clone();
        unique.sort_by_key(|c| format!("{c:?}"));
        unique.dedup();
        // Every preset must have a visually distinct signature accent.
        assert_eq!(unique.len(), accents.len());
        assert_eq!(Theme::available().len(), 12);
    }
}
