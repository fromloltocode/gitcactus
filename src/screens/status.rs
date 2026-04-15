use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::git::status::RepoStatus;
use crate::mascot::cactus;
use crate::screens::render_help_bar;
use crate::terminology::Terms;
use crate::theme::Theme;

/// Render the status screen with a two-column layout:
///   Left  — file status grouped by category
///   Right — cactus mascot + contextual tip + educational note
///   Bottom — keybinding footer
pub fn render(frame: &mut Frame, area: Rect, repo: &RepoStatus, terms: &Terms, theme: &Theme) {
    let outer = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(terms.title_status())
        .title_alignment(Alignment::Center);
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    // Top area + footer
    let vert = Layout::vertical([
        Constraint::Min(1),       // body
        Constraint::Length(2),    // footer
    ])
    .split(inner);

    // Two-column body
    let cols = Layout::horizontal([
        Constraint::Percentage(60), // main panel
        Constraint::Percentage(40), // side panel
    ])
    .split(vert[0]);

    render_main_panel(frame, cols[0], repo, terms);
    render_side_panel(frame, cols[1], repo, theme);
    render_footer(frame, vert[1]);
}

// ── Main panel: branch + grouped file lists ──────────────────────────

fn render_main_panel(frame: &mut Frame, area: Rect, repo: &RepoStatus, terms: &Terms) {
    let block = Block::default()
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();

    // Branch header
    lines.push(Line::from(vec![
        Span::styled(format!("  {}: ", terms.current_branch()), Style::default().fg(Color::DarkGray)),
        Span::styled(
            &repo.branch,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    lines.push(Line::from(""));

    if !repo.is_real {
        lines.push(Line::from(Span::styled(
            "  Could not read a git repository in the current directory.",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        // Cap file lists for display (data layer provides all files).
        const LIMIT: usize = 10;

        // Staged
        push_group(
            &mut lines,
            terms.staged(),
            "*",
            Color::Green,
            repo.staged,
            &repo.staged_files[..repo.staged_files.len().min(LIMIT)],
        );

        // Modified
        push_group(
            &mut lines,
            terms.modified(),
            "~",
            Color::Yellow,
            repo.modified,
            &repo.modified_files[..repo.modified_files.len().min(LIMIT)],
        );

        // Untracked
        push_group(
            &mut lines,
            terms.untracked(),
            "?",
            Color::Red,
            repo.untracked,
            &repo.untracked_files[..repo.untracked_files.len().min(LIMIT)],
        );

        // Clean-repo note
        let total = repo.staged.unwrap_or(0)
            + repo.modified.unwrap_or(0)
            + repo.untracked.unwrap_or(0);
        if total == 0 {
            lines.push(Line::from(Span::styled(
                "  Nothing to report — working tree clean.",
                Style::default().fg(Color::DarkGray),
            )));
        }
    }

    let paragraph = Paragraph::new(Text::from(lines));
    frame.render_widget(paragraph, inner);
}

/// Push a section header + file list for one status group.
fn push_group<'a>(
    lines: &mut Vec<Line<'a>>,
    label: &str,
    symbol: &str,
    color: Color,
    count: Option<usize>,
    files: &'a [String],
) {
    let n = count.unwrap_or(0);
    // Section header: "  ── Staged (3) ──"
    lines.push(Line::from(vec![
        Span::styled("  ── ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{label} ({n})"),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" ──", Style::default().fg(Color::DarkGray)),
    ]));

    if n == 0 {
        lines.push(Line::from(Span::styled(
            "     (none)",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        for name in files {
            lines.push(Line::from(vec![
                Span::styled(format!("   {symbol} "), Style::default().fg(color)),
                Span::styled(name.as_str(), Style::default().fg(Color::White)),
            ]));
        }
        if n > files.len() {
            lines.push(Line::from(Span::styled(
                format!("     … and {} more", n - files.len()),
                Style::default().fg(Color::DarkGray),
            )));
        }
    }

    lines.push(Line::from("")); // spacing between groups
}

// ── Side panel: cactus + tip + educational note ──────────────────────

fn render_side_panel(frame: &mut Frame, area: Rect, repo: &RepoStatus, theme: &Theme) {
    let chunks = Layout::vertical([
        Constraint::Length(1),  // top padding
        Constraint::Length(10), // cactus art
        Constraint::Length(1),  // spacing
        Constraint::Length(4),  // contextual tip
        Constraint::Length(1),  // spacing
        Constraint::Min(3),     // educational note
    ])
    .split(area);

    // Cactus art — color tracks the active theme.
    let art = Paragraph::new(Text::from(cactus::small()))
        .style(Style::default().fg(theme.cactus))
        .alignment(Alignment::Center);
    frame.render_widget(art, chunks[1]);

    // Contextual tip
    let tip_text = cactus::status_tip(repo.staged, repo.modified, repo.untracked);
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
            " What is staging?",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            " Staging lets you choose exactly",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            " which changes go into your next",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            " commit. Think of it as packing a",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            " box before shipping it.",
            Style::default().fg(Color::DarkGray),
        )),
    ]))
    .wrap(Wrap { trim: true });
    frame.render_widget(edu, chunks[5]);
}

// ── Footer ───────────────────────────────────────────────────────────

fn render_footer(frame: &mut Frame, area: Rect) {
    render_help_bar(frame, area, &[
        ("Esc", "back"),
        ("q", "quit"),
    ]);
}

/// Render a generic placeholder screen for features not yet implemented.
///
/// Currently unused — every screen has a real renderer — but kept
/// available for any future not-yet-built screens to re-use.
#[allow(dead_code)]
pub fn render_placeholder(frame: &mut Frame, area: Rect, name: &str) {
    let outer = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(format!(" {name} "))
        .title_alignment(Alignment::Center);
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let chunks = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Min(1),
        Constraint::Length(2),
    ])
    .split(inner);

    let text = Paragraph::new(Line::from(Span::styled(
        format!("[ {name} — coming soon ]"),
        Style::default().fg(Color::DarkGray),
    )))
    .alignment(Alignment::Center);
    frame.render_widget(text, chunks[1]);

    let sub = Paragraph::new(Line::from(Span::styled(
        "This screen is a placeholder. Implementation is planned for a future release.",
        Style::default().fg(Color::DarkGray),
    )))
    .alignment(Alignment::Center);
    frame.render_widget(sub, chunks[2]);

    render_help_bar(frame, chunks[4], &[
        ("Esc", "back"),
        ("q", "quit"),
    ]);
}
