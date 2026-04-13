/// Top-level rendering dispatcher.
///
/// The `draw` function inspects `app.screen` and delegates to the
/// appropriate screen module. No business logic lives here.
use ratatui::Frame;

use crate::app::{App, Screen};
use crate::git::status::RepoStatus;
use crate::screens::{menu, stage, status, title, update};

pub fn draw(frame: &mut Frame, app: &App, repo_status: &RepoStatus) {
    let area = frame.area();

    match app.screen {
        Screen::Title => title::render(frame, area),
        Screen::Menu => menu::render(frame, area, app),
        Screen::Status => status::render(frame, area, repo_status),
        Screen::Stage => stage::render(frame, area, app),
        Screen::Update => update::render(frame, area, app),
        Screen::Commit => status::render_placeholder(frame, area, "Commit Changes"),
        Screen::Branches => status::render_placeholder(frame, area, "Branches"),
        Screen::History => status::render_placeholder(frame, area, "History"),
        Screen::RemoteSync => status::render_placeholder(frame, area, "Remote Sync"),
        Screen::Help => status::render_placeholder(frame, area, "Help"),
    }
}
