use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};
use crate::{app::App, theme};

/// Interactive crop-focus editor: the source image is drawn as a box
/// preserving its aspect ratio, with the fixed 1:1 crop frame (`█`)
/// positioned by the focal point (arrows move it in 5% steps).
pub fn render_crop_modal(frame: &mut Frame, app: &App) {
    let screen = frame.area();
    let popup_w = 62.min(screen.width.saturating_sub(4));
    let popup_h = 24.min(screen.height.saturating_sub(2));

    let v = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((screen.height.saturating_sub(popup_h)) / 2),
            Constraint::Length(popup_h),
            Constraint::Min(0),
        ])
        .split(screen);
    let popup_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length((screen.width.saturating_sub(popup_w)) / 2),
            Constraint::Length(popup_w),
            Constraint::Min(0),
        ])
        .split(v[1])[1];

    frame.render_widget(Clear, popup_area);

    let (fx, fy) = app.download.crop_cursor;
    let block = Block::default()
        .title(format!(" 󰣵 Crop Focus ({fx:.2}, {fy:.2}) "))
        .title_alignment(Alignment::Center)
        .title_bottom(Line::from(vec![Span::styled(
            " [←/→/↑/↓: Move Frame | Enter: Apply | c: Center | Esc: Cancel] ",
            Style::default().fg(theme::OVERLAY0),
        )]))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::PEACH).add_modifier(Modifier::BOLD));
    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let Some((w, h)) = app.download.crop_img_size else {
        frame.render_widget(
            Paragraph::new("No image loaded").alignment(Alignment::Center),
            inner,
        );
        return;
    };

    // Box size preserving aspect (terminal cells are ~2:1 tall).
    let box_w = (inner.width.saturating_sub(4).min(52)).max(10) as f32;
    let aspect = h as f32 / w as f32;
    let box_h = ((box_w * aspect / 2.0).round() as u16).clamp(4, inner.height.saturating_sub(5).max(4));

    // Crop window: square of side min(w,h), offset by the focal point.
    let side = w.min(h) as f32;
    let scale_x = box_w / w as f32;
    let scale_y = box_h as f32 / h as f32;
    let win_w = (side * scale_x).round().max(1.0) as usize;
    let win_h = (side * scale_y).round().max(1.0) as usize;
    let max_left = (box_w as usize).saturating_sub(win_w);
    let max_top = (box_h as usize).saturating_sub(win_h);
    let win_left = ((max_left as f32) * fx).round() as usize;
    let win_top = ((max_top as f32) * fy).round() as usize;

    let mut lines = Vec::new();
    lines.push(Line::from(vec![Span::styled(
        format!(" Source: {w}x{h}px  •  applying to center_crop "),
        Style::default().fg(theme::OVERLAY0),
    )]));
    lines.push(Line::raw(""));
    for row in 0..box_h as usize {
        let mut row_str = String::from("  ");
        for col in 0..box_w as usize {
            let in_x = col >= win_left && col < win_left + win_w;
            let in_y = row >= win_top && row < win_top + win_h;
            let edge = (col == win_left || col + 1 == win_left + win_w) && in_y
                || (row == win_top || row + 1 == win_top + win_h) && in_x;
            row_str.push(if edge {
                '█'
            } else if in_x && in_y {
                '▓'
            } else {
                '░'
            });
        }
        let style = Style::default().fg(theme::BLUE);
        lines.push(Line::from(Span::styled(row_str, style)));
    }
    if let Some((cfx, cfy)) = app.download.cover_focus {
        lines.push(Line::raw(""));
        lines.push(Line::from(vec![Span::styled(
            format!(" Saved focus: ({cfx:.2}, {cfy:.2}) — Enter overwrites "),
            Style::default().fg(theme::OVERLAY0),
        )]));
    }

    frame.render_widget(Paragraph::new(lines).alignment(Alignment::Center), inner);
}
