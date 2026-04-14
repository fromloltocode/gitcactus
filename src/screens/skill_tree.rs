//! Prickly Protocol Tree — local progression screen.
//!
//! Retro 80s "galactica command console" flavor: terminal-native,
//! grayscale by default, subtle accent borders, ASCII frame lines.
//! Not neon sci-fi. The goal is "respectful homage," not "theme park".

use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::App;
use crate::progression::{xp_window, SkillUnlock, SKILL_NODES, UNLOCKS};
use crate::screens::render_help_bar;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let outer = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.muted))
        .title(" Prickly Protocol Tree ")
        .title_alignment(Alignment::Center);
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let vert = Layout::vertical([
        Constraint::Length(6), // command-console header
        Constraint::Min(1),
        Constraint::Length(2),
    ])
    .split(inner);

    render_header(frame, vert[0], app);

    let cols = Layout::horizontal([
        Constraint::Percentage(58),
        Constraint::Percentage(42),
    ])
    .split(vert[1]);

    render_skill_grid(frame, cols[0], app);
    render_side_panel(frame, cols[1], app);
    render_footer(frame, vert[2]);
}

fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    let p = &app.profile;
    let (cur, next) = xp_window(p.xp);
    let progress = if next > cur {
        (p.xp - cur) as f32 / (next - cur) as f32
    } else {
        1.0
    };
    let bar_width = 24usize;
    let filled = ((bar_width as f32) * progress).round() as usize;
    let empty = bar_width.saturating_sub(filled);
    let bar: String = "\u{2588}".repeat(filled) + &"\u{2591}".repeat(empty);

    // ASCII command-console frame.
    let top = "\u{2554}\u{2550}\u{2550}\u{2550} OPERATOR STATUS \u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2557}";
    let bot = "\u{255A}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{255D}";

    let primary = Style::default().fg(app.theme.primary).add_modifier(Modifier::BOLD);
    let muted = Style::default().fg(app.theme.muted);
    let highlight = Style::default().fg(app.theme.highlight).add_modifier(Modifier::BOLD);

    let xp_line = if next > cur {
        format!("{}/{} XP", p.xp - cur, next - cur)
    } else {
        "MAX".into()
    };

    let lines = vec![
        Line::from(Span::styled(top, muted)),
        Line::from(vec![
            Span::styled("\u{2551} LEVEL ", muted),
            Span::styled(format!("{:>2}", p.level), primary),
            Span::styled("   ", muted),
            Span::styled("[", muted),
            Span::styled(bar, highlight),
            Span::styled("]  ", muted),
            Span::styled(xp_line, primary),
        ]),
        Line::from(vec![
            Span::styled("\u{2551} COMMITS ", muted),
            Span::styled(format!("{}", p.stats.commits_created), primary),
            Span::styled("   STAGED ", muted),
            Span::styled(format!("{}", p.stats.files_staged), primary),
            Span::styled("   BRANCHES ", muted),
            Span::styled(format!("{}", p.stats.branches_created), primary),
            Span::styled("   SWITCH ", muted),
            Span::styled(format!("{}", p.stats.branches_switched), primary),
        ]),
        Line::from(vec![
            Span::styled("\u{2551} COMBOS ", muted),
            Span::styled(format!("{}", p.stats.combos_completed), primary),
            Span::styled("   REBASES ", muted),
            Span::styled(format!("{}", p.stats.rebases_completed), primary),
            Span::styled("   UNLOCKS ", muted),
            Span::styled(format!("{}", p.unlocks.len()), primary),
        ]),
        Line::from(Span::styled(bot, muted)),
    ];

    frame.render_widget(Paragraph::new(Text::from(lines)), area);
}

