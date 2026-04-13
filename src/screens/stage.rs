use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::{App, StageFileKind, StageMode};
use crate::mascot::cactus;
use crate::screens::render_help_bar;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let outer = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(" Stage Changes ")
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

    render_file_panel(frame, cols[0], app);
    render_side_panel(frame, cols[1], app);
    render_footer(frame, vert[1], app);

    // Overlay confirmation dialog if active
    if app.stage.mode == StageMode::Confirm {
        render_confirm_dialog(frame, area, app);
    }
    if app.stage.mode == StageMode::Result {
        render_result_dialog(frame, area, app);
    }
}

// ── Left panel: file list ────────────────────────────────────────────

fn render_file_panel(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let stage = &app.stage;
    let mut lines: Vec<Line> = Vec::new();

    // Already-staged section (read-only)
    if !stage.already_staged.is_empty() {
        lines.push(Line::from(Span::styled(
            "  Already Staged",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )));
        for (i, path) in stage.already_staged.iter().enumerate() {
            if i >= 10 {
                lines.push(Line::from(Span::styled(
                    format!("     … and {} more", stage.already_staged.len() - 10),
                    Style::default().fg(Color::DarkGray),
                )));
                break;
            }
            lines.push(Line::from(vec![
                Span::styled("   * ", Style::default().fg(Color::Green)),
                Span::styled(path.as_str(), Style::default().fg(Color::DarkGray)),
            ]));
        }
        lines.push(Line::from(""));
    }

    // Stageable files header
    lines.push(Line::from(Span::styled(
        "  Select files to stage",
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    if stage.entries.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No modified or untracked files.",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        for (i, entry) in stage.entries.iter().enumerate() {
            let is_cursor = i == stage.cursor;
            let checkbox = if entry.selected { "[x]" } else { "[ ]" };
            let symbol = match entry.kind {
                StageFileKind::Modified => "~",
                StageFileKind::Untracked => "?",
            };
            let kind_color = match entry.kind {
                StageFileKind::Modified => Color::Yellow,
                StageFileKind::Untracked => Color::Red,
            };

            let (prefix, name_style) = if is_cursor {
                (
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                (Style::default().fg(Color::Gray), Style::default().fg(Color::Gray))
            };

            let cursor_char = if is_cursor { ">" } else { " " };

            lines.push(Line::from(vec![
                Span::styled(format!("  {cursor_char} "), prefix),
                Span::styled(format!("{checkbox} "), prefix),
                Span::styled(format!("{symbol} "), Style::default().fg(kind_color)),
                Span::styled(entry.path.as_str(), name_style),
            ]));
        }
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
    let art = Paragraph::new(Text::from(cactus::CACTUS_SMALL))
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Center);
    frame.render_widget(art, chunks[1]);

    // Contextual tip
    let stage = &app.stage;
    let tip_text = cactus::stage_tip(stage.selected_count(), stage.entries.len());
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
            " Staging = choosing",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            " Staging picks which changes go",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            " into your next commit. Nothing",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            " is committed until you say so.",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from(Span::styled(
            " ~ modified  ? untracked",
            Style::default().fg(Color::DarkGray),
        )),
    ]))
    .wrap(Wrap { trim: true });
    frame.render_widget(edu, chunks[5]);
}

// ── Footer ───────────────────────────────────────────────────────────

fn render_footer(frame: &mut Frame, area: Rect, app: &App) {
    let selected = app.stage.selected_count();
    let stage_label: String;
    let mut bindings: Vec<(&str, &str)> = vec![
        ("\u{2191}/\u{2193}/w/s", "move"),
        ("Space", "toggle"),
        ("d", "diff"),
        ("a", "all"),
        ("r", "refresh"),
    ];
    if selected > 0 {
        stage_label = format!("stage ({selected})");
        bindings.push(("Enter", &stage_label));
    }
    bindings.push(("Esc", "back"));
    bindings.push(("q", "quit"));
    render_help_bar(frame, area, &bindings);
}

// ── Confirmation dialog ──────────────────────────────────────────────

fn render_confirm_dialog(frame: &mut Frame, area: Rect, app: &App) {
    let count = app.stage.selected_count();
    let dialog = centered_rect(50, 9, area);
    frame.render_widget(Clear, dialog);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Confirm Staging ")
        .title_alignment(Alignment::Center);
    let inner = block.inner(dialog);
    frame.render_widget(block, dialog);

    let text = Paragraph::new(Text::from(vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  Stage {count} selected file{}?", if count == 1 { "" } else { "s" }),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  This will add files to the staging area.",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            "  Nothing is committed yet.",
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
        .stage
        .result_msg
        .as_ref()
        .map(|(m, ok)| (m.as_str(), *ok))
        .unwrap_or(("Done.", true));

    let dialog = centered_rect(50, 7, area);
    frame.render_widget(Clear, dialog);

    let color = if is_ok { Color::Green } else { Color::Red };
    let title = if is_ok { " Success " } else { " Error " };

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
    ]));
    frame.render_widget(text, inner);
}

// ── Helpers ──────────────────────────────────────────────────────────

/// Return a centered `Rect` of the given percentage width and fixed height.
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
