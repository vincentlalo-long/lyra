use ratatui::{
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};
use crate::{app::App, theme};

/// Render the interactive Plugin Manager, Path Editor, and Plugin Store UI
pub fn render(frame: &mut Frame, app: &mut App, area: Rect) {
    let block = Block::default()
        .title(" 󰏖 Plugin Manager & Extension Store ")
        .title_bottom(Line::from(vec![
            Span::styled(" [↑/↓: Select | Space/Enter: Toggle | d: Remove | r: Restore | e: Edit Path | D: Store] ", Style::default().fg(theme::OVERLAY0)),
        ]))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::BLUE));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.width < 10 || inner.height < 4 {
        return;
    }

    let mut lines = Vec::new();
    let mut item_line_indices = Vec::new();

    // Section 1: Installed Plugins
    lines.push(Line::from(vec![
        Span::styled("  Installed Plugins", Style::default().fg(theme::MAUVE).add_modifier(Modifier::BOLD)),
        Span::styled(" — [Space/Enter: Toggle | d: Remove | r: Restore]", Style::default().fg(theme::OVERLAY0)),
    ]));
    lines.push(Line::raw(""));

    let n_plugins = app.plugins.len();
    for (idx, plugin) in app.plugins.iter().enumerate() {
        item_line_indices.push(lines.len());
        let is_sel = app.plugin_selected == idx;

        let (badge, badge_style) = if plugin.is_removed {
            (" 󰅖 Removed  ", Style::default().fg(theme::RED).bg(theme::SURFACE0).add_modifier(Modifier::CROSSED_OUT))
        } else if plugin.enabled {
            (" 󰄬 Enabled  ", Style::default().fg(theme::BASE).bg(theme::GREEN).add_modifier(Modifier::BOLD))
        } else {
            (" 󰅖 Disabled ", Style::default().fg(theme::TEXT).bg(theme::SURFACE1))
        };

        let action_hint = if is_sel {
            if plugin.is_removed {
                Span::styled(" ◀ r: Restore ▶ ", Style::default().fg(theme::GREEN).add_modifier(Modifier::BOLD))
            } else {
                Span::styled(" ◀ Space: Toggle | d: Remove ▶ ", Style::default().fg(theme::PEACH).add_modifier(Modifier::BOLD))
            }
        } else {
            Span::raw("")
        };

        if is_sel {
            lines.push(Line::from(vec![
                Span::styled(" ❯ ", Style::default().fg(theme::MAUVE).add_modifier(Modifier::BOLD)),
                Span::styled(format!("{:<28}", plugin.name), Style::default().fg(theme::YELLOW).add_modifier(Modifier::BOLD)),
                Span::styled(format!(" {:<8} ", plugin.version), Style::default().fg(theme::SUBTEXT0)),
                Span::styled(badge, badge_style),
                action_hint,
            ]));
            lines.push(Line::from(vec![
                Span::raw("     "),
                Span::styled(&plugin.description, Style::default().fg(theme::TEXT)),
            ]));
        } else {
            let name_style = if plugin.is_removed {
                Style::default().fg(theme::OVERLAY0).add_modifier(Modifier::CROSSED_OUT)
            } else {
                Style::default().fg(theme::TEXT)
            };
            lines.push(Line::from(vec![
                Span::raw("   "),
                Span::styled(format!("{:<28}", plugin.name), name_style),
                Span::styled(format!(" {:<8} ", plugin.version), Style::default().fg(theme::OVERLAY0)),
                Span::styled(badge, badge_style),
            ]));
            lines.push(Line::from(vec![
                Span::raw("     "),
                Span::styled(&plugin.description, Style::default().fg(theme::SUBTEXT0)),
            ]));
        }
        lines.push(Line::raw(""));
    }

    // Section 2: Configurable Paths
    lines.push(Line::from(vec![
        Span::styled("  Configurable Storage & Library Paths", Style::default().fg(theme::PEACH).add_modifier(Modifier::BOLD)),
        Span::styled(" — [Enter / e: Edit Path]", Style::default().fg(theme::OVERLAY0)),
    ]));
    lines.push(Line::raw(""));

    let n_paths = app.config_paths.len();
    for (idx, path_item) in app.config_paths.iter().enumerate() {
        let item_idx = n_plugins + idx;
        item_line_indices.push(lines.len());
        let is_sel = app.plugin_selected == item_idx;

        if is_sel {
            lines.push(Line::from(vec![
                Span::styled(" ❯ ", Style::default().fg(theme::MAUVE).add_modifier(Modifier::BOLD)),
                Span::styled(format!("{:<24}", path_item.label), Style::default().fg(theme::YELLOW).add_modifier(Modifier::BOLD)),
                Span::styled(&path_item.path, Style::default().fg(theme::BLUE).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)),
                Span::styled("  ◀ Enter/e: Edit ▶", Style::default().fg(theme::PEACH).add_modifier(Modifier::BOLD)),
            ]));
            lines.push(Line::from(vec![
                Span::raw("     "),
                Span::styled(&path_item.description, Style::default().fg(theme::TEXT)),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::raw("   "),
                Span::styled(format!("{:<24}", path_item.label), Style::default().fg(theme::SUBTEXT0)),
                Span::styled(&path_item.path, Style::default().fg(theme::TEXT)),
            ]));
            lines.push(Line::from(vec![
                Span::raw("     "),
                Span::styled(&path_item.description, Style::default().fg(theme::OVERLAY0)),
            ]));
        }
        lines.push(Line::raw(""));
    }

    // Section 3: Download Community Plugins Option
    let store_item_idx = n_plugins + n_paths;
    item_line_indices.push(lines.len());
    let is_sel_store = app.plugin_selected == store_item_idx;

    lines.push(Line::from(vec![
        Span::styled("  Download New Plugins", Style::default().fg(theme::GREEN).add_modifier(Modifier::BOLD)),
        Span::styled(" — [Community Repository Preview]", Style::default().fg(theme::OVERLAY0)),
    ]));
    lines.push(Line::raw(""));

    let avail_count = app.downloadable_plugins.len();
    let browse_text = format!(" 󰇚 [ Browse & Download Community Plugins ({avail_count} available) ] ");
    if is_sel_store {
        lines.push(Line::from(vec![
            Span::styled(" ❯ ", Style::default().fg(theme::MAUVE).add_modifier(Modifier::BOLD)),
            Span::styled(browse_text, Style::default().fg(theme::BASE).bg(theme::GREEN).add_modifier(Modifier::BOLD)),
            Span::styled("  ◀ Enter: Open Store / Download ▶", Style::default().fg(theme::PEACH).add_modifier(Modifier::BOLD)),
        ]));
    } else {
        lines.push(Line::from(vec![
            Span::raw("   "),
            Span::styled(browse_text, Style::default().fg(theme::GREEN).bg(theme::SURFACE0)),
        ]));
    }
    lines.push(Line::raw(""));

    // Render scrollable paragraph
    let visible_rows = inner.height as usize;
    let total_lines = lines.len();
    let selected_line = item_line_indices.get(app.plugin_selected).copied().unwrap_or(0);

    let scroll_offset = if total_lines <= visible_rows {
        0
    } else {
        let target = selected_line.saturating_sub(visible_rows / 3);
        target.min(total_lines.saturating_sub(visible_rows))
    } as u16;

    frame.render_widget(Paragraph::new(lines).scroll((scroll_offset, 0)), inner);

    if total_lines > visible_rows {
        let mut scrollbar_state = ScrollbarState::new(total_lines.saturating_sub(visible_rows))
            .position(scroll_offset as usize);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None)
            .thumb_style(Style::default().fg(theme::BLUE))
            .track_style(Style::default().fg(theme::SURFACE0));
        frame.render_stateful_widget(
            scrollbar,
            area.inner(Margin { vertical: 1, horizontal: 0 }),
            &mut scrollbar_state,
        );
    }

    // Modal 1: Edit Path Dialog
    if let Some(edit_idx) = app.editing_path_index {
        render_path_edit_modal(frame, app, edit_idx);
    }

    // Modal 2: Download Plugin Store Dialog
    if app.show_plugin_store {
        render_plugin_store_modal(frame, app);
    }
}

