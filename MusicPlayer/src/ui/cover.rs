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
        .title(" 󰥠 Album Art ")
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
            // 1:1 visual square in terminal cells: 1 cell height ~= 2 cell widths
            let max_rows = inner.height;
            let max_cols = inner.width;

            let mut rows = max_rows;
            let mut cols = rows * 2;
            if cols > max_cols {
                cols = max_cols;
                rows = (cols / 2).max(1);
            }

            if rows > 0 && cols > 0 {
                let pad_top = (inner.height.saturating_sub(rows)) / 2;
                let pad_left = (inner.width.saturating_sub(cols)) / 2;

                let target_rect = Rect::new(
                    inner.x + pad_left,
                    inner.y + pad_top,
                    cols,
                    rows,
                );

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
                    // Fallback for non-Kitty terminals: Unicode halfblock (▀).
                    // Paint a dimmed full-panel backdrop (stretched art) so the
                    // panel looks filled, then overlay the sharp square.
                    let mut lines = cover_art.render_halfblocks_dimmed(
                        inner.width,
                        inner.height,
                        0.35,
                    );
                    let fg_lines = cover_art.render_halfblocks(cols, rows);
                    for (i, fg) in fg_lines.into_iter().enumerate() {
                        if let Some(bg) = lines.get_mut(pad_top as usize + i) {
                            let mut spans = std::mem::take(&mut bg.spans);
                            let start = (pad_left as usize).min(spans.len());
                            let end = (start + cols as usize).min(spans.len());
                            spans.splice(start..end, fg.spans);
                            bg.spans = spans;
                        }
                    }
                    frame.render_widget(Paragraph::new(lines), inner);
                    app.kitty_cover_rect = None;
                    return;
                }
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
    placeholder_lines.push(Line::from(Span::styled("No Album Art", Style::default().fg(theme::SURFACE2))));

    let placeholder = Paragraph::new(placeholder_lines).alignment(Alignment::Center);
    frame.render_widget(placeholder, inner);
}
