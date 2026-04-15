use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::{App, UpdateMode, UPDATE_ACTIONS};
use crate::mascot::cactus;
use crate::screens::render_help_bar;
use crate::update;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let outer = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(" Check for Updates ")
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
    render_footer(frame, vert[1]);

    // Overlay dialogs
    if app.update.mode == UpdateMode::ReleaseNotes {
        render_release_notes(frame, area, app);
    }
    if app.update.mode == UpdateMode::ConfirmUpdate {
        render_confirm_update(frame, area, app);
    }
    if app.update.mode == UpdateMode::Result {
        render_result_dialog(frame, area, app);
    }
}

// ── Left panel: version info + action menu ───────────────────────────

fn render_main_panel(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let info = app.update.info.as_ref();
    let current = info.map_or(update::VERSION, |i| i.current);
    let latest = info
        .map(|i| i.latest.as_str())
        .unwrap_or("checking…");
    let up_to_date = info.is_some_and(|i| i.is_up_to_date);

    let mut lines: Vec<Line> = Vec::new();

    // Version header
    lines.push(Line::from(Span::styled(
        "  Version Info",
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    // Current version
    lines.push(Line::from(vec![
        Span::styled("  Installed:  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("v{current}"),
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
        ),
    ]));

    // Latest version
    let latest_color = if up_to_date { Color::Green } else { Color::Yellow };
    lines.push(Line::from(vec![
        Span::styled("  Latest:     ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("v{latest}"),
            Style::default().fg(latest_color).add_modifier(Modifier::BOLD),
        ),
    ]));

    lines.push(Line::from(""));

    // Status badge
    let check_err = info.and_then(|i| i.error.as_deref());
    if let Some(err) = check_err {
        lines.push(Line::from(Span::styled(
            "  ! Could not check for updates",
            Style::default().fg(Color::Yellow),
        )));
        lines.push(Line::from(Span::styled(
            format!("    {err}"),
            Style::default().fg(Color::DarkGray),
        )));
    } else if up_to_date {
        lines.push(Line::from(Span::styled(
            "  ✓ You're up to date!",
            Style::default().fg(Color::Green),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            "  ↑ A newer version is available",
            Style::default().fg(Color::Yellow),
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  ── Actions ──",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(""));

    // Action menu
    for (i, label) in UPDATE_ACTIONS.iter().enumerate() {
        let is_cursor = i == app.update.cursor;
        let (prefix, style) = if is_cursor {
            (
                "> ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            ("  ", Style::default().fg(Color::Gray))
        };
        lines.push(Line::from(Span::styled(
            format!("  {prefix}{label}"),
            style,
        )));
    }

    let paragraph = Paragraph::new(Text::from(lines));
    frame.render_widget(paragraph, inner);
}

// ── Right panel: cactus + tip ────────────────────────────────────────

fn render_side_panel(frame: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::vertical([
        Constraint::Length(1),  // top padding
        Constraint::Length(10), // cactus art
        Constraint::Length(1),  // spacing
        Constraint::Length(5),  // tip
        Constraint::Length(1),  // spacing
        Constraint::Min(3),     // educational note
    ])
    .split(area);

    // Cactus art — color tracks the active theme.
    let art = Paragraph::new(Text::from(cactus::small()))
        .style(Style::default().fg(app.theme.cactus))
        .alignment(Alignment::Center);
    frame.render_widget(art, chunks[1]);

    // Contextual tip
    let up_to_date = app.update.info.as_ref().is_some_and(|i| i.is_up_to_date);
    let tip_text = cactus::update_tip(up_to_date);
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

    // Version note
    let note = Paragraph::new(Text::from(vec![
        Line::from(Span::styled(
            " About updates",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            " Updates bring new features,",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            " bug fixes, and improvements.",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            " GitCactus will never update",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            " without your permission.",
            Style::default().fg(Color::DarkGray),
        )),
    ]))
    .wrap(Wrap { trim: true });
    frame.render_widget(note, chunks[5]);
}

// ── Footer ───────────────────────────────────────────────────────────

fn render_footer(frame: &mut Frame, area: Rect) {
    render_help_bar(frame, area, &[
        ("\u{2191}/\u{2193}/w/s", "move"),
        ("Enter", "select"),
        ("Esc", "back"),
        ("q", "quit"),
    ]);
}

// ── Release notes overlay ────────────────────────────────────────────

fn render_release_notes(frame: &mut Frame, area: Rect, app: &App) {
    let dialog = centered_rect(60, 14, area);
    frame.render_widget(Clear, dialog);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Release Notes ")
        .title_alignment(Alignment::Center);
    let inner = block.inner(dialog);
    frame.render_widget(block, dialog);

    let mut lines: Vec<Line> = vec![Line::from("")];

    if let Some(info) = &app.update.info {
        for entry in &info.changelog {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  v{}", entry.version),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
            lines.push(Line::from(Span::styled(
                format!("    {}", entry.summary),
                Style::default().fg(Color::White),
            )));
            lines.push(Line::from(""));
        }
    } else {
        lines.push(Line::from(Span::styled(
            "  No release information available.",
            Style::default().fg(Color::DarkGray),
        )));
    }

    lines.push(Line::from(Span::styled(
        "  Press any key to close.",
        Style::default().fg(Color::DarkGray),
    )));

    let text = Paragraph::new(Text::from(lines));
    frame.render_widget(text, inner);
}

// ── Confirm Update ───────────────────────────────────────────────────

fn render_confirm_update(frame: &mut Frame, area: Rect, app: &App) {
    let plan = match app.update.plan.as_ref() {
        Some(p) => p,
        None => return,
    };
    let cmd = match plan.command.as_ref() {
        Some(c) => c,
        None => return, // Unsupported plans skip ConfirmUpdate entirely.
    };

    let dialog = centered_rect(68, 14, area);
    frame.render_widget(Clear, dialog);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Confirm Update ")
        .title_alignment(Alignment::Center);
    let inner = block.inner(dialog);
    frame.render_widget(block, dialog);

    let text = Paragraph::new(Text::from(vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Detected install: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                plan.kind.label(),
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  GitCactus will now run:",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(vec![
            Span::styled("    $ ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                cmd.render(),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  The TUI will hand over the terminal to the updater so you",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            "  see its real output. Press Enter when it's finished to",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            "  return to GitCactus.",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  y/Enter", Style::default().fg(Color::Green)),
            Span::styled(" run update    ", Style::default().fg(Color::DarkGray)),
            Span::styled("n/Esc", Style::default().fg(Color::Red)),
            Span::styled(" cancel", Style::default().fg(Color::DarkGray)),
        ]),
    ]));
    frame.render_widget(text, inner);
}

// ── Result (real outcome) ────────────────────────────────────────────

fn render_result_dialog(frame: &mut Frame, area: Rect, app: &App) {
    use crate::app::UpdateOutcome;

    let (title, border_color, lines): (&str, Color, Vec<Line<'_>>) = match app
        .update
        .outcome
        .as_ref()
    {
        Some(UpdateOutcome::Success { message }) => (
            " Update Complete ",
            Color::Green,
            vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  \u{2713} Update finished.",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    format!("  {message}"),
                    Style::default().fg(Color::White),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "  Press any key to close.",
                    Style::default().fg(Color::DarkGray),
                )),
            ],
        ),
        Some(UpdateOutcome::Failure { message }) => (
            " Update Failed ",
            Color::Red,
            vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  \u{2717} Update did not complete cleanly.",
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    format!("  {message}"),
                    Style::default().fg(Color::White),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "  Your existing GitCactus install is unchanged.",
                    Style::default().fg(Color::DarkGray),
                )),
                Line::from(Span::styled(
                    "  Press any key to close.",
                    Style::default().fg(Color::DarkGray),
                )),
            ],
        ),
        Some(UpdateOutcome::Unsupported) | None => {
            render_unsupported_dialog(frame, area, app);
            return;
        }
    };

    let dialog = centered_rect(68, lines.len() as u16 + 2, area);
    frame.render_widget(Clear, dialog);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(title)
        .title_alignment(Alignment::Center);
    let inner = block.inner(dialog);
    frame.render_widget(block, dialog);
    let text = Paragraph::new(Text::from(lines));
    frame.render_widget(text, inner);
}

/// Honest fallback when we can't self-update for this install kind.
fn render_unsupported_dialog(frame: &mut Frame, area: Rect, app: &App) {
    let kind_label = app
        .update
        .plan
        .as_ref()
        .map(|p| p.kind.label())
        .unwrap_or("unknown");
    let instructions = app
        .update
        .plan
        .as_ref()
        .map(|p| p.kind.manual_instructions())
        .unwrap_or_else(|| {
            vec!["See https://github.com/fromloltocode/gitcactus#install".into()]
        });

    // Size the dialog to fit the longest instruction block we carry.
    let height = (instructions.len() as u16) + 8;
    let dialog = centered_rect(70, height, area);
    frame.render_widget(Clear, dialog);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .title(" Self-Update Unavailable ")
        .title_alignment(Alignment::Center);
    let inner = block.inner(dialog);
    frame.render_widget(block, dialog);

    let mut lines: Vec<Line> = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  GitCactus can't self-update this install (",
                Style::default().fg(Color::Yellow),
            ),
            Span::styled(
                kind_label,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(").", Style::default().fg(Color::Yellow)),
        ]),
    ];
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  You can update manually:",
        Style::default().fg(Color::DarkGray),
    )));
    for l in &instructions {
        lines.push(Line::from(Span::styled(
            format!("    {l}"),
            Style::default().fg(Color::White),
        )));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Press any key to close.",
        Style::default().fg(Color::DarkGray),
    )));

    let text = Paragraph::new(Text::from(lines));
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