/// Floating modal for editing a configuration path
fn render_path_edit_modal(frame: &mut Frame, app: &App, edit_idx: usize) {
    let screen = frame.area();
    let label = app.config_paths.get(edit_idx).map(|p| p.label.as_str()).unwrap_or("Path");

    let popup_w = 70.min(screen.width.saturating_sub(4));
    let popup_h = 7.min(screen.height.saturating_sub(2));

    let v_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((screen.height.saturating_sub(popup_h)) / 2),
            Constraint::Length(popup_h),
            Constraint::Min(0),
        ])
        .split(screen);

    let h_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length((screen.width.saturating_sub(popup_w)) / 2),
            Constraint::Length(popup_w),
            Constraint::Min(0),
        ])
        .split(v_chunks[1]);

    let popup_area = h_chunks[1];
    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(format!(" 󰏫 Edit {label} "))
        .title_bottom(Line::from(vec![
            Span::styled(" [Enter: Save | Esc: Cancel | Backspace: Delete] ", Style::default().fg(theme::OVERLAY0)),
        ]))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::PEACH));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let lines = vec![
        Line::from(vec![
            Span::styled(" Enter new path: ", Style::default().fg(theme::YELLOW).add_modifier(Modifier::BOLD)),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(&app.editing_path_input, Style::default().fg(theme::TEXT).bg(theme::SURFACE0)),
            Span::styled("█", Style::default().fg(theme::PEACH)),
        ]),
    ];

    frame.render_widget(Paragraph::new(lines), inner);
}

