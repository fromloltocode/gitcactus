//! Rebase Portal — read-only preview screen.
//!
//! Shows what a rebase onto a target branch *would* do, without
//! executing anything. The underlying git operation is never invoked.

use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::App;
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
    render_footer(frame, vert[2]);
}

fn render_preview_banner(frame: &mut Frame, area: Rect) {
    let banner = Paragraph::new(Line::from(vec![
        Span::styled(
            "  \u{25C6} PREVIEW ONLY ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "  No changes will be made to your repository.",
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
                "  Loading preview…",
                Style::default().fg(Color::DarkGray),
            )));
            frame.render_widget(p, inner);
            return;
        }
    };

    match &preview.kind {
        PreviewKind::NotARepo => {
            render_message(
                frame,
                inner,
                "Not a git repository.",
                "Run gitcactus from inside a git repo to preview a rebase.",
                Color::Red,
            );
            return;
        }
        PreviewKind::EmptyRepo => {
            render_message(
                frame,
                inner,
                "This repository has no commits yet.",
                "There's nothing to rebase from.",
                Color::Yellow,
            );
            return;
        }
        PreviewKind::DetachedHead => {
            render_message(
                frame,
                inner,
                "You are in detached HEAD state.",
                "Check out a branch first, then try Rebase Portal again.",
                Color::Yellow,
            );
            return;
        }
        PreviewKind::SameBranch => {
            render_message(
                frame,
                inner,
                "Source and target are the same path.",
                "Pick a different target to preview a rebase.",
                Color::Yellow,
            );
            return;
        }
        PreviewKind::BranchMissing => {
            render_message(
                frame,
                inner,
                &format!("Target path '{}' was not found.", preview.target),
                "Only local branches can be previewed.",
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
            Span::styled("  (merge base)", Style::default().fg(Color::DarkGray)),
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
            "    Rebase would refuse to run until these are handled.",
            Style::default().fg(Color::DarkGray),
        )));
    }
    if preview.has_merge_commits {
        lines.push(Line::from(Span::styled(
            "  \u{26A0} Range contains merge commits.",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(Span::styled(
            "    Rebase replay may need --rebase-merges to preserve them.",
            Style::default().fg(Color::DarkGray),
        )));
    }
    if preview.dirty_tree || preview.has_merge_commits {
        lines.push(Line::from(""));
    }

    match preview.kind {
        PreviewKind::NoCommitsToReplay => {
            lines.push(Line::from(Span::styled(
                "  \u{2713} No commits to replay.",
                Style::default().fg(Color::Green),
            )));
            lines.push(Line::from(Span::styled(
                "    Source is already up to date with target.",
                Style::default().fg(Color::DarkGray),
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
                lines.push(Line::from(Span::styled(
                    format!("       by {}", c.author),
                    Style::default().fg(Color::DarkGray),
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
        Constraint::Length(6),
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
            " This is a preview. I'm not touching your repo.",
            Style::default().fg(Color::White),
        )),
        Line::from(""),
        Line::from(Span::styled(
            " Look at the commit list.",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            " That's what a rebase would replay.",
            Style::default().fg(Color::DarkGray),
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
        Line::from(Span::styled(
            " It moves your work to sit",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            " on top of a new base.",
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

fn render_footer(frame: &mut Frame, area: Rect) {
    render_help_bar(frame, area, &[
        ("\u{2191}/\u{2193}/w/s", "scroll"),
        ("Esc", "back"),
        ("q", "quit"),
    ]);
}

fn truncate_str(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}\u{2026}", &s[..max - 1])
    }
}