fn render_skill_grid(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(app.theme.muted));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let p = &app.profile;
    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  SKILL GRID",
        Style::default()
            .fg(app.theme.highlight)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    for (i, node) in SKILL_NODES.iter().enumerate() {
        let unlocked = node.unlock.is_unlocked(p);
        let marker = if unlocked { "\u{25A0}" } else { "\u{25A1}" }; // ■ vs □
        let title_style = if unlocked {
            Style::default()
                .fg(app.theme.success)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(app.theme.muted)
        };
        let detail_style = if unlocked {
            Style::default().fg(app.theme.highlight)
        } else {
            Style::default().fg(app.theme.muted)
        };

        // Connector line between nodes ("│").
        if i > 0 {
            lines.push(Line::from(Span::styled(
                "    \u{2502}",
                Style::default().fg(app.theme.muted),
            )));
        }
        lines.push(Line::from(vec![
            Span::styled(format!("  {} ", marker), title_style),
            Span::styled(node.title, title_style),
        ]));
        lines.push(Line::from(Span::styled(
            format!("      {}", node.detail),
            detail_style,
        )));
        if !unlocked && !matches!(node.unlock, SkillUnlock::Reserved) {
            lines.push(Line::from(Span::styled(
                format!("      \u{2022} locked: {}", node.unlock.requirement_label()),
                Style::default().fg(app.theme.warning),
            )));
        } else if matches!(node.unlock, SkillUnlock::Reserved) {
            lines.push(Line::from(Span::styled(
                "      \u{2022} reserved for a future phase",
                Style::default().fg(app.theme.muted),
            )));
        }
    }

    frame.render_widget(Paragraph::new(Text::from(lines)), inner);
}

fn render_side_panel(frame: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(3),
        Constraint::Length(1),
        Constraint::Min(1),
    ])
    .split(area);

    // Command-console-style "TRANSMISSION" subheader.
    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            "  \u{25B2} TRANSMISSION",
            Style::default()
                .fg(app.theme.primary)
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    frame.render_widget(header, chunks[1]);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(Span::styled(
        "  Unlocks earned:",
        Style::default()
            .fg(app.theme.highlight)
            .add_modifier(Modifier::BOLD),
    )));
    if app.profile.unlocks.is_empty() {
        lines.push(Line::from(Span::styled(
            "    (none yet — keep using GitCactus)",
            Style::default().fg(app.theme.muted),
        )));
    } else {
        for earned in &app.profile.unlocks {
            let label = UNLOCKS
                .iter()
                .find(|(k, _, _)| *k == earned.as_str())
                .map(|(_, l, _)| *l)
                .unwrap_or(earned.as_str());
            lines.push(Line::from(vec![
                Span::styled(
                    "    \u{2605} ",
                    Style::default().fg(app.theme.warning),
                ),
                Span::styled(
                    label,
                    Style::default()
                        .fg(app.theme.success)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Upcoming:",
        Style::default().fg(app.theme.muted),
    )));
    for (key, label, cond) in UNLOCKS {
        if !app.profile.unlocks.iter().any(|u| u == key) {
            use crate::progression::UnlockCondition as C;
            let req = match cond {
                C::Level(n) => format!("at level {n}"),
                C::CombosAtLeast(n) => format!("after {n} combos"),
                C::BranchesCreatedAtLeast(n) => format!("after {n} branches created"),
            };
            lines.push(Line::from(vec![
                Span::styled(
                    "    \u{2606} ",
                    Style::default().fg(app.theme.muted),
                ),
                Span::styled(
                    *label,
                    Style::default().fg(app.theme.muted),
                ),
                Span::styled(
                    format!(" ({req})"),
                    Style::default().fg(app.theme.muted),
                ),
            ]));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Everything here is local.",
        Style::default().fg(app.theme.muted),
    )));
    lines.push(Line::from(Span::styled(
        "  No accounts, no telemetry.",
        Style::default().fg(app.theme.muted),
    )));

    frame.render_widget(
        Paragraph::new(Text::from(lines)).wrap(Wrap { trim: false }),
        chunks[3],
    );
}

fn render_footer(frame: &mut Frame, area: Rect) {
    render_help_bar(frame, area, &[
        ("Esc", "back"),
        ("q", "quit"),
    ]);
}
