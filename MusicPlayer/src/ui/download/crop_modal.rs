use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};
use crate::{app::App, theme};

/// Interactive crop editor with a live WYSIWYG result preview: every
/// frame move re-crops the source to the 1:1 window at the focal point
/// (same math as the final `center_crop`, so what you see is what the
/// download embeds). Real pixels via kitty, halfblock fallback otherwise.
pub fn render_crop_modal(frame: &mut Frame, app: &mut App) {
    let screen = frame.area();
    let popup_w = 72.min(screen.width.saturating_sub(4));
    let popup_h = 26.min(screen.height.saturating_sub(2));

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
        .title(format!(" 󰣵 Crop Preview ({fx:.2}, {fy:.2}) "))
        .title_alignment(Alignment::Center)
        .title_bottom(Line::from(vec![Span::styled(
            " [←/→/↑/↓: Move 5% | Shift: Fine 1% | Enter: Apply | c: Center | Esc: Cancel] ",
            Style::default().fg(theme::OVERLAY0),
        )]))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::PEACH).add_modifier(Modifier::BOLD));
    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(6),   // Live result preview canvas
            Constraint::Length(3), // Source info + saved focus
        ])
        .split(inner);

    // Square canvas for the 1:1 result.
    let max_side = rows[0].width.min(rows[0].height.saturating_mul(2)).min(40);
    let side = max_side.max(6);
    let canvas = ratatui::layout::Rect::new(
        rows[0].x + rows[0].width.saturating_sub(side) / 2,
        rows[0].y + rows[0].height.saturating_sub(side / 2) / 2,
        side,
        (side / 2).max(3),
    );

    if let Some(art) = &app.download.crop_preview {
        if crate::cover::is_kitty_supported() {
            let buf = frame.buffer_mut();
            for y in canvas.y..canvas.bottom() {
                for x in canvas.x..canvas.right() {
                    if let Some(cell) = buf.cell_mut((x, y)) {
                        cell.set_skip(true);
                    }
                }
            }
            app.kitty_cover_rect = Some(canvas);
        } else {
            let preview_lines = art.render_halfblocks(canvas.width, canvas.height);
            frame.render_widget(Paragraph::new(preview_lines).alignment(Alignment::Center), canvas);
            app.kitty_cover_rect = None;
        }
    } else {
        app.kitty_cover_rect = None;
        frame.render_widget(
            Paragraph::new("[ No Preview — image unreadable ]").alignment(Alignment::Center),
            rows[0],
        );
    }

    let mut info = vec![Line::from(vec![Span::styled(
        match app.download.crop_img_size {
            Some((w, h)) => format!(" Source: {w}x{h}px  •  1:1 result above is exactly what gets embedded "),
            None => " Source: unknown ".to_string(),
        },
        Style::default().fg(theme::OVERLAY0),
    )])];
    if let Some((cfx, cfy)) = app.download.cover_focus {
        info.push(Line::from(vec![Span::styled(
            format!(" Saved focus: ({cfx:.2}, {cfy:.2}) — Enter overwrites "),
            Style::default().fg(theme::OVERLAY0),
        )]));
    } else {
        info.push(Line::from(vec![Span::styled(
            " No saved focus yet — Enter applies the frame above ".to_string(),
            Style::default().fg(theme::OVERLAY0),
        )]));
    }
    frame.render_widget(Paragraph::new(info).alignment(Alignment::Center), rows[1]);
}
