//! Commit history screen — read-only, scrollable list of past commits.

use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::App;
use crate::mascot::cactus;
use crate::screens::render_help_bar;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let outer = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(app.terms.title_history())
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

    render_commit_list(frame, cols[0], app);
    render_side_panel(frame, cols[1], app);
    render_footer(frame, vert[1]);
}

// ── Left panel: commit list ──────────────────────────────────────────

fn render_commit_list(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let hist = &app.history;

    // Handle error / empty states
    if let Some(err) = &hist.result.error {
        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                format!("  {err}"),
                Style::default().fg(Color::DarkGray),
            )),
        ];
        let p = Paragraph::new(Text::from(lines));
        frame.render_widget(p, inner);
        return;
    }

    if hist.result.entries.is_empty() {
        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                "  No commits found.",
                Style::default().fg(Color::DarkGray),
            )),
        ];
        let p = Paragraph::new(Text::from(lines));
        frame.render_widget(p, inner);
        return;
    }

    // Calculate visible window — each entry takes 3 lines
    let visible_height = inner.height as usize;
    let entry_height = 3;
    let visible_count = (visible_height / entry_height).max(1);

    // Scroll window so cursor is always visible
    let total = hist.result.entries.len();
    let scroll_start = if hist.cursor >= visible_count {
        hist.cursor - visible_count + 1
    } else {
        0
    };
    let scroll_end = total.min(scroll_start + visible_count);

    let mut lines: Vec<Line> = Vec::new();

    for i in scroll_start..scroll_end {
        let entry = &hist.result.entries[i];
        let is_cursor = i == hist.cursor;

        let cursor_char = if is_cursor { "\u{25B6}" } else { " " }; // ▶ vs space
        let hash_color = Color::Yellow;

        let (msg_style, meta_style) = if is_cursor {
            (
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
                Style::default().fg(Color::Cyan),
            )
        } else {
            (
                Style::default().fg(Color::Gray),
                Style::default().fg(Color::DarkGray),
            )
        };

        // Line 1: cursor + hash + message
        lines.push(Line::from(vec![
            Span::styled(format!("  {cursor_char} "), meta_style),
            Span::styled(&entry.short_hash, Style::default().fg(hash_color)),
            Span::styled(" ", Style::default()),
            Span::styled(
                truncate_str(&entry.summary, 50),
                msg_style,
            ),
        ]));

        // Line 2: author + time
        lines.push(Line::from(Span::styled(
            format!("        {} \u{2022} {}", entry.author, entry.relative_time),
            meta_style,
        )));

        // Line 3: separator
        if i + 1 < scroll_end {
            lines.push(Line::from(Span::styled(
                "    \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}",
                Style::default().fg(Color::DarkGray),
            )));
        } else {
            lines.push(Line::from(""));
        }
    }

    // Scroll indicator
    if total > visible_count {
        lines.push(Line::from(Span::styled(
            format!("  [{}-{} of {total}]", scroll_start + 1, scroll_end),
            Style::default().fg(Color::DarkGray),
        )));
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

    let commit_word = app.terms.commit();
    // Build tip text safely — we need owned strings for non-static labels
    let tip_line = format!(" Each entry is a {commit_word} — a saved snapshot of your project.");
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

    let history_word = app.terms.commit_history();
    let edu_title = format!(" What is {history_word}?");
    let edu = Paragraph::new(Text::from(vec![
        Line::from(Span::styled(
            edu_title,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            " It's a timeline of every",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            " change saved to your project.",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            " You can always look back to",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            " see what changed and when.",
            Style::default().fg(Color::DarkGray),
        )),
    ]))
    .wrap(Wrap { trim: true });
    frame.render_widget(edu, chunks[5]);
}

// ── Footer ───────────────────────────────────────────────────────────

fn render_footer(frame: &mut Frame, area: Rect) {
    render_help_bar(frame, area, &[
        ("\u{2191}/\u{2193}/w/s", "move"),
        ("r", "refresh"),
        ("Esc", "back"),
        ("q", "quit"),
    ]);
}

// ── Helpers ──────────────────────────────────────────────────────────

fn truncate_str(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}\u{2026}", &s[..max - 1])
    }
}
