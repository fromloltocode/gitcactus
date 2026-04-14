/// Top-level rendering dispatcher.
///
/// The `draw` function inspects `app.screen` and delegates to the
/// appropriate screen module. No business logic lives here.
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::app::{App, Screen};
use crate::git::status::RepoStatus;
use crate::screens::{branches, commit, commit_details, diff, help, history, intro, menu, rebase_portal, settings_screen, stage, status, title, update};

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
        Screen::RebasePortal => rebase_portal::render(frame, area, app),
        Screen::RemoteSync => status::render_placeholder(frame, area, "Remote Sync"),
    }

    // Overlay any transient editor message after the screen renders.
    if let Some((msg, is_ok)) = &app.editor_msg {
        render_editor_banner(frame, area, msg, *is_ok);
    }
}

fn render_editor_banner(frame: &mut Frame, area: Rect, msg: &str, is_ok: bool) {
    if area.height < 3 || area.width < 10 {
        return;
    }
    let width = area.width.saturating_sub(4).min(80);
    let height = 3u16;
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + area.height.saturating_sub(height + 1);
    let banner_area = Rect { x, y, width, height };

    frame.render_widget(Clear, banner_area);

    let color = if is_ok { Color::Green } else { Color::Yellow };
    let title = if is_ok { " Editor " } else { " Editor \u{2014} Notice " };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(color))
        .title(title)
        .title_alignment(Alignment::Center);
    let inner = block.inner(banner_area);
    frame.render_widget(block, banner_area);

    let text = Paragraph::new(Line::from(Span::styled(
        format!(" {msg}   (press any key to dismiss)"),
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    )));
    frame.render_widget(text, inner);
}
