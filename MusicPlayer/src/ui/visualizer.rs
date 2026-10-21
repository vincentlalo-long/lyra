use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};
use crate::app::App;

/// Visualizer render styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VizStyle {
    Bars,      // Cột nhấp nhô (equalizer bars)
    Wave,      // Sóng sine connects đều
    Dual,      // Sóng đối xứng 2 bên từ tâm
    Spectrum,  // Equalizer spectrum (cao-thấp ngẫu nhiên)
    Dots,      // Điểm nhấp nhô rải đều
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
}

impl Default for VizColor {
    fn default() -> Self {
        Self::Accent
    }
}

impl VizColor {
    pub fn from_name(name: &str, theme: &crate::theme::Theme) -> Color {
        match name {
            "cyan" => Color::Rgb(0, 255, 255),
            "green" => Color::Rgb(0, 255, 0),
            "magenta" => Color::Rgb(255, 0, 255),
            "yellow" => Color::Rgb(255, 255, 0),
            _ => theme.accent,
        }
    }
}

/// Number of bars/columns in the visualizer.
const VIZ_BARS: usize = 26;

/// Generate bar heights for the given style and phase.
fn viz_heights(style: VizStyle, phase: f32, playing: bool) -> Vec<u16> {
    let mut heights = vec![0u16; VIZ_BARS];

    if !playing {
        // When paused: show small static heartbeat.
        for i in 0..VIZ_BARS {
            let t = i as f32 / VIZ_BARS as f32;
            heights[i] = ((t * 5.0 + phase).sin() * 1.5 + 2.5).clamp(0.0, 4.0) as u16;
        }
        return heights;
    }

    match style {
        VizStyle::Bars => {
            // Bouncing bars with varied frequency per bar.
            for i in 0..VIZ_BARS {
                let t = i as f32 * 0.9 + phase * 2.5;
                let h = ((t.sin() * 0.4 + (t * 1.7).cos() * 0.6 + (t * 3.1).sin() * 0.3 + 1.5) / 3.0).clamp(0.0, 1.0);
                heights[i] = (h * 14.0) as u16 + 1;
            }
        }
        VizStyle::Wave => {
            // Smooth sine wave across bars.
            for i in 0..VIZ_BARS {
                let t = i as f32 * 0.45 - phase * 1.5;
                let h = (t.sin() * 0.6 + 0.4).clamp(0.0, 1.0);
                heights[i] = (h * 11.0) as u16 + 2;
            }
        }
        VizStyle::Dual => {
            // Mirrored from center (左右 đối xứng).
            let mid = VIZ_BARS / 2;
            for i in 0..=mid {
                let t = i as f32 * 0.85 + phase * 1.8;
                let h = ((t.sin() * 0.5 + (t * 2.1).cos() * 0.5 + 1.0) / 2.0).clamp(0.0, 1.0);
                let hv = (h * 11.0) as u16 + 2;
                heights[i] = hv;
                let mirror = VIZ_BARS - 1 - i;
                if mirror < VIZ_BARS && mirror != i {
                    heights[mirror] = hv;
                }
            }
        }
        VizStyle::Spectrum => {
            // Equalizer-like: random-ish heights per bar, smoother changes.
            for i in 0..VIZ_BARS {
                let t = i as f32 * 1.1 + phase;
                let h = ((t.sin() * 0.5 + (t * 1.3).cos() * 0.3 + (t * 4.2).sin() * 0.2 + 1.5) / 2.5).clamp(0.0, 1.0);
                heights[i] = (h * 13.0) as u16 + 1;
            }
        }
        VizStyle::Dots => {
            // Sparse dots that bounce independently.
            for i in 0..VIZ_BARS {
                let t = i as f32 * 1.3 + phase * 2.2;
                let h = ((t.sin() * 0.7 + 0.7) * (t * 2.9).cos().abs() + 0.3).clamp(0.0, 1.0);
                heights[i] = (h * 10.0) as u16 + 1;
            }
        }
    }

    heights
}

/// Render the audio visualizer inside `area`.
pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    if area.height < 3 || area.width < 8 {
        return;
    }

    let style = VizStyle::from_name(
        app.visualizer_style
            .as_deref()
            .unwrap_or("bars"),
    );
    let color = VizColor::from_name(
        app.visualizer_color.as_deref().unwrap_or(""),
        &app.theme,
    );

    let heights = viz_heights(style, app.visualizer_phase, !app.audio.is_paused);

    // Build bar lines from bottom up.
    let max_h = heights.iter().max().copied().unwrap_or(1).max(1) as usize;
    let avail_h = (area.height as usize).saturating_sub(2); // account for borders
    let draw_h = max_h.min(avail_h);

    let mut lines: Vec<Line> = Vec::with_capacity(draw_h + 2);

    // Draw bars from bottom (row 0 = bottom) to top.
    for row in (0..draw_h).rev() {
        let mut spans = Vec::with_capacity(VIZ_BARS * 2);
        for &h in &heights {
            if (h as usize) > row {
                spans.push(Span::styled("█", Style::default().fg(color).add_modifier(Modifier::BOLD)));
            } else {
                spans.push(Span::styled(" ", Style::default()));
            }
            spans.push(Span::styled(" ", Style::default())); // gap between bars
        }
        lines.push(Line::from(spans));
    }

    // Bottom label line showing style and state.
    let style_name = style.to_name();
    let state = if app.audio.is_paused { "paused" } else { "live" };
    let label = format!(" Visualizer: {} [{}] ", style_name, state);
    lines.push(Line::from(vec![
        Span::styled(label, Style::default().fg(app.theme.faint).add_modifier(Modifier::BOLD)),
    ]));

    let block = Block::default()
        .title(Span::styled(
            " 󰕒 Audio Visualizer ",
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(color));

    frame.render_widget(
        Paragraph::new(lines).block(block).style(Style::default().bg(app.theme.selected_bg)),
        area,
    );
}
