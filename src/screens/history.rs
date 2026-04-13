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
    render_footer(frame, vert[1], app);
}

// ── Left panel: commit list ──────────────────────────────────────────

fn render_commit_list(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let hist = &app.history;

    // Handle error state
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

    let visible = hist.visible_indices();
    let total_all = hist.result.entries.len();
    let total_visible = visible.len();

    // Build the top header: search bar or filter-active banner.
    let mut lines: Vec<Line> = Vec::new();
    if hist.search_mode {
        lines.push(Line::from(vec![
            Span::styled(
                "  / ",
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                if hist.filter.is_empty() {
                    "\u{2588}".to_string() // block cursor
                } else {
                    format!("{}\u{2588}", hist.filter)
                },
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(
                    "   ({total_visible}/{total_all} match{})",
                    if total_visible == 1 { "" } else { "es" }
                ),
                Style::default().fg(Color::DarkGray),
            ),
        ]));
        lines.push(Line::from(""));
    } else if !hist.filter.is_empty() {
        lines.push(Line::from(vec![
            Span::styled(
                "  Filter: ",
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                &hist.filter,
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(
                    "   ({total_visible}/{total_all})   Esc to clear, / to edit",
                    total_visible = total_visible,
                    total_all = total_all
                ),
                Style::default().fg(Color::DarkGray),
            ),
        ]));
        lines.push(Line::from(""));
    }

    // No filtered results: friendly message.
    if total_visible == 0 {
        lines.push(Line::from(Span::styled(
            "  No commits match this filter.",
            Style::default().fg(Color::Yellow),
        )));
        lines.push(Line::from(Span::styled(
            "  Edit the filter or press Esc to clear it.",
            Style::default().fg(Color::DarkGray),
        )));
        let p = Paragraph::new(Text::from(lines));
        frame.render_widget(p, inner);
        return;
    }

    // Compute scroll window in *visible* space.
    let entry_height = 3;
    let header_lines = lines.len() as u16;
    let body_height = inner.height.saturating_sub(header_lines) as usize;
    let visible_count = (body_height / entry_height).max(1);
    let scroll_start = if hist.cursor >= visible_count {
        hist.cursor - visible_count + 1
    } else {
        0
    };
    let scroll_end = total_visible.min(scroll_start + visible_count);

    for (vi, &real_idx) in visible.iter().enumerate().take(scroll_end).skip(scroll_start) {
        let entry = &hist.result.entries[real_idx];
        let is_cursor = vi == hist.cursor;

        let cursor_char = if is_cursor { "\u{25B6}" } else { " " };
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

        lines.push(Line::from(vec![
            Span::styled(format!("  {cursor_char} "), meta_style),
            Span::styled(&entry.short_hash, Style::default().fg(hash_color)),
            Span::styled(" ", Style::default()),
            Span::styled(truncate_str(&entry.summary, 50), msg_style),
        ]));

        lines.push(Line::from(Span::styled(
            format!("        {} \u{2022} {}", entry.author, entry.relative_time),
            meta_style,
        )));

        if vi + 1 < scroll_end {
            lines.push(Line::from(Span::styled(
                "    \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}",
                Style::default().fg(Color::DarkGray),
            )));
        } else {
            lines.push(Line::from(""));
        }
    }

    // Scroll indicator
    if total_visible > visible_count {
        lines.push(Line::from(Span::styled(
            format!(
                "  [{}-{} of {total_visible}]",
                scroll_start + 1,
                scroll_end
            ),
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
    let tip_line = if app.history.search_mode || !app.history.filter.is_empty() {
        format!(" Type to filter your {commit_word}s by message, author, or hash.")
    } else {
        format!(" Each entry is a {commit_word}. Press / to search the list.")
    };
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

fn render_footer(frame: &mut Frame, area: Rect, app: &App) {
    if app.history.search_mode {
        render_help_bar(frame, area, &[
            ("type", "filter"),
            ("Backspace", "erase"),
            ("Enter", "apply"),
            ("Esc", "cancel"),
        ]);
    } else {
        render_help_bar(frame, area, &[
            ("\u{2191}/\u{2193}/w/s", "move"),
            ("Enter", "details"),
            ("d", "diff"),
            ("/", "search"),
            ("r", "refresh"),
            ("Esc", "back"),
            ("q", "quit"),
        ]);
    }
}

// ── Helpers ──────────────────────────────────────────────────────────

fn truncate_str(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}\u{2026}", &s[..max - 1])
    }
}
