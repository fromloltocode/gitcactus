//! Rebase Portal — preview + confirm screen.
//!
//! Reading the current preview and deciding whether to proceed are both
//! non-destructive. Execution happens on [`crate::screens::rebase_execute`]
//! after the user explicitly confirms here.

use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::{App, RebasePortalMode};
use crate::git::rebase_preview::{PreviewKind, RebasePreview};
use crate::mascot::cactus;
use crate::screens::render_help_bar;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let outer = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta))
        .title(app.terms.title_rebase_portal())
        .title_alignment(Alignment::Center);
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let vert = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(1),
        Constraint::Length(2),
    ])
    .split(inner);

    render_preview_banner(frame, vert[0]);

    let cols = Layout::horizontal([
        Constraint::Percentage(65),
        Constraint::Percentage(35),
    ])
    .split(vert[1]);

    render_main_panel(frame, cols[0], app);
    render_side_panel(frame, cols[1]);
    render_footer(frame, vert[2], app);

    if app.rebase_portal.mode == RebasePortalMode::Confirm {
        render_confirm_dialog(frame, area, app);
    }
}

fn render_preview_banner(frame: &mut Frame, area: Rect) {
    let banner = Paragraph::new(Line::from(vec![
        Span::styled(
            "  \u{25C6} PREVIEW ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "  Nothing is rebased until you explicitly confirm.",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
    ]))
    .alignment(Alignment::Left);
    frame.render_widget(banner, area);
}

fn render_main_panel(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let preview = match app.rebase_portal.preview.as_ref() {
        Some(p) => p,
        None => {
            let p = Paragraph::new(Line::from(Span::styled(
                "  Loading preview\u{2026}",
                Style::default().fg(Color::DarkGray),
            )));
            frame.render_widget(p, inner);
            return;
        }
    };

    match &preview.kind {
        PreviewKind::NotARepo => {
            render_message(frame, inner, "Not a git repository.", "", Color::Red);
            return;
        }
        PreviewKind::EmptyRepo => {
            render_message(
                frame,
                inner,
                "This repository has no commits yet.",
                "Nothing to rebase.",
                Color::Yellow,
            );
            return;
        }
        PreviewKind::DetachedHead => {
            render_message(
                frame,
                inner,
                "You are in detached HEAD state.",
                "Check out a branch first, then try again.",
                Color::Yellow,
            );
            return;
        }
        PreviewKind::SameBranch => {
            render_message(
                frame,
                inner,
                "Source and target are the same path.",
                "Pick a different target.",
                Color::Yellow,
            );
            return;
        }
        PreviewKind::BranchMissing => {
            render_message(
                frame,
                inner,
                &format!("Target path '{}' was not found.", preview.target),
                "",
                Color::Red,
            );
            return;
        }
        PreviewKind::Error(msg) => {
            render_message(frame, inner, "Could not compute preview.", msg, Color::Red);
            return;
        }
        _ => {}
    }

    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  Source:  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            &preview.source,
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" @ {}", preview.source_tip),
            Style::default().fg(Color::Yellow),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  Target:  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            &preview.target,
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" @ {}", preview.target_tip),
            Style::default().fg(Color::Yellow),
        ),
    ]));
    if let Some(base) = &preview.merge_base {
        lines.push(Line::from(vec![
            Span::styled("  Common:  ", Style::default().fg(Color::DarkGray)),
            Span::styled(base.as_str(), Style::default().fg(Color::Yellow)),
        ]));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(diagram_line(preview)));
    lines.push(Line::from(""));

    if preview.dirty_tree {
        lines.push(Line::from(Span::styled(
            "  \u{26A0} Working tree has uncommitted changes.",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(Span::styled(
            "    Execution is blocked. Commit or stash first.",
            Style::default().fg(Color::DarkGray),
        )));
        lines.push(Line::from(""));
    }
    if preview.has_merge_commits {
        lines.push(Line::from(Span::styled(
            "  \u{26A0} Range contains merge commits.",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(Span::styled(
            "    Git may refuse to replay them without --rebase-merges.",
            Style::default().fg(Color::DarkGray),
        )));
        lines.push(Line::from(""));
    }

    match preview.kind {
        PreviewKind::NoCommitsToReplay => {
            lines.push(Line::from(Span::styled(
                "  \u{2713} Nothing to replay \u{2014} source is up to date.",
                Style::default().fg(Color::Green),
            )));
        }
        PreviewKind::Ready => {
            let n = preview.commits.len();
            let truncated_note = if preview.truncated { " (truncated)" } else { "" };
            lines.push(Line::from(Span::styled(
                format!("  Commits that would be replayed ({n}){truncated_note}:"),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));

            let scroll = app.rebase_portal.scroll.min(n.saturating_sub(1));
            for (i, c) in preview.commits.iter().enumerate().skip(scroll) {
                let merge_badge = if c.is_merge { " [merge]" } else { "" };
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  {:>3}. ", i + 1),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled(&c.short_hash, Style::default().fg(Color::Yellow)),
                    Span::styled(" ", Style::default()),
                    Span::styled(
                        truncate_str(&c.summary, 44),
                        Style::default().fg(Color::White),
                    ),
                    Span::styled(merge_badge, Style::default().fg(Color::Magenta)),
                ]));
            }
            lines.push(Line::from(""));
            if app.rebase_portal.can_execute() {
                lines.push(Line::from(Span::styled(
                    "  Press Enter to confirm and start the rebase.",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                )));
            } else if preview.dirty_tree {
                lines.push(Line::from(Span::styled(
                    "  Execution blocked \u{2014} clean the working tree first.",
                    Style::default().fg(Color::Yellow),
                )));
            }
        }
        _ => {}
    }

    let paragraph = Paragraph::new(Text::from(lines));
    frame.render_widget(paragraph, inner);
}

fn diagram_line(preview: &RebasePreview) -> Vec<Span<'_>> {
    vec![
        Span::styled("  ", Style::default()),
        Span::styled(
            &preview.target,
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("@{}", preview.target_tip),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled("  \u{2192}  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            "[portal]",
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  \u{2192}  ", Style::default().fg(Color::DarkGray)),
        Span::styled("(replayed commits)", Style::default().fg(Color::Cyan)),
    ]
}

fn render_message(frame: &mut Frame, area: Rect, title: &str, detail: &str, color: Color) {
    let p = Paragraph::new(Text::from(vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  {title}"),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!("  {detail}"),
            Style::default().fg(Color::DarkGray),
        )),
    ]))
    .wrap(Wrap { trim: true });
    frame.render_widget(p, area);
}

