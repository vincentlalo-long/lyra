use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};
use crate::app::App;

mod cover_modal;
mod form;
mod progress;
mod results;

pub fn render(frame: &mut Frame, app: &mut App, area: Rect) {
    let vert_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Search / URL Input Box
            Constraint::Length(5),  // Active Download Progress Card (or status)
            Constraint::Min(5),     // Search Results / Guidelines
        ])
        .split(area);

    results::render_input_bar(frame, app, vert_chunks[0]);
    progress::render_download_status(frame, app, vert_chunks[1]);
    results::render_search_results(frame, app, vert_chunks[2]);

    if app.download.show_metadata_form {
        form::render_metadata_form(frame, app);
    }

    if app.download.show_dir_picker {
        crate::ui::dir_picker::render(frame, app);
    }

    if app.download.show_cover_picker_modal {
        cover_modal::render_cover_picker_modal(frame, app);
    }

    if app.download.show_cover_file_picker {
        cover_modal::render_cover_file_picker_modal(frame, app);
    }
}
