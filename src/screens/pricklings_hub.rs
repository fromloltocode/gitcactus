//! Pricklings hub — saved projects + action rows.
//!
//! Layout:
//!   Top area:  saved pricklings (one row each, cursor-selectable)
//!   Action rows: "Find new pricklings" and "Manage scan locations"
//!
//! The cursor walks saved-then-actions as one continuous list so the
//! same keyboard flow as other screens (↑/↓ + Enter) just works.

use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::{App, HubSelection, PricklingsHubState};
use crate::mascot::cactus;
use crate::screens::render_help_bar;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let outer = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(app.terms.title_pricklings())
        .title_alignment(Alignment::Center);
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let vert = Layout::vertical([Constraint::Min(1), Constraint::Length(2)]).split(inner);
    let cols = Layout::horizontal([Constraint::Percentage(65), Constraint::Percentage(35)])
        .split(vert[0]);

    render_list(frame, cols[0], app);
    render_side(frame, cols[1], app);
    render_footer(frame, vert[1]);
}

fn render_list(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let hub = &app.pricklings_hub;
    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(Span::styled(
        "  Saved pricklings",
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    if hub.store.saved.is_empty() {
        lines.push(Line::from(Span::styled(
            "     (none yet — go find some!)",
            Style::default().fg(Color::DarkGray),
        )));
        lines.push(Line::from(""));
    } else {
        for (i, p) in hub.store.saved.iter().enumerate() {
            let is_cursor = hub.cursor == i;
            lines.push(row(
                is_cursor,
                app,
                p.display_name.clone(),
                p.display_path(),
            ));
        }
        lines.push(Line::from(""));
    }

    lines.push(Line::from(Span::styled(
        "  Actions",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(""));

    // Action rows — their cursor positions sit immediately after
    // saved entries.
    let first_action = hub.store.saved.len();
    let find_label = app.terms.pricklings_find_action();
    let manage_label = "Manage scan locations";
    let find_desc = "Scan your approved roots for Git repositories.";
    let manage_desc = "Add or remove the directories that get scanned.";

    lines.push(row(
        hub.cursor == first_action,
        app,
        find_label.to_string(),
        find_desc.to_string(),
    ));
    lines.push(row(
        hub.cursor == first_action + 1,
        app,
        manage_label.to_string(),
        manage_desc.to_string(),
    ));

    // Last-action result banner.
    if let Some((msg, ok)) = &hub.result_msg {
        lines.push(Line::from(""));
        let color = if *ok { app.theme.success } else { app.theme.error };
        lines.push(Line::from(Span::styled(format!("  {msg}"), Style::default().fg(color))));
    }

    let paragraph = Paragraph::new(Text::from(lines));
    frame.render_widget(paragraph, inner);
}

/// One row (title + subtitle) styled for the current cursor state.
/// Takes owned Strings so the returned `Line` is `'static` — lets us
/// freely push it into a `Vec<Line>` without borrow gymnastics.
fn row(is_cursor: bool, app: &App, title: String, subtitle: String) -> Line<'static> {
    let prefix = if is_cursor { " >" } else { "  " };
    let (label_style, sub_style) = if is_cursor {
        (
            Style::default()
                .fg(Color::Black)
                .bg(app.theme.highlight)
                .add_modifier(Modifier::BOLD),
            Style::default().fg(Color::White),
        )
    } else {
        (
            Style::default().fg(Color::Gray),
            Style::default().fg(Color::DarkGray),
        )
    };
    Line::from(vec![
        Span::styled(format!("  {prefix} "), label_style),
        Span::styled(title, label_style),
        Span::styled(format!("  —  {subtitle}"), sub_style),
    ])
}

fn render_side(frame: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(10),
        Constraint::Length(1),
        Constraint::Length(5),
        Constraint::Length(1),
        Constraint::Min(3),
    ])
    .split(area);

    let art = Paragraph::new(Text::from(cactus::small()))
        .style(Style::default().fg(app.theme.cactus))
        .alignment(Alignment::Center);
    frame.render_widget(art, chunks[1]);

    let tip_text = tip_for_selection(&app.pricklings_hub);
    let tip = Paragraph::new(Text::from(vec![
        Line::from(Span::styled(
            " Cactus says:",
            Style::default()
                .fg(app.theme.success)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            format!(" {tip_text}"),
            Style::default().fg(Color::White),
        )),
    ]))
    .wrap(Wrap { trim: true });
    frame.render_widget(tip, chunks[3]);

    let explainer = Paragraph::new(Text::from(vec![
        Line::from(Span::styled(
            " What's a prickling?",
            Style::default()
                .fg(app.theme.primary)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            " A local Git project you've opened or",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            " saved. Nothing is sent to a server —",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            " this list lives on your machine.",
            Style::default().fg(Color::DarkGray),
        )),
    ]))
    .wrap(Wrap { trim: true });
    frame.render_widget(explainer, chunks[5]);
}

fn tip_for_selection(hub: &PricklingsHubState) -> &'static str {
    match hub.selection() {
        HubSelection::Saved(_) => "Press Enter to open this prickling. 'd' removes it from your list.",
        HubSelection::Find => "Press Enter to scan your approved roots for Git repositories.",
        HubSelection::ManageLocations => "Press Enter to add, remove, or scan approved locations.",
    }
}

fn render_footer(frame: &mut Frame, area: Rect) {
    render_help_bar(
        frame,
        area,
        &[
            ("\u{2191}/\u{2193}/w/s", "move"),
            ("Enter", "open/select"),
            ("d", "remove"),
            ("Esc", "back"),
            ("q", "quit"),
        ],
    );
}
