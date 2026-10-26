use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};
use crate::app::App;

/// Visualizer render styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VizStyle {
    Bars,      // Cava Equalizer spectrum (dynamic bass/mids/treble bands)
    Wave,      // Smooth continuous oscilloscope sound wave
    Dual,      // Stereo center-out mirrored cava
    Spectrum,  // Dynamic studio peak-meter spectrum
    Dots,      // Lofi ambient floating particles
}

impl Default for VizStyle {
    fn default() -> Self {
        Self::Bars
    }
}

impl VizStyle {
    pub fn from_name(name: &str) -> Self {
        match name {
            "wave" => Self::Wave,
            "dual" => Self::Dual,
            "spectrum" => Self::Spectrum,
            "dots" => Self::Dots,
            _ => Self::Bars,
        }
    }

    #[allow(dead_code)]
    pub fn to_name(self) -> &'static str {
        match self {
            Self::Bars => "bars",
            Self::Wave => "wave",
            Self::Dual => "dual",
            Self::Spectrum => "spectrum",
            Self::Dots => "dots",
        }
    }
}

/// Visualizer color preset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VizColor {
    Accent,
    Cyan,
    Green,
    Magenta,
    Yellow,
    Rainbow,
}

impl Default for VizColor {
    fn default() -> Self {
        Self::Accent
    }
}

impl VizColor {
    pub fn from_name(name: &str) -> Self {
        match name {
            "cyan" => Self::Cyan,
            "green" => Self::Green,
            "magenta" => Self::Magenta,
            "yellow" => Self::Yellow,
            "rainbow" | "gradient" => Self::Rainbow,
            _ => Self::Accent,
        }
    }

    #[allow(dead_code)]
    pub fn to_name(self) -> &'static str {
        match self {
            Self::Accent => "accent",
            Self::Cyan => "cyan",
            Self::Green => "green",
            Self::Magenta => "magenta",
            Self::Yellow => "yellow",
            Self::Rainbow => "rainbow",
        }
    }
}

/// Smooth 8-level vertical block ladder for sub-cell resolution.
const VIZ_PARTIALS: [&str; 8] = [" ", "▂", "▃", "▄", "▅", "▆", "▇", "█"];

/// Returns the color style for a cell based on preset, row height, and column position.
fn cell_style(
    color_preset: VizColor,
    style: VizStyle,
    col_idx: usize,
    bar_count: usize,
    row: usize,
    draw_h: usize,
    is_peak: bool,
    theme: &crate::theme::Theme,
) -> Style {
    let y_ratio = row as f32 / (draw_h.saturating_sub(1).max(1) as f32);

    let (base_color, peak_boost) = match color_preset {
        VizColor::Rainbow => {
            let x = col_idx as f32 / (bar_count.saturating_sub(1).max(1) as f32);
            let (r, g, b) = if x < 0.20 {
                // Purple to Royal Blue
                let f = x / 0.20;
                (
                    (150.0 * (1.0 - f) + 40.0 * f) as u8,
                    (50.0 * (1.0 - f) + 120.0 * f) as u8,
                    (240.0 * (1.0 - f) + 255.0 * f) as u8,
                )
            } else if x < 0.45 {
                // Blue to Cyan
                let f = (x - 0.20) / 0.25;
                (
                    (40.0 * (1.0 - f) + 0.0 * f) as u8,
                    (120.0 * (1.0 - f) + 230.0 * f) as u8,
                    (255.0 * (1.0 - f) + 240.0 * f) as u8,
                )
            } else if x < 0.70 {
                // Cyan to Lime Green
                let f = (x - 0.45) / 0.25;
                (
                    (0.0 * (1.0 - f) + 90.0 * f) as u8,
                    (230.0 * (1.0 - f) + 245.0 * f) as u8,
                    (240.0 * (1.0 - f) + 70.0 * f) as u8,
                )
            } else if x < 0.88 {
                // Lime to Amber
                let f = (x - 0.70) / 0.18;
                (
                    (90.0 * (1.0 - f) + 255.0 * f) as u8,
                    (245.0 * (1.0 - f) + 180.0 * f) as u8,
                    (70.0 * (1.0 - f) + 30.0 * f) as u8,
                )
            } else {
                // Amber to Coral Red
                let f = (x - 0.88) / 0.12;
                (
                    255,
                    (180.0 * (1.0 - f) + 70.0 * f) as u8,
                    (30.0 * (1.0 - f) + 70.0 * f) as u8,
                )
            };
            (Color::Rgb(r, g, b), true)
        }
        VizColor::Cyan => {
            let color = if y_ratio >= 0.75 {
                Color::Rgb(215, 255, 255)
            } else if y_ratio >= 0.35 {
                Color::Rgb(0, 225, 255)
            } else {
                Color::Rgb(0, 130, 165)
            };
            (color, false)
        }
        VizColor::Green => {
            let color = if y_ratio >= 0.75 {
                Color::Rgb(225, 255, 120)
            } else if y_ratio >= 0.35 {
                Color::Rgb(60, 225, 90)
            } else {
                Color::Rgb(25, 130, 55)
            };
            (color, false)
        }
        VizColor::Magenta => {
            let color = if y_ratio >= 0.75 {
                Color::Rgb(255, 195, 245)
            } else if y_ratio >= 0.35 {
                Color::Rgb(240, 60, 210)
            } else {
                Color::Rgb(135, 25, 115)
            };
            (color, false)
        }
        VizColor::Yellow => {
            let color = if y_ratio >= 0.75 {
                Color::Rgb(255, 250, 160)
            } else if y_ratio >= 0.35 {
                Color::Rgb(255, 195, 20)
            } else {
                Color::Rgb(165, 105, 0)
            };
            (color, false)
        }
        VizColor::Accent => {
            let color = if y_ratio >= 0.75 {
                theme.warning
            } else if y_ratio >= 0.35 {
                theme.accent
            } else {
                theme.accent_dim
            };
            (color, false)
        }
    };

    if style == VizStyle::Dots {
        if is_peak {
            Style::default().fg(base_color).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.faint)
        }
    } else if is_peak || peak_boost {
        Style::default().fg(base_color).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(base_color)
    }
}