fn render_side_panel(frame: &mut Frame, area: Rect) {
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
        .style(Style::default().fg(Color::Magenta))
        .alignment(Alignment::Center);
    frame.render_widget(art, chunks[1]);

    let tip = Paragraph::new(Text::from(vec![
        Line::from(Span::styled(
            " Cactus says:",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            " Review first. I'll wait for you to confirm before anything changes.",
            Style::default().fg(Color::White),
        )),
    ]))
    .wrap(Wrap { trim: true });
    frame.render_widget(tip, chunks[3]);

    let edu = Paragraph::new(Text::from(vec![
        Line::from(Span::styled(
            " Why care?",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            " Rebase rewrites history.",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from(Span::styled(
            " Avoid on shared branches.",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
    ]))
    .wrap(Wrap { trim: true });
    frame.render_widget(edu, chunks[5]);
}

fn render_footer(frame: &mut Frame, area: Rect, app: &App) {
    let mut bindings: Vec<(&str, &str)> =
        vec![("\u{2191}/\u{2193}/w/s", "scroll")];
    if app.rebase_portal.can_execute() {
        bindings.push(("Enter", "confirm"));
    }
    bindings.push(("Esc", "back"));
    bindings.push(("q", "quit"));
    render_help_bar(frame, area, &bindings);
}

fn render_confirm_dialog(frame: &mut Frame, area: Rect, app: &App) {
    let dialog = centered_rect(64, 13, area);
    frame.render_widget(Clear, dialog);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red))
        .title(" Confirm Rebase ")
        .title_alignment(Alignment::Center);
    let inner = block.inner(dialog);
    frame.render_widget(block, dialog);

    let preview = match app.rebase_portal.preview.as_ref() {
        Some(p) => p,
        None => return,
    };
    let n = preview.commits.len();

    let text = Paragraph::new(Text::from(vec![
        Line::from(""),
        Line::from(Span::styled(
            format!(
                "  Replay {n} commit{} from '{}' onto '{}'?",
                if n == 1 { "" } else { "s" },
                preview.source,
                preview.target,
            ),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  \u{26A0} This rewrites history on your current branch.",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "    Do not rebase branches that are shared with others.",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  If anything fails mid-rebase, execution stops and you'll see",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            "  clear instructions. Nothing is force-applied.",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  y/Enter", Style::default().fg(Color::Green)),
            Span::styled(" start    ", Style::default().fg(Color::DarkGray)),
            Span::styled("n/Esc", Style::default().fg(Color::Red)),
            Span::styled(" cancel", Style::default().fg(Color::DarkGray)),
        ]),
    ]))
    .wrap(Wrap { trim: false });
    frame.render_widget(text, inner);
}

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
