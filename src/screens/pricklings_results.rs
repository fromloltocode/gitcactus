//! Post-scan results list.
//!
//! Rendered after ScanPricklings completes. Every row is a discovered
//! Prickling; users can Open (Enter), Save to the hub (s), or Rescan (r).

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
        .title(" Pricklings Found ")
        .title_alignment(Alignment::Center);
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let vert = Layout::vertical([Constraint::Min(1), Constraint::Length(2)]).split(inner);
    let cols = Layout::horizontal([Constraint::Percentage(65), Constraint::Percentage(35)])
        .split(vert[0]);

    render_list(frame, cols[0], app);
    render_side(frame, cols[1], app);
    render_footer(frame, vert[1], app);
}

fn render_list(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let res = &app.pricklings_results;
    let mut lines: Vec<Line> = Vec::new();

    if res.results.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No pricklings found in the approved roots.",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Try approving another directory in Scan Locations,",
            Style::default().fg(Color::DarkGray),
        )));
        lines.push(Line::from(Span::styled(
            "  or press 'r' to rescan.",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            format!("  {} prickling{} found", res.results.len(), if res.results.len() == 1 { "" } else { "s" }),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));

        for (i, p) in res.results.iter().enumerate() {
            let is_cursor = res.cursor == i;
            let prefix = if is_cursor { " >" } else { "  " };
            let (name_style, path_style) = if is_cursor {
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
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                    Style::default().fg(Color::DarkGray),
                )
            };
            lines.push(Line::from(vec![
                Span::styled(format!("  {prefix} "), name_style),
                Span::styled(p.display_name.clone(), name_style),
            ]));
            lines.push(Line::from(Span::styled(
                format!("        {}", p.display_path()),
                path_style,
            )));
        }
    }

    // Errors (permission denied etc.) — shown at the bottom so they
    // don't push real results off-screen.
    if !res.errors.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Skipped:",
            Style::default().fg(app.theme.warning),
        )));
        for (path, msg) in res.errors.iter().take(5) {
            lines.push(Line::from(Span::styled(
                format!("    {}  —  {msg}", path.display()),
                Style::default().fg(Color::DarkGray),
            )));
        }
        if res.errors.len() > 5 {
            lines.push(Line::from(Span::styled(
                format!("    … and {} more", res.errors.len() - 5),
                Style::default().fg(Color::DarkGray),
            )));
        }
    }

    // Transient action banner.
    if let Some((msg, ok)) = &res.result_msg {
        let color = if *ok { app.theme.success } else { app.theme.error };
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("  {msg}"),
            Style::default().fg(color),
        )));
    }

    let paragraph = Paragraph::new(Text::from(lines));
    frame.render_widget(paragraph, inner);
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

    let tip = Paragraph::new(Text::from(vec![
        Line::from(Span::styled(
            " Cactus says:",
            Style::default()
                .fg(app.theme.success)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            " Enter opens a prickling. 's' saves it to the hub so you can come back later.",
            Style::default().fg(Color::White),
        )),
    ]))
    .wrap(Wrap { trim: true });
    frame.render_widget(tip, chunks[3]);

    let explainer = Paragraph::new(Text::from(vec![
        Line::from(Span::styled(
            " Nothing has changed yet.",
            Style::default()
                .fg(app.theme.primary)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            " Discovery is read-only. Repos on",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            " this list are untouched until you",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            " open one.",
            Style::default().fg(Color::DarkGray),
        )),
    ]))
    .wrap(Wrap { trim: true });
    frame.render_widget(explainer, chunks[5]);
}

fn render_footer(frame: &mut Frame, area: Rect, app: &App) {
    let has_any = !app.pricklings_results.results.is_empty();
    let mut bindings: Vec<(&str, &str)> = Vec::new();
    if has_any {
        bindings.push(("\u{2191}/\u{2193}/w/s", "move"));
        bindings.push(("Enter", "open"));
        bindings.push(("s", "save"));
    }
    bindings.push(("r", "rescan"));
    bindings.push(("Esc", "back"));
    bindings.push(("q", "quit"));
    render_help_bar(frame, area, &bindings);
}
