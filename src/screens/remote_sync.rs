//! Remote Sync screen — Phase 1 (visibility + safe fetch).
//!
//! Layout mirrors the Branches / History screens:
//! - Left panel: remote state overview (current branch, upstream,
//!   ahead/behind, list of configured remotes).
//! - Right panel: cactus mascot + plain-English tip + "remotes in one
//!   paragraph" explainer.
//! - Footer: consistent keybindings.
//!
//! Push and Pull are intentionally surfaced as *disabled* footer hints
//! so users can see where they will eventually live, without being
//! able to trigger them yet.

use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::{App, RemoteSyncMode};
use crate::git::remote::{RemoteInfo, TrackingInfo};
use crate::mascot::cactus;
use crate::screens::render_help_bar;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let outer = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(app.terms.title_remote_sync())
        .title_alignment(Alignment::Center);
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    // Body + footer
    let vert = Layout::vertical([Constraint::Min(1), Constraint::Length(2)]).split(inner);

    // Two-column body
    let cols = Layout::horizontal([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(vert[0]);

    render_main_panel(frame, cols[0], app);
    render_side_panel(frame, cols[1], app);
    render_footer(frame, vert[1], app);

    // Overlay dialogs — rendered last so they sit on top.
    if app.remote_sync.mode == RemoteSyncMode::ConfirmFetch {
        render_confirm_dialog(frame, area, app);
    }
    if app.remote_sync.mode == RemoteSyncMode::Result {
        render_result_dialog(frame, area, app);
    }
}

// ── Main panel: remote state ─────────────────────────────────────────

fn render_main_panel(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let info = &app.remote_sync.info;
    let mut lines: Vec<Line> = Vec::new();

    // Non-repo case.
    if !info.is_real {
        lines.push(Line::from(Span::styled(
            "  Not a git repository.",
            Style::default().fg(Color::DarkGray),
        )));
        if let Some(err) = &info.error {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("  {err}"),
                Style::default().fg(Color::Red),
            )));
        }
        let paragraph = Paragraph::new(Text::from(lines)).wrap(Wrap { trim: false });
        frame.render_widget(paragraph, inner);
        return;
    }

    // ── Current branch ──
    lines.push(Line::from(vec![
        Span::styled(
            format!("  {}: ", app.terms.current_branch()),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(
            info.current_branch
                .as_deref()
                .unwrap_or("(detached HEAD)"),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
    ]));

    // ── Tracking / ahead-behind ──
    lines.push(Line::from(""));
    push_tracking_block(&mut lines, info);

    // ── Configured remotes ──
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  ── Configured remotes ──",
        Style::default().fg(Color::DarkGray),
    )));

    if info.remotes.is_empty() {
        lines.push(Line::from(Span::styled(
            "     (none)",
            Style::default().fg(Color::DarkGray),
        )));
        lines.push(Line::from(Span::styled(
            "  Add one with:  git remote add origin <url>",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        for (i, remote) in info.remotes.iter().enumerate() {
            let is_cursor = i == app.remote_sync.cursor;
            let is_tracked = info
                .tracking
                .as_ref()
                .map(|t| t.remote_name == remote.name)
                .unwrap_or(false);

            let (name_style, cursor_char) = if is_cursor {
                (
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                    ">",
                )
            } else {
                (Style::default().fg(Color::White), " ")
            };

            let tracked_marker = if is_tracked { " (tracked)" } else { "" };

            lines.push(Line::from(vec![
                Span::styled(format!("  {cursor_char} "), name_style),
                Span::styled(&remote.name, name_style),
                Span::styled(
                    tracked_marker,
                    Style::default().fg(Color::Green),
                ),
            ]));
            if !remote.url.is_empty() {
                lines.push(Line::from(Span::styled(
                    format!("      {}", remote.url),
                    Style::default().fg(Color::DarkGray),
                )));
            }
        }
    }

    let paragraph = Paragraph::new(Text::from(lines)).wrap(Wrap { trim: false });
    frame.render_widget(paragraph, inner);
}

fn push_tracking_block<'a>(lines: &mut Vec<Line<'a>>, info: &RemoteInfo) {
    match (info.tracking.clone(), info.current_branch.is_some()) {
        (Some(t), _) => push_tracking(lines, t),
        (None, true) => {
            lines.push(Line::from(Span::styled(
                "  Not tracking a remote branch.",
                Style::default().fg(Color::Yellow),
            )));
            lines.push(Line::from(Span::styled(
                "  (This branch has no upstream set.)",
                Style::default().fg(Color::DarkGray),
            )));
        }
        (None, false) => {
            lines.push(Line::from(Span::styled(
                "  Detached HEAD \u{2014} no upstream to compare to.",
                Style::default().fg(Color::DarkGray),
            )));
        }
    }
}

fn push_tracking<'a>(lines: &mut Vec<Line<'a>>, t: TrackingInfo) {
    lines.push(Line::from(vec![
        Span::styled("  Tracking:      ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            t.upstream,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
    ]));

    if t.ahead_behind_unknown {
        lines.push(Line::from(Span::styled(
            "  Ahead/behind:  unknown \u{2014} try Fetch first.",
            Style::default().fg(Color::DarkGray),
        )));
    } else if t.ahead == 0 && t.behind == 0 {
        lines.push(Line::from(vec![
            Span::styled("  Ahead/behind:  ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                "up to date",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
    } else {
        let ahead_color = if t.ahead > 0 { Color::Yellow } else { Color::DarkGray };
        let behind_color = if t.behind > 0 { Color::Yellow } else { Color::DarkGray };
        lines.push(Line::from(vec![
            Span::styled("  Ahead:         ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                t.ahead.to_string(),
                Style::default().fg(ahead_color).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "   (local-only commits)",
                Style::default().fg(Color::DarkGray),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  Behind:        ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                t.behind.to_string(),
                Style::default()
                    .fg(behind_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "   (remote-only commits)",
                Style::default().fg(Color::DarkGray),
            ),
        ]));
    }
}

// ── Side panel: cactus + tip + explainer ─────────────────────────────

fn render_side_panel(frame: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::vertical([
        Constraint::Length(1),  // padding
        Constraint::Length(10), // cactus
        Constraint::Length(1),  // spacing
        Constraint::Length(5),  // tip
        Constraint::Length(1),  // spacing
        Constraint::Min(3),     // explainer
    ])
    .split(area);

    let art = Paragraph::new(Text::from(cactus::small()))
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Center);
    frame.render_widget(art, chunks[1]);

    let tip_text = remote_sync_tip(&app.remote_sync.info);
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

    let explainer = Paragraph::new(Text::from(vec![
        Line::from(Span::styled(
            " What is a remote?",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            format!(" {}", app.terms.remote_description()),
            Style::default().fg(Color::DarkGray),
        )),
    ]))
    .wrap(Wrap { trim: true });
    frame.render_widget(explainer, chunks[5]);
}

/// Pick a context-appropriate tip based on remote state.
fn remote_sync_tip(info: &RemoteInfo) -> &'static str {
    if !info.is_real {
        return "Open a folder with a git repo to see its remotes.";
    }
    if info.remotes.is_empty() {
        return "No remotes yet. Add one and Fetch will light up.";
    }
    match &info.tracking {
        None => "Your branch isn't tracking a remote yet \u{2014} that's fine for local work.",
        Some(t) if t.ahead_behind_unknown => {
            "Ahead/behind is unknown until your local copy knows about the remote \u{2014} try Fetch."
        }
        Some(t) if t.ahead == 0 && t.behind == 0 => {
            "You're in sync. Fetch anytime you want a fresh view."
        }
        Some(t) if t.behind > 0 => {
            "The remote moved ahead. Fetch is safe; pulling will come in a later release."
        }
        Some(_) => "You have local commits not on the remote. Push will come in a later release.",
    }
}

// ── Footer ───────────────────────────────────────────────────────────

fn render_footer(frame: &mut Frame, area: Rect, app: &App) {
    let has_target = app.remote_sync.fetch_target().is_some();
    let mut bindings: Vec<(&str, &str)> = vec![("\u{2191}/\u{2193}/w/s", "move")];
    if has_target {
        bindings.push(("Enter", "fetch"));
    }
    bindings.push(("r", "refresh"));
    // Surface push/pull as disabled placeholders so users know where
    // those actions will eventually live. They do nothing yet.
    bindings.push(("(P)", "push \u{2014} soon"));
    bindings.push(("(L)", "pull \u{2014} soon"));
    bindings.push(("Esc", "back"));
    bindings.push(("q", "quit"));
    render_help_bar(frame, area, &bindings);
}

// ── Confirmation dialog ──────────────────────────────────────────────

fn render_confirm_dialog(frame: &mut Frame, area: Rect, app: &App) {
    let target = app
        .remote_sync
        .fetch_target()
        .unwrap_or_else(|| "(unknown)".into());

    let dialog = centered_rect(55, 10, area);
    frame.render_widget(Clear, dialog);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Confirm Fetch ")
        .title_alignment(Alignment::Center);
    let inner = block.inner(dialog);
    frame.render_widget(block, dialog);

    let text = Paragraph::new(Text::from(vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Fetch from  ", Style::default().fg(Color::White)),
            Span::styled(
                target,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  ?", Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Fetch only updates your local view of the remote.",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            "  Your branches and working files are not changed.",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  y/Enter", Style::default().fg(Color::Green)),
            Span::styled(" confirm    ", Style::default().fg(Color::DarkGray)),
            Span::styled("n/Esc", Style::default().fg(Color::Red)),
            Span::styled(" cancel", Style::default().fg(Color::DarkGray)),
        ]),
    ]))
    .wrap(Wrap { trim: false });
    frame.render_widget(text, inner);
}

// ── Result dialog ────────────────────────────────────────────────────

fn render_result_dialog(frame: &mut Frame, area: Rect, app: &App) {
    let (msg, is_ok) = app
        .remote_sync
        .result_msg
        .as_ref()
        .map(|(m, ok)| (m.as_str(), *ok))
        .unwrap_or(("Done.", true));

    let dialog = centered_rect(60, 8, area);
    frame.render_widget(Clear, dialog);

    let color = if is_ok { Color::Green } else { Color::Red };
    let title = if is_ok { " Fetch \u{2014} Success " } else { " Fetch \u{2014} Error " };

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
    .wrap(Wrap { trim: false });
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
