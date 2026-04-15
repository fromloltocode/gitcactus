//! Outside-repo entry screen.
//!
//! Shown when `gitcactus` is launched in a directory that isn't a Git
//! repository. Gives the user an on-ramp into the Pricklings flow
//! without forcing them to `cd` first or read the README.

use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::{App, LAUNCHPAD_ITEMS};
use crate::mascot::cactus;
use crate::screens::render_help_bar;

/// Labels shown on the Launchpad, in cursor order.
/// Index 0 = Pricklings, 1 = Settings, 2 = Exit.
pub const LAUNCHPAD_ACTION_SUBTITLES: &[&str] = &[
    "Open a project or manage your saved pricklings.",
    "Theme and terminology preferences.",
    "Quit GitCactus.",
];

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let outer = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(" GitCactus ")
        .title_alignment(Alignment::Center);
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let columns = Layout::horizontal([
        Constraint::Length(22), // sidebar cactus
        Constraint::Min(30),
    ])
    .split(inner);

    render_sidebar(frame, columns[0], app);
    render_main(frame, columns[1], app);
}

fn render_sidebar(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(10),
        Constraint::Length(1),
        Constraint::Min(1),
    ])
    .split(inner);

    let art = Paragraph::new(Text::from(cactus::small()))
        .style(Style::default().fg(app.theme.cactus))
        .alignment(Alignment::Center);
    frame.render_widget(art, chunks[1]);

    let tip = Paragraph::new(Text::from(vec![
        Line::from(Span::styled(
            " Cactus says:",
            Style::default()
                .fg(app.theme.success)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            " You're not inside a Git repo yet. Let's find one!",
            Style::default().fg(Color::White),
        )),
    ]))
    .wrap(Wrap { trim: true });
    frame.render_widget(tip, chunks[3]);
}

fn render_main(frame: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::vertical([
        Constraint::Length(3), // header
        Constraint::Min(1),    // items
        Constraint::Length(2), // footer
    ])
    .split(area);

    let header = Paragraph::new(Text::from(vec![
        Line::from(Span::styled(
            "  Welcome back to GitCactus",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "  You're outside a repository — pick one to open.",
            Style::default().fg(Color::DarkGray),
        )),
    ]));
    frame.render_widget(header, chunks[0]);

    let labels = labels_for(app);
    let cursor = app.launchpad.cursor.min(LAUNCHPAD_ITEMS - 1);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));
    for (i, label) in labels.iter().enumerate() {
        let is_cursor = i == cursor;
        let prefix = if is_cursor { " > " } else { "   " };
        let (style_label, style_desc) = if is_cursor {
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
        lines.push(Line::from(Span::styled(
            format!(" {prefix}{label}"),
            style_label,
        )));
        if let Some(sub) = LAUNCHPAD_ACTION_SUBTITLES.get(i) {
            lines.push(Line::from(Span::styled(format!("       {sub}"), style_desc)));
        }
        lines.push(Line::from(""));
    }
    let list = Paragraph::new(Text::from(lines));
    frame.render_widget(list, chunks[1]);

    render_help_bar(
        frame,
        chunks[2],
        &[
            ("\u{2191}/\u{2193}/w/s", "move"),
            ("Enter", "select"),
            ("q", "quit"),
        ],
    );
}

/// Terminology-aware labels for each Launchpad row.
fn labels_for(app: &App) -> [String; LAUNCHPAD_ITEMS] {
    let pricklings = app.terms.pricklings_menu_label().to_string();
    [
        pricklings,
        "Settings".to_string(),
        "Exit".to_string(),
    ]
}
