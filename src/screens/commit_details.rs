//! Commit Details screen — read-only view of a single commit.

use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::App;
use crate::git::commit_details::ChangeKind;
use crate::mascot::cactus;
use crate::screens::render_help_bar;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let outer = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(app.terms.title_commit_details())
        .title_alignment(Alignment::Center);
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let vert = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(2),
    ])
    .split(inner);

    let cols = Layout::horizontal([
        Constraint::Percentage(65),
        Constraint::Percentage(35),
    ])
    .split(vert[0]);

    render_main_panel(frame, cols[0], app);
    render_side_panel(frame, cols[1], app);
    render_footer(frame, vert[1]);
}

// ── Left panel: details + file list ──────────────────────────────────

fn render_main_panel(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let details = &app.commit_details.details;

    // Error state
    if let Some(err) = &details.error {
        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                format!("  {err}"),
                Style::default().fg(Color::Red),
            )),
        ];
        let p = Paragraph::new(Text::from(lines));
        frame.render_widget(p, inner);
        return;
    }

    let mut lines: Vec<Line> = Vec::new();

    // Hash header
    lines.push(Line::from(vec![
        Span::styled("  Hash:    ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            &details.short_hash,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("  ({})", details.full_hash),
            Style::default().fg(Color::DarkGray),
        ),
    ]));

    // Author
    lines.push(Line::from(vec![
        Span::styled("  Author:  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            &details.author,
            Style::default().fg(Color::White),
        ),
        Span::styled(
            if details.email.is_empty() {
                String::new()
            } else {
                format!(" <{}>", details.email)
            },
            Style::default().fg(Color::DarkGray),
        ),
    ]));

    // Time
    lines.push(Line::from(vec![
        Span::styled("  When:    ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            &details.relative_time,
            Style::default().fg(Color::Cyan),
        ),
    ]));

    lines.push(Line::from(""));

    // Message header + body
    lines.push(Line::from(Span::styled(
        "  ── Message ──",
        Style::default().fg(Color::DarkGray),
    )));
    if details.message.is_empty() {
        lines.push(Line::from(Span::styled(
            "  (no message)",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        for msg_line in details.message.lines() {
            lines.push(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(msg_line, Style::default().fg(Color::White)),
            ]));
        }
    }

    lines.push(Line::from(""));

    // File changes
    let file_count = details.files.len();
    let files_label = format!("  ── Files Changed ({file_count}) ──");
    lines.push(Line::from(Span::styled(
        files_label,
        Style::default().fg(Color::DarkGray),
    )));

    if details.files.is_empty() {
        lines.push(Line::from(Span::styled(
            "     (none)",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        for f in &details.files {
            let (symbol, color) = match f.kind {
                ChangeKind::Added => ("+", Color::Green),
                ChangeKind::Modified => ("~", Color::Yellow),
                ChangeKind::Deleted => ("-", Color::Red),
                ChangeKind::Renamed => ("\u{2192}", Color::Cyan), // →
                ChangeKind::Other => ("?", Color::DarkGray),
            };
            lines.push(Line::from(vec![
                Span::styled(format!("   {symbol} "), Style::default().fg(color)),
                Span::styled(&f.path, Style::default().fg(Color::White)),
            ]));
        }
        if details.files_truncated {
            lines.push(Line::from(Span::styled(
                "     … list truncated",
                Style::default().fg(Color::Yellow),
            )));
        }
    }

    // Apply scroll offset
    let scroll = app.commit_details.scroll;
    let start = scroll.min(lines.len().saturating_sub(1));
    let visible = &lines[start..];

    let paragraph = Paragraph::new(Text::from(visible.to_vec()));
    frame.render_widget(paragraph, inner);
}

// ── Right panel: cactus + explanation ─────────────────────────────────

fn render_side_panel(frame: &mut Frame, area: Rect, app: &App) {
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
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Center);
    frame.render_widget(art, chunks[1]);

    let commit_word = app.terms.commit();
    let tip_line = format!(" This {commit_word} is a snapshot — you can always come back to it.");
    let tip = Paragraph::new(Text::from(vec![
        Line::from(Span::styled(
            " Cactus says:",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            tip_line,
            Style::default().fg(Color::White),
        )),
    ]))
    .wrap(Wrap { trim: true });
    frame.render_widget(tip, chunks[3]);

    let edu = Paragraph::new(Text::from(vec![
        Line::from(Span::styled(
            " File markers",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled(" + ", Style::default().fg(Color::Green)),
            Span::styled("added   ", Style::default().fg(Color::DarkGray)),
            Span::styled("~ ", Style::default().fg(Color::Yellow)),
            Span::styled("modified", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled(" - ", Style::default().fg(Color::Red)),
            Span::styled("deleted   ", Style::default().fg(Color::DarkGray)),
            Span::styled("\u{2192} ", Style::default().fg(Color::Cyan)),
            Span::styled("renamed", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            " Press d to see the full diff.",
            Style::default().fg(Color::DarkGray),
        )),
    ]))
    .wrap(Wrap { trim: true });
    frame.render_widget(edu, chunks[5]);
}

// ── Footer ───────────────────────────────────────────────────────────

fn render_footer(frame: &mut Frame, area: Rect) {
    render_help_bar(frame, area, &[
        ("\u{2191}/\u{2193}/w/s", "scroll"),
        ("d/Enter", "view diff"),
        ("o", "open 1st file"),
        ("Esc", "back"),
        ("q", "quit"),
    ]);
}
