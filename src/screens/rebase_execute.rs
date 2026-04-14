//! Rebase execution + animation screen.
//!
//! By the time this screen is reached, the rebase has *already run* — the
//! animation is a playback of what just happened, not a fake progress bar.
//! This keeps textual status honest: the cactus and portal diagram exist
//! for flavor, but numerical progress and error details always reflect
//! the real result.

use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::{App, RebaseExecuteMode};
use crate::screens::render_help_bar;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let outer = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta))
        .title(app.terms.title_rebase_execute())
        .title_alignment(Alignment::Center);
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let vert = Layout::vertical([
        Constraint::Length(3), // header: source/target/count
        Constraint::Min(1),    // animation + commit list
        Constraint::Length(2), // footer
    ])
    .split(inner);

    render_header(frame, vert[0], app);
    render_body(frame, vert[1], app);
    render_footer(frame, vert[2], app);
}

fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    let s = &app.rebase_execute;
    let status = match &s.mode {
        RebaseExecuteMode::Animating => "Replaying commits\u{2026}",
        RebaseExecuteMode::Success => "Rebase complete",
        RebaseExecuteMode::Conflict { .. } => "Stopped at conflict",
        RebaseExecuteMode::Failure { .. } => "Rebase failed",
        RebaseExecuteMode::Aborted { .. } => "Rebase aborted",
    };
    let status_color = match &s.mode {
        RebaseExecuteMode::Animating => Color::Cyan,
        RebaseExecuteMode::Success => Color::Green,
        RebaseExecuteMode::Conflict { .. } => Color::Yellow,
        RebaseExecuteMode::Failure { .. } | RebaseExecuteMode::Aborted { ok: false, .. } => {
            Color::Red
        }
        RebaseExecuteMode::Aborted { ok: true, .. } => Color::Green,
    };

    let n = s.commits.len();
    let step_label = match &s.mode {
        RebaseExecuteMode::Animating => format!("step {}/{n}", (s.step + 1).min(n)),
        RebaseExecuteMode::Success => format!("{n}/{n}"),
        RebaseExecuteMode::Conflict { .. } => "conflict".into(),
        RebaseExecuteMode::Failure { .. } => "failed".into(),
        RebaseExecuteMode::Aborted { .. } => "aborted".into(),
    };

    let lines = vec![
        Line::from(vec![
            Span::styled("  Source: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                &s.source,
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
            Span::styled("    Target: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                &s.target,
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            ),
            Span::styled("    ", Style::default()),
            Span::styled(
                step_label,
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                format!("  Status: {status}"),
                Style::default()
                    .fg(status_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
    ];
    frame.render_widget(Paragraph::new(Text::from(lines)), area);
}

fn render_body(frame: &mut Frame, area: Rect, app: &App) {
    let cols = Layout::horizontal([
        Constraint::Percentage(60),
        Constraint::Percentage(40),
    ])
    .split(area);

    render_animation(frame, cols[0], app);
    render_commit_stream(frame, cols[1], app);
}

/// 8-bit portal animation. Grey cactus hops across grey portals.
fn render_animation(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let s = &app.rebase_execute;
    let n = s.commits.len().max(1);

    // We draw up to 8 portals so wide repos don't overflow.
    let portal_slots = 8.min(n + 1); // +1 for "landed" position past last commit
    let step = s.step.min(portal_slots.saturating_sub(1));

    // Grey 8-bit portal art (2 lines).
    let portal_lines: &[&str] = &[
        "  ╔═╗  ",
        "  ╚═╝  ",
    ];

    // Cactus (3 lines).
    let cactus_standing: &[&str] = &[
        " ▓▓ ",
        " ▓▓ ",
        " ▀▀ ",
    ];
    let cactus_fallen: &[&str] = &[
        "    ",
        "▓▓▓▓",
        "▀▀▀▀",
    ];

    // Build a grid: 3 rows of "stuff on the ground" + 2 rows of portal tops/bottoms
    let art_width: usize = portal_slots * 7;

    // Row 0: portals top
    let row_top: String = (0..portal_slots).map(|_| portal_lines[0]).collect();
    // Row 1: portals bottom
    let row_bot: String = (0..portal_slots).map(|_| portal_lines[1]).collect();

    // Cactus overlay — pad with spaces so it sits under the portal at `step`.
    let cactus_art = if s.fell { cactus_fallen } else { cactus_standing };
    let pad_chars: usize = step.saturating_mul(7) + 1;
    let mut cactus_rows: [String; 3] = [String::new(), String::new(), String::new()];
    for (i, row) in cactus_art.iter().enumerate() {
        let mut line = " ".repeat(pad_chars);
        line.push_str(row);
        if line.len() < art_width {
            line.push_str(&" ".repeat(art_width - line.len()));
        }
        cactus_rows[i] = line;
    }

    let grey = Style::default().fg(Color::DarkGray);
    let cactus_color = if s.fell { Color::Red } else { Color::DarkGray };
    let cactus_style = Style::default()
        .fg(cactus_color)
        .add_modifier(Modifier::BOLD);

    let lines: Vec<Line> = vec![
        Line::from(""),
        Line::from(Span::styled(format!("  {row_top}"), grey)),
        Line::from(Span::styled(format!("  {row_bot}"), grey)),
        Line::from(Span::styled(format!("  {}", cactus_rows[0]), cactus_style)),
        Line::from(Span::styled(format!("  {}", cactus_rows[1]), cactus_style)),
        Line::from(Span::styled(format!("  {}", cactus_rows[2]), cactus_style)),
        Line::from(""),
        Line::from(Span::styled(
            match &s.mode {
                RebaseExecuteMode::Animating => {
                    let shown = (s.step + 1).min(s.commits.len());
                    let total = s.commits.len();
                    format!("  Landing on portal {shown} of {total}\u{2026}")
                }
                RebaseExecuteMode::Success => "  All portals cleared.".into(),
                RebaseExecuteMode::Conflict { .. } => {
                    "  Fell between portals \u{2014} conflict.".into()
                }
                RebaseExecuteMode::Failure { .. } => {
                    "  Couldn't enter the portal.".into()
                }
                RebaseExecuteMode::Aborted { .. } => {
                    "  Abort complete. Returned to start.".into()
                }
            },
            Style::default().fg(Color::White),
        )),
    ];

    let p = Paragraph::new(Text::from(lines));
    frame.render_widget(p, inner);
}

fn render_commit_stream(frame: &mut Frame, area: Rect, app: &App) {
    let s = &app.rebase_execute;
    let mut lines: Vec<Line> = Vec::new();

    match &s.mode {
        RebaseExecuteMode::Conflict { stderr } => {
            lines.push(Line::from(Span::styled(
                "  Conflict \u{2014} rebase paused",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  Nothing is lost. Your branch is half-rebased and paused.",
                Style::default().fg(Color::White),
            )));
            lines.push(Line::from(""));

            // What YOU do (outside GitCactus, in your editor/terminal).
            lines.push(Line::from(Span::styled(
                "  What to do next (outside GitCactus):",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )));
            for step in &[
                "1. Open the conflicted files in your editor.",
                "2. Resolve the <<<<<<< / ======= / >>>>>>> markers.",
                "3. git add <file>   (stage each resolved file)",
            ] {
                lines.push(Line::from(Span::styled(
                    format!("   {step}"),
                    Style::default().fg(Color::DarkGray),
                )));
            }
            lines.push(Line::from(""));

            // What GitCactus can do for you right now.
            lines.push(Line::from(Span::styled(
                "  Then, here in GitCactus:",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(vec![
                Span::styled("     c", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                Span::styled("  Continue rebase (runs `git rebase --continue`)", Style::default().fg(Color::DarkGray)),
            ]));
            lines.push(Line::from(vec![
                Span::styled("     a", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                Span::styled("  Abort rebase (runs `git rebase --abort`)", Style::default().fg(Color::DarkGray)),
            ]));
            lines.push(Line::from(vec![
                Span::styled("   Esc", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::styled("  Leave this screen, keep the rebase paused", Style::default().fg(Color::DarkGray)),
            ]));
            lines.push(Line::from(""));

            // Honest note about what we don't do.
            lines.push(Line::from(Span::styled(
                "  GitCactus will not auto-resolve conflicts or auto-stage",
                Style::default().fg(Color::DarkGray),
            )));
            lines.push(Line::from(Span::styled(
                "  files. Those steps are always yours.",
                Style::default().fg(Color::DarkGray),
            )));

            if !stderr.is_empty() {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "  git output:",
                    Style::default().fg(Color::DarkGray),
                )));
                for line in stderr.lines().take(10) {
                    lines.push(Line::from(Span::styled(
                        format!("  {line}"),
                        Style::default().fg(Color::DarkGray),
                    )));
                }
            }
        }
        RebaseExecuteMode::Failure { message } => {
            lines.push(Line::from(Span::styled(
                "  Failure",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));
            for line in message.lines().take(10) {
                lines.push(Line::from(Span::styled(
                    format!("  {line}"),
                    Style::default().fg(Color::White),
                )));
            }
        }
        RebaseExecuteMode::Aborted { message, ok } => {
            lines.push(Line::from(Span::styled(
                if *ok { "  Aborted" } else { "  Abort error" },
                Style::default()
                    .fg(if *ok { Color::Green } else { Color::Red })
                    .add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("  {message}"),
                Style::default().fg(Color::White),
            )));
        }
        _ => {
            let total = s.commits.len();
            lines.push(Line::from(Span::styled(
                format!("  Commits in range ({total}):"),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));
            let highlight = s.step.min(total.saturating_sub(1));
            for (i, c) in s.commits.iter().enumerate() {
                let is_current = matches!(s.mode, RebaseExecuteMode::Animating) && i == highlight;
                let is_done = matches!(s.mode, RebaseExecuteMode::Success)
                    || (matches!(s.mode, RebaseExecuteMode::Animating) && i < highlight);
                let marker = if is_done {
                    "\u{2713}"
                } else if is_current {
                    "\u{25B6}"
                } else {
                    " "
                };
                let style = if is_current {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else if is_done {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                lines.push(Line::from(Span::styled(
                    format!("  {marker} {c}"),
                    style,
                )));
            }
        }
    }

    let p = Paragraph::new(Text::from(lines)).wrap(Wrap { trim: false });
    frame.render_widget(p, area);
}

fn render_footer(frame: &mut Frame, area: Rect, app: &App) {
    let bindings: Vec<(&str, &str)> = match &app.rebase_execute.mode {
        RebaseExecuteMode::Animating => vec![
            ("any key", "skip animation"),
            ("q", "quit"),
        ],
        RebaseExecuteMode::Success => vec![
            ("Enter", "return to branches"),
            ("Esc", "back"),
            ("q", "quit"),
        ],
        RebaseExecuteMode::Conflict { .. } => vec![
            ("c", "continue rebase"),
            ("a", "abort rebase"),
            ("Esc", "back (keep paused)"),
            ("q", "quit"),
        ],
        RebaseExecuteMode::Failure { .. } => vec![
            ("Enter", "return to branches"),
            ("Esc", "back"),
            ("q", "quit"),
        ],
        RebaseExecuteMode::Aborted { .. } => vec![
            ("any key", "return to branches"),
            ("q", "quit"),
        ],
    };
    render_help_bar(frame, area, &bindings);
}