/// Floating modal for Community Plugin Store / Download UI (Mockup)
fn render_plugin_store_modal(frame: &mut Frame, app: &App) {
    let screen = frame.area();
    let popup_w = 84.min(screen.width.saturating_sub(4));
    let popup_h = 22.min(screen.height.saturating_sub(4));

    let v_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((screen.height.saturating_sub(popup_h)) / 2),
            Constraint::Length(popup_h),
            Constraint::Min(0),
        ])
        .split(screen);

    let h_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length((screen.width.saturating_sub(popup_w)) / 2),
            Constraint::Length(popup_w),
            Constraint::Min(0),
        ])
        .split(v_chunks[1]);

    let popup_area = h_chunks[1];
    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" 󰇚 Community Plugin Repository (Store Preview) ")
        .title_bottom(Line::from(vec![
            Span::styled(" [↑/↓: Select | Enter: Install / Download | Esc/q: Close] ", Style::default().fg(theme::OVERLAY0)),
        ]))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::GREEN));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let mut lines = Vec::new();
    lines.push(Line::from(vec![
        Span::styled("  Available Community Plugins for Lyra", Style::default().fg(theme::GREEN).add_modifier(Modifier::BOLD)),
        Span::styled(" — Press Enter to download & install", Style::default().fg(theme::OVERLAY0)),
    ]));
    lines.push(Line::raw(""));

    for (idx, plugin) in app.downloadable_plugins.iter().enumerate() {
        let is_sel = app.store_selected == idx;
        let (status_badge, status_style) = if plugin.is_installed {
            (" 󰄬 Installed ", Style::default().fg(theme::BASE).bg(theme::GREEN).add_modifier(Modifier::BOLD))
        } else if plugin.version.contains("Coming Soon") {
            (" 󰥔 Coming Soon ", Style::default().fg(theme::BASE).bg(theme::PEACH).add_modifier(Modifier::BOLD))
        } else {
            (" 󰇚 Download  ", Style::default().fg(theme::BASE).bg(theme::BLUE).add_modifier(Modifier::BOLD))
        };

        if is_sel {
            lines.push(Line::from(vec![
                Span::styled(" ❯ ", Style::default().fg(theme::MAUVE).add_modifier(Modifier::BOLD)),
                Span::styled(format!("{:<34}", plugin.name), Style::default().fg(theme::YELLOW).add_modifier(Modifier::BOLD)),
                Span::styled(format!(" {:<11} ", plugin.version), Style::default().fg(theme::SUBTEXT0)),
                Span::styled(format!(" {:<18} ", plugin.author), Style::default().fg(theme::BLUE)),
                Span::styled(status_badge, status_style),
                Span::styled(" ◀ Enter: Install ▶", Style::default().fg(theme::PEACH).add_modifier(Modifier::BOLD)),
            ]));
            lines.push(Line::from(vec![
                Span::raw("     "),
                Span::styled(&plugin.description, Style::default().fg(theme::TEXT)),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::raw("   "),
                Span::styled(format!("{:<34}", plugin.name), Style::default().fg(theme::TEXT)),
                Span::styled(format!(" {:<11} ", plugin.version), Style::default().fg(theme::OVERLAY0)),
                Span::styled(format!(" {:<18} ", plugin.author), Style::default().fg(theme::SUBTEXT0)),
                Span::styled(status_badge, status_style),
            ]));
            lines.push(Line::from(vec![
                Span::raw("     "),
                Span::styled(&plugin.description, Style::default().fg(theme::SUBTEXT0)),
            ]));
        }
        lines.push(Line::raw(""));
    }

    let visible_rows = inner.height as usize;
    let total_lines = lines.len();
    let scroll_offset = if app.store_selected >= 3 && total_lines > visible_rows {
        ((app.store_selected - 2) * 3).min(total_lines.saturating_sub(visible_rows)) as u16
    } else {
        0
    };

    frame.render_widget(Paragraph::new(lines).scroll((scroll_offset, 0)), inner);
}
