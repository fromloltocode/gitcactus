//! Read-only diff preview screen.
//!
//! Shows a syntax-colored diff for a single file. Never mutates the repo.

use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::App;
use crate::git::diff::DiffKind;
use crate::mascot::cactus;
use crate::screens::render_help_bar;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let outer = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(app.terms.title_diff())
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
        Constraint::Percentage(70),
        Constraint::Percentage(30),
    ])
    .split(vert[0]);

    render_diff_panel(frame, cols[0], app);
    render_side_panel(frame, cols[1]);
    render_footer(frame, vert[1], app);
}

// ── Left panel: diff content ─────────────────────────────────────────

fn render_diff_panel(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let diff = &app.diff;
    let mut lines: Vec<Line> = Vec::new();

    // File header
    let kind_label = match &diff.result.kind {
        DiffKind::Modified => "modified",
        DiffKind::NewFile => "new file",
        DiffKind::Deleted => "deleted",
        DiffKind::Renamed => "renamed",
        DiffKind::Binary => "binary",
        DiffKind::Empty => "no changes",
        DiffKind::Error(_) => "error",
    };
    lines.push(Line::from(vec![
        Span::styled("  File: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            &diff.result.file_path,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("  ({kind_label})"),
            Style::default().fg(Color::DarkGray),
        ),
    ]));
    lines.push(Line::from(Span::styled(
        "  \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}",
        Style::default().fg(Color::DarkGray),
    )));

    // Handle special cases
    match &diff.result.kind {
        DiffKind::Binary => {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  Binary file — diff preview not available.",
                Style::default().fg(Color::Yellow),
            )));
        }
        DiffKind::Empty => {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  No changes detected for this file.",
                Style::default().fg(Color::DarkGray),
            )));
        }
        DiffKind::Error(msg) => {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("  Error: {msg}"),
                Style::default().fg(Color::Red),
            )));
        }
        _ => {
            // Render diff lines with scroll offset
            let visible_height = inner.height.saturating_sub(4) as usize; // header takes ~3 lines
            let total = diff.result.lines.len();
            let start = diff.scroll.min(total.saturating_sub(1));
            let end = total.min(start + visible_height);

            for dl in &diff.result.lines[start..end] {
                let (prefix, color) = match dl.origin {
                    '+' => ("+", Color::Green),
                    '-' => ("-", Color::Red),
                    'H' => ("@", Color::Cyan),
                    _ => (" ", Color::White),
                };
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  {prefix} "),
                        Style::default().fg(color),
                    ),
                    Span::styled(&dl.content, Style::default().fg(color)),
                ]));
            }

            if diff.result.truncated && end >= total {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "  ... diff truncated (file too large) ...",
                    Style::default().fg(Color::Yellow),
                )));
            }

            // Scroll indicator
            if total > visible_height {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    format!(
                        "  line {}-{} of {total}",
                        start + 1,
                        end.min(total)
                    ),
                    Style::default().fg(Color::DarkGray),
                )));
            }
        }
    }

    let paragraph = Paragraph::new(Text::from(lines));
    frame.render_widget(paragraph, inner);
}

// ── Right panel: cactus + explanation ─────────────────────────────────

fn render_side_panel(frame: &mut Frame, area: Rect) {
    let chunks = Layout::vertical([
        Constraint::Length(1),  // padding
        Constraint::Length(10), // cactus
        Constraint::Length(1),  // spacing
        Constraint::Length(5),  // tip
        Constraint::Length(1),  // spacing
        Constraint::Min(3),     // educational note
    ])
    .split(area);

    let art = Paragraph::new(Text::from(cactus::CACTUS_SMALL))
        .style(Style::default().fg(Color::White))
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
            " This is read-only! You're just looking at what changed.",
            Style::default().fg(Color::White),
        )),
    ]))
    .wrap(Wrap { trim: true });
    frame.render_widget(tip, chunks[3]);

    let edu = Paragraph::new(Text::from(vec![
        Line::from(Span::styled(
            " What is a diff?",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            " A diff shows exactly what",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            " lines were added (+) or",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            " removed (-) in a file.",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            " Review before you stage!",
            Style::default().fg(Color::DarkGray),
        )),
    ]))
    .wrap(Wrap { trim: true });
    frame.render_widget(edu, chunks[5]);
}

// ── Footer ───────────────────────────────────────────────────────────

fn render_footer(frame: &mut Frame, area: Rect, app: &App) {
    let total = app.diff.result.lines.len();
    let has_scroll = total > 0;

    if has_scroll {
        render_help_bar(frame, area, &[
            ("\u{2191}/\u{2193}/w/s", "scroll"),
            ("Esc", "back"),
            ("q", "quit"),
        ]);
    } else {
        render_help_bar(frame, area, &[
            ("Esc", "back"),
            ("q", "quit"),
        ]);
    }
}