/// Generate dynamic bar heights in 1/8th units based on style, phase, and state.
fn viz_heights(style: VizStyle, phase: f32, playing: bool, bar_count: usize, max_height: u16) -> Vec<u16> {
    let mut heights = vec![0u16; bar_count];
    let total_subunits = max_height.max(1) * 8;
    let n = bar_count.max(1) as f32;

    if !playing {
        // Soft ambient breathing wave when paused
        for i in 0..bar_count {
            let t = i as f32 / n;
            let center_env = (1.0 - (t - 0.5).abs() * 1.4).max(0.2);
            let wave = ((t * 4.5 + phase * 0.6).sin() * 0.10 + 0.12) * center_env;
            let h = (wave * total_subunits as f32).max(1.0).min(total_subunits as f32);
            heights[i] = h as u16;
        }
        return heights;
    }

    match style {
        VizStyle::Bars => {
            // Realistic Cava Equalizer:
            // Bass on left (heavy rhythmic punch), mids in center, sparkling highs on right.
            let kick = (phase * 2.2).sin().max(0.0).powf(2.2); // Rhythmic 4/4 bass beat
            for i in 0..bar_count {
                let x = i as f32 / n;
                // Bass kick wave
                let bass_wave = (phase * 1.1 + (i as f32) * 0.28).sin() * 0.32 + kick * 0.40;
                // Mid musical harmonics
                let mid_wave = (phase * 2.4 + (i as f32) * 0.42).sin() * 0.32
                    + (phase * 3.8 - (i as f32) * 0.65).cos() * 0.18;
                // High sparkle jitter
                let high_wave = (phase * 5.2 + (i as f32) * 1.1).sin() * 0.24
                    + (phase * 8.4 + (i as f32) * 1.8).cos() * 0.14;

                let h_norm = if x < 0.28 {
                    // Bass zone: punchy low-end
                    0.25 + bass_wave * 0.65 + mid_wave * 0.15
                } else if x < 0.72 {
                    // Mid zone: vocal & rhythm
                    0.20 + mid_wave * 0.55 + high_wave * 0.20 + kick * 0.15
                } else {
                    // Treble zone: hi-hats & air
                    0.15 + high_wave * 0.60 + mid_wave * 0.15
                };

                let h = (h_norm.clamp(0.06, 0.96) * total_subunits as f32) as u16;
                heights[i] = h.max(1);
            }
        }
        VizStyle::Wave => {
            // Continuous analog oscilloscope sound wave
            for i in 0..bar_count {
                let t = (i as f32 / n) * std::f32::consts::PI * 2.0;
                let w1 = (t * 2.0 - phase * 1.6).sin() * 0.38;
                let w2 = (t * 4.0 + phase * 0.9).cos() * 0.16;
                let w3 = (t * 1.0 - phase * 2.4).sin() * 0.12;
                let h_norm = (0.50 + w1 + w2 + w3).clamp(0.06, 0.96);
                heights[i] = ((h_norm * total_subunits as f32) as u16).max(1);
            }
        }
        VizStyle::Dual => {
            // Stereo center-out mirrored cava
            let mid = bar_count / 2;
            let kick = (phase * 2.2).sin().max(0.0).powf(2.0);
            for i in 0..=mid {
                let dist_from_center = (mid.saturating_sub(i)) as f32 / mid.max(1) as f32;
                let t = i as f32 * 0.35 + phase * 1.8;
                let pulse = (t.sin() * 0.45 + (t * 2.2).cos() * 0.25).abs();
                let center_boost = (1.0 - dist_from_center * 0.75) * kick * 0.45;
                let h_norm = (0.12 + pulse * 0.45 + center_boost).clamp(0.06, 0.96);
                let val = ((h_norm * total_subunits as f32) as u16).max(1);

                heights[i] = val;
                let mirror = bar_count.saturating_sub(1).saturating_sub(i);
                if mirror < bar_count {
                    heights[mirror] = val;
                }
            }
        }
        VizStyle::Spectrum => {
            // High dynamic range studio peak meter
            for i in 0..bar_count {
                let t = i as f32 * 0.48 + phase * 2.0;
                let raw = ((t.sin() * 0.45 + (t * 1.7).cos() * 0.30 + (t * 4.5).sin() * 0.25 + 1.0) / 2.0)
                    .powf(1.35);
                let h = (raw.clamp(0.06, 0.98) * total_subunits as f32) as u16;
                heights[i] = h.max(1);
            }
        }
        VizStyle::Dots => {
            // Minimalist floating particles
            for i in 0..bar_count {
                let t = i as f32 * 0.38 + phase * 1.6;
                let raw = (t.sin() * 0.50 + (t * 2.8).cos().abs() * 0.35 + 0.15).clamp(0.08, 0.95);
                let h = (raw * total_subunits as f32) as u16;
                heights[i] = h.max(1);
            }
        }
    }

    heights
}

