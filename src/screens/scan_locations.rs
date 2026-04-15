//! Scan Locations — manage approved roots + kick off a scan.
//!
//! Keyboard model:
//!   Browse mode (default):
//!     ↑/↓ move, Enter on a root = noop, Enter on "Add path" opens
//!     input, Enter on "Scan now" triggers the scan, 'd' removes the
//!     selected root, Esc = back, q = quit.
//!   Adding mode:
//!     type a path, Enter to approve, Esc to cancel. Suggestions
//!     from `pricklings::roots::default_scan_root_candidates()` are
//!     shown underneath so the user rarely needs to type anything.
//!   Result mode:
//!     any key dismisses the result banner and returns to Browse.

use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::{
    App, ScanLocationsMode, ScanLocationsSelection, ScanLocationsState,
};
use crate::mascot::cactus;
use crate::screens::render_help_bar;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let outer = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(" Scan Locations ")
        .title_alignment(Alignment::Center);
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let vert = Layout::vertical([Constraint::Min(1), Constraint::Length(2)]).split(inner);
    let cols = Layout::horizontal([Constraint::Percentage(65), Constraint::Percentage(35)])
        .split(vert[0]);

    render_list(frame, cols[0], app);
    render_side(frame, cols[1], app);
    render_footer(frame, vert[1], &app.scan_locations);

    if app.scan_locations.mode == ScanLocationsMode::Adding {
        render_add_dialog(frame, area, app);
    }
    if app.scan_locations.mode == ScanLocationsMode::Result {
        render_result_dialog(frame, area, app);
    }
}

fn render_list(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let sl = &app.scan_locations;
    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(Span::styled(
        "  Approved roots",
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::styled(
        "  GitCactus only scans these directories.",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(""));

    if sl.roots.is_empty() {
        lines.push(Line::from(Span::styled(
            "     (none approved yet)",
            Style::default().fg(Color::DarkGray),
        )));
        lines.push(Line::from(""));
    } else {
        for (i, root) in sl.roots.iter().enumerate() {
            let is_cursor = sl.cursor == i;
            let (row_style, path_style) = if is_cursor {
                (
                    Style::default()
                        .fg(Color::Black)
                        .bg(app.theme.highlight)
                        .add_modifier(Modifier::BOLD),
                    Style::default()
                        .fg(Color::Black)
                        .bg(app.theme.highlight),
                )
            } else {
                (
                    Style::default().fg(Color::Gray),
                    Style::default().fg(Color::DarkGray),
                )
            };
            let prefix = if is_cursor { " >" } else { "  " };
            lines.push(Line::from(vec![
                Span::styled(format!("  {prefix} "), row_style),
                Span::styled(root.display().to_string(), path_style),
            ]));
        }
        lines.push(Line::from(""));
    }

    // Action rows.
    let first_action = sl.roots.len();
    let add_label = "Add path…";
    let scan_label = if sl.roots.is_empty() {
        "Scan now (no roots — add one first)"
    } else {
        "Scan now"
    };
    lines.push(action_row(sl.cursor == first_action, app, add_label));
    lines.push(action_row(sl.cursor == first_action + 1, app, scan_label));

    let paragraph = Paragraph::new(Text::from(lines));
    frame.render_widget(paragraph, inner);
}

fn action_row<'a>(is_cursor: bool, app: &App, label: &'a str) -> Line<'a> {
    let prefix = if is_cursor { " >" } else { "  " };
    let style = if is_cursor {
        Style::default()
            .fg(Color::Black)
            .bg(app.theme.highlight)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    };
    Line::from(vec![
        Span::styled(format!("  {prefix} "), style),
        Span::styled(label.to_string(), style),
    ])
}

fn render_side(frame: &mut Frame, area: Rect, app: &App) {
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
        .style(Style::default().fg(app.theme.cactus))
        .alignment(Alignment::Center);
    frame.render_widget(art, chunks[1]);

    let tip_text = tip_for(&app.scan_locations);
    let tip = Paragraph::new(Text::from(vec![
        Line::from(Span::styled(
            " Cactus says:",
            Style::default()
                .fg(app.theme.success)
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
            " Why approve roots?",
            Style::default()
                .fg(app.theme.primary)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            " GitCactus never scans your whole",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            " disk. Only the folders you approve.",
            Style::default().fg(Color::DarkGray),
        )),
    ]))
    .wrap(Wrap { trim: true });
    frame.render_widget(explainer, chunks[5]);
}

fn tip_for(sl: &ScanLocationsState) -> &'static str {
    match sl.selection() {
        ScanLocationsSelection::Root(_) => "Press 'd' to remove this root. Enter on Scan now to use it.",
        ScanLocationsSelection::AddPath => "Press Enter to type a new path. Common locations are suggested.",
        ScanLocationsSelection::ScanNow => "Press Enter to scan every approved root for Git repositories.",
    }
}

fn render_footer(frame: &mut Frame, area: Rect, sl: &ScanLocationsState) {
    match sl.mode {
        ScanLocationsMode::Browse => render_help_bar(
            frame,
            area,
            &[
                ("\u{2191}/\u{2193}/w/s", "move"),
                ("Enter", "select"),
                ("d", "remove root"),
                ("Esc", "back"),
                ("q", "quit"),
            ],
        ),
        ScanLocationsMode::Adding => render_help_bar(
            frame,
            area,
            &[
                ("type", "path"),
                ("Enter", "approve"),
                ("Esc", "cancel"),
            ],
        ),
        ScanLocationsMode::Result => render_help_bar(
            frame,
            area,
            &[("any key", "dismiss")],
        ),
    }
}

// ── Add-path overlay ─────────────────────────────────────────────────

fn render_add_dialog(frame: &mut Frame, area: Rect, app: &App) {
    let dialog = centered_rect(70, 15, area);
    frame.render_widget(Clear, dialog);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.primary))
        .title(" Add Scan Root ")
        .title_alignment(Alignment::Center);
    let inner = block.inner(dialog);
    frame.render_widget(block, dialog);

    let mut lines: Vec<Line> = vec![
        Line::from(Span::styled(
            "  Type a directory path (spaces are fine):",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  > ", Style::default().fg(app.theme.primary)),
            Span::styled(
                app.scan_locations.input_buffer.clone(),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "_",
                Style::default()
                    .fg(app.theme.primary)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
    ];

    if !app.scan_locations.suggestions.is_empty() {
        lines.push(Line::from(Span::styled(
            "  Suggestions (copy+paste or type to override):",
            Style::default().fg(Color::DarkGray),
        )));
        for s in &app.scan_locations.suggestions {
            lines.push(Line::from(Span::styled(
                format!("    • {}", s.display()),
                Style::default().fg(Color::White),
            )));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  The path must exist. It'll be canonicalised on approval.",
        Style::default().fg(Color::DarkGray),
    )));

    let body = Paragraph::new(Text::from(lines)).wrap(Wrap { trim: false });
    frame.render_widget(body, inner);
}

fn render_result_dialog(frame: &mut Frame, area: Rect, app: &App) {
    let (msg, ok) = app
        .scan_locations
        .result_msg
        .as_ref()
        .map(|(m, o)| (m.as_str(), *o))
        .unwrap_or(("Done.", true));
    let dialog = centered_rect(55, 7, area);
    frame.render_widget(Clear, dialog);
    let color = if ok { app.theme.success } else { app.theme.error };
    let title = if ok { " Done " } else { " Error " };
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
