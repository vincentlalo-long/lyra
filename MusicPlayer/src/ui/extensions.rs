use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};
use crate::{app::App, theme};

/// Render the Extensions Workspace (Tab 4).
/// Hosts installed interactive plugins (such as YouTube Audio Downloader),
/// or displays a clean status card if no interactive extension is currently active.
pub fn render(frame: &mut Frame, app: &mut App, area: Rect) {
    #[cfg(feature = "download")]
    {
        let is_download_enabled = app
            .plugins
            .iter()
            .find(|p| p.id == "download")
            .map(|p| p.enabled && !p.is_removed)
            .unwrap_or(true);

        if is_download_enabled {
            super::download::render(frame, app, area);
            return;
        }
    }

    // Empty state / Guidance view when no interactive extension is active
    let block = Block::default()
        .title(" 󱊄 Extensions Workspace ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::MAUVE));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 6 {
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(inner.height.saturating_sub(6) / 2),
            Constraint::Length(6),
            Constraint::Min(0),
        ])
        .split(inner);

    let text = vec![
        Line::from(vec![
            Span::styled("󱊄 No Active Extensions Enabled", Style::default().fg(theme::YELLOW).add_modifier(Modifier::BOLD)),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled("All interactive extensions are currently disabled or not installed.", Style::default().fg(theme::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("Go to [5] Plugins to enable or install community extensions.", Style::default().fg(theme::SUBTEXT0)),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled("Press [5] Plugins Manager  •  [1] Playlist  •  [?] Help", Style::default().fg(theme::MAUVE).add_modifier(Modifier::BOLD)),
        ]),
    ];

    frame.render_widget(Paragraph::new(text).alignment(Alignment::Center), chunks[1]);
}
