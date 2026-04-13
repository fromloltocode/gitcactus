/// Top-level rendering dispatcher.
///
/// The `draw` function inspects `app.screen` and delegates to the
/// appropriate screen module. No business logic lives here.
use ratatui::Frame;

use crate::app::{App, Screen};
use crate::git::status::RepoStatus;
use crate::screens::{branches, commit, commit_details, diff, help, history, intro, menu, settings_screen, stage, status, title, update};

pub fn draw(frame: &mut Frame, app: &App, repo_status: &RepoStatus) {
    let area = frame.area();

    match app.screen {
        Screen::Intro => intro::render(frame, area, app),
        Screen::Title => title::render(frame, area),
        Screen::Menu => menu::render(frame, area, app),
        Screen::Status => status::render(frame, area, repo_status, &app.terms),
        Screen::Stage => stage::render(frame, area, app),
        Screen::Update => update::render(frame, area, app),
        Screen::Commit => commit::render(frame, area, app),
        Screen::Help => help::render(frame, area, app),
        Screen::DiffPreview => diff::render(frame, area, app),
        Screen::Settings => settings_screen::render(frame, area, app),
        Screen::CommitDetails => commit_details::render(frame, area, app),
        Screen::History => history::render(frame, area, app),
        Screen::Branches => branches::render(frame, area, app),
        Screen::RemoteSync => status::render_placeholder(frame, area, "Remote Sync"),
    }
}
