//! Branches / Saved Paths screen — list and safely switch branches.

use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::{App, BranchesMode};
use crate::mascot::cactus;
use crate::screens::render_help_bar;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let outer = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(app.terms.title_branches())
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

    render_list(frame, cols[0], app);
    render_side_panel(frame, cols[1], app);
    render_footer(frame, vert[1], app);

    // Overlay dialogs
    if app.branches_state.mode == BranchesMode::ConfirmSwitch {
        render_confirm_dialog(frame, area, app);
    }
    if app.branches_state.mode == BranchesMode::Result {
        render_result_dialog(frame, area, app);
    }
}

// ── Left panel: branch list ──────────────────────────────────────────

fn render_list(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let state = &app.branches_state;
    let mut lines: Vec<Line> = Vec::new();

    // Error or empty
    if let Some(err) = &state.branches.error {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("  {err}"),
            Style::default().fg(Color::DarkGray),
        )));
        let p = Paragraph::new(Text::from(lines));
        frame.render_widget(p, inner);
        return;
    }

    if state.branches.branches.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  No branches found (repo may be empty).",
            Style::default().fg(Color::DarkGray),
        )));
        let p = Paragraph::new(Text::from(lines));
        frame.render_widget(p, inner);
        return;
    }

    // Header
    let count = state.branches.branches.len();
    let label = app.terms.branch();
    lines.push(Line::from(Span::styled(
        format!("  Local {label}s ({count})"),
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    for (i, branch) in state.branches.branches.iter().enumerate() {
        let is_cursor = i == state.cursor;

        // Cursor indicator
        let cursor_char = if is_cursor { "\u{25B6}" } else { " " }; // ▶
        // Current-branch marker
        let star = if branch.is_current { "*" } else { " " };

        let (name_style, meta_style) = if is_cursor {
            (
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
                Style::default().fg(Color::White),
            )
        } else if branch.is_current {
            (
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
                Style::default().fg(Color::DarkGray),
            )
        } else {
            (
                Style::default().fg(Color::Gray),
                Style::default().fg(Color::DarkGray),
            )
        };

        let current_badge = if branch.is_current { " (current)" } else { "" };

        // Line 1: cursor + star + name + current badge
        lines.push(Line::from(vec![
            Span::styled(format!("  {cursor_char} "), name_style),
            Span::styled(
                format!("{star} "),
                Style::default().fg(Color::Green),
            ),
            Span::styled(branch.name.as_str(), name_style),
            Span::styled(current_badge, Style::default().fg(Color::Green)),
        ]));

        // Line 2: hash + commit summary
        lines.push(Line::from(vec![
            Span::styled("        ", Style::default()),
            Span::styled(&branch.short_hash, Style::default().fg(Color::Yellow)),
            Span::styled(" ", Style::default()),
            Span::styled(
                truncate_str(&branch.summary, 48),
                meta_style,
            ),
        ]));

        lines.push(Line::from(""));
    }

    let paragraph = Paragraph::new(Text::from(lines));
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

    let art = Paragraph::new(Text::from(cactus::CACTUS_SMALL))
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Center);
    frame.render_widget(art, chunks[1]);

    let branch_word = app.terms.branch();
    let tip_line = format!(
        " A {branch_word} is another line of work you can switch to.",
    );
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

    let edu_title = format!(" What is a {}?", app.terms.branch());
    let edu = Paragraph::new(Text::from(vec![
        Line::from(Span::styled(
            edu_title,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            " It lets you experiment without",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            " changing your main work.",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            " * marks the current one.",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            " Switching needs a clean tree.",
            Style::default().fg(Color::DarkGray),
        )),
    ]))
    .wrap(Wrap { trim: true });
    frame.render_widget(edu, chunks[5]);
}

// ── Footer ───────────────────────────────────────────────────────────

fn render_footer(frame: &mut Frame, area: Rect, app: &App) {
    let selected_is_current = app.branches_state.selected_is_current();
    let mut bindings: Vec<(&str, &str)> = vec![
        ("\u{2191}/\u{2193}/w/s", "move"),
    ];
    if !selected_is_current && !app.branches_state.branches.branches.is_empty() {
        bindings.push(("Enter", "switch"));
    }
    bindings.push(("r", "refresh"));
    bindings.push(("Esc", "back"));
    bindings.push(("q", "quit"));
    render_help_bar(frame, area, &bindings);
}

// ── Confirmation dialog ──────────────────────────────────────────────

fn render_confirm_dialog(frame: &mut Frame, area: Rect, app: &App) {
    let name = app.branches_state.selected_name().unwrap_or("???");
    let dialog = centered_rect(60, 11, area);
    frame.render_widget(Clear, dialog);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Confirm Switch ")
        .title_alignment(Alignment::Center);
    let inner = block.inner(dialog);
    frame.render_widget(block, dialog);

    let branch_word = app.terms.branch();
    let text = Paragraph::new(Text::from(vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  Switch to {branch_word}?"),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("    ", Style::default()),
            Span::styled(name, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Only works if your working tree is clean.",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            "  No changes will be discarded.",
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
        .branches_state
        .result_msg
        .as_ref()
        .map(|(m, ok)| (m.as_str(), *ok))
        .unwrap_or(("Done.", true));

    let dialog = centered_rect(60, 7, area);
    frame.render_widget(Clear, dialog);

    let color = if is_ok { Color::Green } else { Color::Red };
    let title = if is_ok { " Switched " } else { " Blocked " };

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
            "  Press any key to continue.",
            Style::default().fg(Color::DarkGray),
        )),
    ]))
    .wrap(Wrap { trim: true });
    frame.render_widget(text, inner);
}

// ── Helpers ──────────────────────────────────────────────────────────

fn truncate_str(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}\u{2026}", &s[..max - 1])
    }
}

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
