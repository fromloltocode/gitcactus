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
use crate::screens::{
    branches, commit, commit_details, diff, help, history, intro, menu, rebase_execute,
    rebase_portal, remote_sync, settings_screen, skill_tree, stage, status, title, update,
};

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
        Screen::SkillTree => skill_tree::render(frame, area, app),
        Screen::RebasePortal => rebase_portal::render(frame, area, app),
        Screen::RebaseExecute => rebase_execute::render(frame, area, app),
        Screen::RemoteSync => remote_sync::render(frame, area, app),
    }

    // Overlay any active animation on top of the normal screen.
    // Animations take priority over the editor banner so a success
    // animation is visible in the moment it's triggered.
    if let Some(anim) = app.animation.as_ref() {
        render_animation_overlay(frame, area, anim, &app.terms);
    } else if let Some((msg, is_ok)) = &app.editor_msg {
        render_editor_banner(frame, area, msg, *is_ok);
    }
}

/// Render a full overlay containing the current animation frame and,
/// during the Teaching phase, a 3–5 line explanation.
fn render_animation_overlay(
    frame: &mut Frame,
    area: Rect,
    anim: &crate::mascot::animations::AnimationState,
    terms: &crate::terminology::Terms,
) {
    use crate::mascot::animations::AnimationPhase;
    use ratatui::layout::Constraint;
    use ratatui::layout::Layout;
    use ratatui::text::Text;
    use ratatui::widgets::Wrap;

    // Compute a centered rect roughly 78x16. We clamp so tiny terminals
    // still get something sensible rather than panicking.
    let w = area.width.min(80);
    let h = area.height.min(18);
    if w < 30 || h < 10 {
        return; // terminal too small — skip the overlay entirely
    }
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    let rect = Rect { x, y, width: w, height: h };

    frame.render_widget(Clear, rect);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(anim.title(terms))
        .title_alignment(Alignment::Center);
    let inner = block.inner(rect);
    frame.render_widget(block, rect);

    // Split: animation art (top) / teaching + hint (bottom)
    let chunks = Layout::vertical([
        Constraint::Length(9), // 8 art lines + 1 breathing room
        Constraint::Min(3),    // teaching lines
        Constraint::Length(1), // dismiss hint
    ])
    .split(inner);

    let art = Paragraph::new(Text::from(anim.current_art()))
        .style(Style::default().fg(Color::Green))
        .alignment(Alignment::Center);
    frame.render_widget(art, chunks[0]);

    // Teaching text only appears once we're past the playing phase.
    if anim.phase == AnimationPhase::Teaching {
        let mut lines: Vec<Line> = Vec::new();
        for (i, text) in anim.teaching_lines(terms).iter().enumerate() {
            let style = if i == 0 {
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };
            lines.push(Line::from(Span::styled(format!("  {text}"), style)));
        }
        let teaching = Paragraph::new(Text::from(lines)).wrap(Wrap { trim: true });
        frame.render_widget(teaching, chunks[1]);
    }

    let hint_text = match anim.phase {
        AnimationPhase::Playing => " (any key to skip) ",
        AnimationPhase::Teaching => " (any key to dismiss) ",
    };
    let hint = Paragraph::new(Line::from(Span::styled(
        hint_text,
        Style::default().fg(Color::DarkGray),
    )))
    .alignment(Alignment::Center);
    frame.render_widget(hint, chunks[2]);
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