/// Render the audio visualizer inside `area`.
pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    if area.height < 3 || area.width < 10 {
        return;
    }

    let style = VizStyle::from_name(
        app.visualizer_style
            .as_deref()
            .unwrap_or("bars"),
    );
    let color_preset = VizColor::from_name(
        app.visualizer_color
            .as_deref()
            .unwrap_or("accent"),
    );

    let draw_h = (area.height as usize).saturating_sub(2).max(1); // Rows available inside borders
    let columns = (area.width as usize).saturating_sub(2);        // Columns available inside borders

    // Adaptive bar sizing: wider bars on large screens, dense bars on compact screens
    let (bar_width, bar_gap) = if columns >= 75 {
        (2, 1)
    } else {
        (1, 1)
    };

    let unit = bar_width + bar_gap;
    let bar_count = ((columns + bar_gap) / unit).clamp(6, 128);
    let used_width = bar_count * bar_width + bar_count.saturating_sub(1) * bar_gap;
    let left_pad = columns.saturating_sub(used_width) / 2;

    let heights = viz_heights(
        style,
        app.visualizer_phase,
        !app.audio.is_paused,
        bar_count,
        draw_h as u16,
    );

    let mut lines: Vec<Line> = Vec::with_capacity(draw_h);

    // Draw bars from top row down to bottom row (row 0 = bottom row of area)
    for row in (0..draw_h).rev() {
        let mut spans = Vec::with_capacity(columns);

        // Left centering padding
        if left_pad > 0 {
            spans.push(Span::raw(" ".repeat(left_pad)));
        }

        for (col_idx, &height) in heights.iter().enumerate() {
            let row_floor = row as u16 * 8;
            let row_ceil = row_floor + 8;

            let (glyph, is_peak) = if style == VizStyle::Dots {
                if height >= row_floor && height < row_ceil + 4 {
                    ("●", true)
                } else if height > row_ceil && (row % 2 == 0) {
                    ("·", false)
                } else {
                    (" ", false)
                }
            } else if height >= row_ceil {
                ("█", height < row_ceil + 8)
            } else if height > row_floor {
                let frac = (height - row_floor - 1).min(7) as usize;
                (VIZ_PARTIALS[frac], true)
            } else {
                (" ", false)
            };

            let c_style = cell_style(
                color_preset,
                style,
                col_idx,
                bar_count,
                row,
                draw_h,
                is_peak,
                &app.theme,
            );

            for _ in 0..bar_width {
                spans.push(Span::styled(glyph, c_style));
            }

            if col_idx + 1 < bar_count && bar_gap > 0 {
                spans.push(Span::raw(" ".repeat(bar_gap)));
            }
        }

        lines.push(Line::from(spans));
    }

    // Header bar with status and controls
    let (status_icon, status_label, status_color) = if app.audio.is_paused {
        ("󰏤 ", "paused", app.theme.warning)
    } else {
        ("● ", "live", app.theme.success)
    };

    let title_style_color = match color_preset {
        VizColor::Rainbow => app.theme.accent,
        VizColor::Cyan => Color::Rgb(0, 225, 255),
        VizColor::Green => Color::Rgb(60, 225, 90),
        VizColor::Magenta => Color::Rgb(240, 60, 210),
        VizColor::Yellow => Color::Rgb(255, 195, 20),
        VizColor::Accent => app.theme.accent,
    };

    let left_title = Line::from(vec![
        Span::styled(" 󰕒 Visualizer ", Style::default().fg(app.theme.accent).add_modifier(Modifier::BOLD)),
        Span::styled("│ ", Style::default().fg(app.theme.border)),
        Span::styled(format!("{} ", style.to_name()), Style::default().fg(title_style_color).add_modifier(Modifier::BOLD)),
        Span::styled(status_icon, Style::default().fg(status_color)),
        Span::styled(format!("{status_label} "), Style::default().fg(status_color)),
    ]);

    let right_title = Line::from(vec![
        Span::styled("[b: Toggle │ ^B: Style │ ^V: Color] ", Style::default().fg(app.theme.faint)),
    ]);

    let block = Block::default()
        .title(left_title)
        .title(right_title.alignment(Alignment::Right))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(app.theme.border));

    frame.render_widget(Paragraph::new(lines).block(block), area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_viz_style_parsing_and_names() {
        assert_eq!(VizStyle::from_name("bars"), VizStyle::Bars);
        assert_eq!(VizStyle::from_name("wave"), VizStyle::Wave);
        assert_eq!(VizStyle::from_name("dual"), VizStyle::Dual);
        assert_eq!(VizStyle::from_name("spectrum"), VizStyle::Spectrum);
        assert_eq!(VizStyle::from_name("dots"), VizStyle::Dots);
        assert_eq!(VizStyle::from_name("unknown"), VizStyle::Bars);

        assert_eq!(VizStyle::Bars.to_name(), "bars");
        assert_eq!(VizStyle::Wave.to_name(), "wave");
        assert_eq!(VizStyle::Dual.to_name(), "dual");
        assert_eq!(VizStyle::Spectrum.to_name(), "spectrum");
        assert_eq!(VizStyle::Dots.to_name(), "dots");
    }

    #[test]
    fn test_viz_color_parsing_and_names() {
        assert_eq!(VizColor::from_name("accent"), VizColor::Accent);
        assert_eq!(VizColor::from_name("cyan"), VizColor::Cyan);
        assert_eq!(VizColor::from_name("green"), VizColor::Green);
        assert_eq!(VizColor::from_name("magenta"), VizColor::Magenta);
        assert_eq!(VizColor::from_name("yellow"), VizColor::Yellow);
        assert_eq!(VizColor::from_name("rainbow"), VizColor::Rainbow);
        assert_eq!(VizColor::from_name("gradient"), VizColor::Rainbow);
        assert_eq!(VizColor::from_name("invalid"), VizColor::Accent);

        assert_eq!(VizColor::Accent.to_name(), "accent");
        assert_eq!(VizColor::Rainbow.to_name(), "rainbow");
    }

    #[test]
    fn test_viz_heights_bounds_all_styles() {
        let styles = [
            VizStyle::Bars,
            VizStyle::Wave,
            VizStyle::Dual,
            VizStyle::Spectrum,
            VizStyle::Dots,
        ];
        let max_h = 6u16;
        let total_subunits = max_h * 8;

        for style in styles {
            // Test playing state
            let heights_play = viz_heights(style, 1.25, true, 24, max_h);
            assert_eq!(heights_play.len(), 24);
            for &h in &heights_play {
                assert!(h >= 1, "{:?} produced 0 height", style);
                assert!(h <= total_subunits, "{:?} exceeded max height", style);
            }

            // Test paused state
            let heights_pause = viz_heights(style, 0.5, false, 24, max_h);
            assert_eq!(heights_pause.len(), 24);
            for &h in &heights_pause {
                assert!(h >= 1);
                assert!(h <= total_subunits);
            }
        }
    }

    #[test]
    fn test_dual_style_symmetry() {
        let bar_count = 20;
        let heights = viz_heights(VizStyle::Dual, 2.5, true, bar_count, 6);
        for i in 0..(bar_count / 2) {
            let mirror = bar_count - 1 - i;
            assert_eq!(
                heights[i], heights[mirror],
                "Dual style must be symmetrical at index {} vs {}",
                i, mirror
            );
        }
    }
}
