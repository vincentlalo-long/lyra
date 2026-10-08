use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};
use crate::{app::App, cover, theme};

pub fn render(frame: &mut Frame, app: &mut App, area: Rect) {
    let block = Block::default()
        .title(" 󰥠 Cover Art ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::BLUE));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.width < 4 || inner.height < 3 {
        app.kitty_cover_rect = None;
        return;
    }

    if let Some(cover_art) = &app.cover {
        if cover_art.image.is_some() {
            // Fill inner box edge-to-edge so no black empty borders waste space
            let cols = inner.width;
            let rows = inner.height;
            let target_rect = inner;

            if cover::is_kitty_supported() {
                // Mark cells as skipped so Ratatui does not overwrite the Kitty graphic
                let buf = frame.buffer_mut();
                for y in target_rect.y..target_rect.bottom() {
                    for x in target_rect.x..target_rect.right() {
                        if let Some(cell) = buf.cell_mut((x, y)) {
                            cell.set_skip(true);
                        }
                    }
                }
                app.kitty_cover_rect = Some(target_rect);
                return;
            } else {
                // Fallback for non-Kitty terminals: Unicode halfblock (▀) scaled edge-to-edge
                let lines = cover_art.render_halfblocks(cols, rows);
                frame.render_widget(Paragraph::new(lines), inner);
                app.kitty_cover_rect = None;
                return;
            }
        }
    }

    app.kitty_cover_rect = None;

    let pad_top = (inner.height.saturating_sub(2)) / 2;
    let mut placeholder_lines = Vec::new();
    for _ in 0..pad_top {
        placeholder_lines.push(Line::raw(""));
    }
    placeholder_lines.push(Line::from(Span::styled("󰝚", Style::default().fg(theme::OVERLAY0).add_modifier(Modifier::BOLD))));
    placeholder_lines.push(Line::from(Span::styled("No Cover Art", Style::default().fg(theme::SURFACE2))));

    let placeholder = Paragraph::new(placeholder_lines).alignment(Alignment::Center);
    frame.render_widget(placeholder, inner);
}
