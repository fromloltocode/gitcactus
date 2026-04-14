use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::{App, CommitMode};
use crate::mascot::cactus;
use crate::screens::render_help_bar;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let outer = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(app.terms.title_commit())
        .title_alignment(Alignment::Center);
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    // Body + footer
    let vert = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(2),
    ])
    .split(inner);

    // Two-column body
    let cols = Layout::horizontal([
        Constraint::Percentage(60),
        Constraint::Percentage(40),
    ])
    .split(vert[0]);

    render_main_panel(frame, cols[0], app);
    render_side_panel(frame, cols[1], app);
    render_footer(frame, vert[1], app);

    // Overlay dialogs
    if app.commit.mode == CommitMode::Confirm {
        render_confirm_dialog(frame, area, app);
    }
    if app.commit.mode == CommitMode::Result {
        render_result_dialog(frame, area, app);
    }
}

// ── Left panel: staged files + message input ─────────────────────────

fn render_main_panel(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let commit = &app.commit;
    let mut lines: Vec<Line> = Vec::new();

    // Staged files summary
    lines.push(Line::from(Span::styled(
        "  Staged Files",
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    if commit.staged_count == 0 {
        lines.push(Line::from(Span::styled(
            "  No files staged.",
            Style::default().fg(Color::Red),
        )));
        lines.push(Line::from(Span::styled(
            "  Use Stage Changes from the menu first.",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        lines.push(Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(
                format!("{} file{} ready to commit:",
                    commit.staged_count,
                    if commit.staged_count == 1 { "" } else { "s" }
                ),
                Style::default().fg(Color::Green),
            ),
        ]));
        lines.push(Line::from(""));

        let display_limit = 8;
        for (i, path) in commit.staged_files.iter().enumerate() {
            if i >= display_limit {
                lines.push(Line::from(Span::styled(
                    format!("     … and {} more", commit.staged_count - display_limit),
                    Style::default().fg(Color::DarkGray),
                )));
                break;
            }
            lines.push(Line::from(vec![
                Span::styled("   * ", Style::default().fg(Color::Green)),
                Span::styled(path.as_str(), Style::default().fg(Color::White)),
            ]));
        }
    }

    lines.push(Line::from(""));

    // Commit message input
    lines.push(Line::from(Span::styled(
        "  ── Commit Message ──",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(""));

    if commit.staged_count == 0 {
        lines.push(Line::from(Span::styled(
            "  (stage files first)",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        // Show the message with a blinking cursor
        let msg_display = if commit.mode == CommitMode::Editing {
            format!("{}█", commit.message)
        } else {
            commit.message.clone()
        };

        let msg_color = if commit.message.is_empty() {
            Color::DarkGray
        } else {
            Color::White
        };

        lines.push(Line::from(vec![
            Span::styled("  > ", Style::default().fg(Color::Cyan)),
            Span::styled(
                if msg_display.is_empty() {
                    "█".to_string()
                } else {
                    msg_display
                },
                Style::default().fg(msg_color),
            ),
        ]));

        lines.push(Line::from(""));

        // Character count
        let len = commit.message.len();
        let count_color = if len > 72 { Color::Yellow } else { Color::DarkGray };
        lines.push(Line::from(Span::styled(
            format!("  {len}/200 characters (aim for under 72)"),
            Style::default().fg(count_color),
        )));
    }

    let paragraph = Paragraph::new(Text::from(lines));
    frame.render_widget(paragraph, inner);
}

// ── Right panel: cactus + tips ───────────────────────────────────────

fn render_side_panel(frame: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::vertical([
        Constraint::Length(1),  // top padding
        Constraint::Length(10), // cactus art
        Constraint::Length(1),  // spacing
        Constraint::Length(5),  // contextual tip
        Constraint::Length(1),  // spacing
        Constraint::Min(3),     // educational note
    ])
    .split(area);

    // Cactus art
    let art = Paragraph::new(Text::from(cactus::small()))
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Center);
    frame.render_widget(art, chunks[1]);

    // Contextual tip
    let commit = &app.commit;
    let tip_text = cactus::commit_tip(
        commit.staged_count,
        !commit.message.trim().is_empty(),
    );
    let tip = Paragraph::new(Text::from(vec![
        Line::from(Span::styled(
            " Cactus says:",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            format!(" {tip_text}"),
            Style::default().fg(Color::White),
        )),
    ]))
    .wrap(Wrap { trim: true });
    frame.render_widget(tip, chunks[3]);

    // Educational note
    let edu = Paragraph::new(Text::from(vec![
        Line::from(Span::styled(
            " What is a commit?",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            " A commit is a snapshot of your",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            " staged changes. It records what",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            " changed and why, so you can",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            " always go back to this point.",
            Style::default().fg(Color::DarkGray),
        )),
    ]))
    .wrap(Wrap { trim: true });
    frame.render_widget(edu, chunks[5]);
}

// ── Footer ───────────────────────────────────────────────────────────

fn render_footer(frame: &mut Frame, area: Rect, app: &App) {
    if app.commit.staged_count == 0 {
        render_help_bar(frame, area, &[
            ("Esc", "back"),
        ]);
    } else if app.commit.can_commit() {
        render_help_bar(frame, area, &[
            ("type", "edit message"),
            ("Enter", "commit"),
            ("Esc", "back"),
        ]);
    } else {
        render_help_bar(frame, area, &[
            ("type", "edit message"),
            ("Backspace", "delete"),
            ("Esc", "back"),
        ]);
    }
}

// ── Confirmation dialog ──────────────────────────────────────────────

fn render_confirm_dialog(frame: &mut Frame, area: Rect, app: &App) {
    let count = app.commit.staged_count;
    let dialog = centered_rect(60, 11, area);
    frame.render_widget(Clear, dialog);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Confirm Commit ")
        .title_alignment(Alignment::Center);
    let inner = block.inner(dialog);
    frame.render_widget(block, dialog);

    let msg_preview: String = if app.commit.message.len() > 50 {
        format!("{}…", &app.commit.message[..49])
    } else {
        app.commit.message.clone()
    };

    let text = Paragraph::new(Text::from(vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  Create commit with {count} staged file{}?",
                if count == 1 { "" } else { "s" }),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Message: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("\"{msg_preview}\""),
                Style::default().fg(Color::Cyan),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  This will create a permanent commit in your history.",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  y/Enter", Style::default().fg(Color::Green)),
            Span::styled(" confirm    ", Style::default().fg(Color::DarkGray)),
            Span::styled("n/Esc", Style::default().fg(Color::Red)),
            Span::styled(" cancel", Style::default().fg(Color::DarkGray)),
        ]),
    ]));
    frame.render_widget(text, inner);
}

// ── Result dialog ────────────────────────────────────────────────────

fn render_result_dialog(frame: &mut Frame, area: Rect, app: &App) {
    let (msg, is_ok) = app
        .commit
        .result_msg
        .as_ref()
        .map(|(m, ok)| (m.as_str(), *ok))
        .unwrap_or(("Done.", true));

    let dialog = centered_rect(55, 7, area);
    frame.render_widget(Clear, dialog);

    let color = if is_ok { Color::Green } else { Color::Red };
    let title = if is_ok { " Commit Created " } else { " Error " };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(color))
        .title(title)
        .title_alignment(Alignment::Center);
    let inner = block.inner(dialog);
    frame.render_widget(block, dialog);

    let text = Paragraph::new(Text::from(vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  {msg}"),
            Style::default().fg(Color::White),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Press any key to return to menu.",
            Style::default().fg(Color::DarkGray),
        )),
    ]));
    frame.render_widget(text, inner);
}

// ── Helpers ──────────────────────────────────────────────────────────

fn centered_rect(percent_x: u16, height: u16, area: Rect) -> Rect {
    let vert = Layout::vertical([
        Constraint::Min(0),
        Constraint::Length(height),
        Constraint::Min(0),
    ])
    .split(area);
    let horiz = Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(vert[1]);
    horiz[1]
}
