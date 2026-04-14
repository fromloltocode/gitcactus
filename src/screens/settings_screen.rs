//! In-app Settings screen for terminology mode selection.

use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::{App, SETTINGS_TERM_MODES};
use crate::mascot::cactus;
use crate::screens::render_help_bar;
use crate::terminology::TermMode;

/// Mode descriptions shown next to each option.
const MODE_DESCRIPTIONS: &[&str] = &[
    "Friendlier labels that avoid Git jargon entirely",
    "Friendly labels with Git terms in parentheses (recommended)",
    "Standard Git vocabulary, no translation",
];

/// Example row for each mode.
const MODE_EXAMPLES: &[&str] = &[
    "e.g. \"Ready to Save\", \"Checkpoint\"",
    "e.g. \"Ready to Save (Staged)\", \"Checkpoint (Commit)\"",
    "e.g. \"Staged\", \"Commit\"",
];

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let outer = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(" Settings ")
        .title_alignment(Alignment::Center);
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let vert = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(2),
    ])
    .split(inner);

    let cols = Layout::horizontal([
        Constraint::Percentage(60),
        Constraint::Percentage(40),
    ])
    .split(vert[0]);

    render_main_panel(frame, cols[0], app);
    render_side_panel(frame, cols[1], app);
    render_footer(frame, vert[1]);
}

// ── Left panel: mode selector ────────────────────────────────────────

fn render_main_panel(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let cursor = app.settings_state.cursor;
    let active_mode = app.terms.mode;

    let mut lines: Vec<Line> = vec![
        Line::from(Span::styled(
            "  Terminology Mode",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Choose how GitCactus labels Git concepts:",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
    ];

    for (i, &mode) in SETTINGS_TERM_MODES.iter().enumerate() {
        let is_cursor = i == cursor;
        let is_active = mode == active_mode;

        let label = match mode {
            TermMode::Beginner => "Beginner",
            TermMode::Hybrid => "Hybrid",
            TermMode::Git => "Git",
        };

        let marker = if is_active { "\u{25CF}" } else { "\u{25CB}" }; // ● vs ○
        let cursor_char = if is_cursor { ">" } else { " " };

        let (label_style, desc_style) = if is_cursor {
            (
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
                Style::default().fg(Color::White),
            )
        } else {
            (
                Style::default().fg(Color::Gray),
                Style::default().fg(Color::DarkGray),
            )
        };

        let active_badge = if is_active { " (active)" } else { "" };

        lines.push(Line::from(vec![
            Span::styled(
                format!("  {cursor_char} {marker} "),
                label_style,
            ),
            Span::styled(
                format!("{label}{active_badge}"),
                label_style,
            ),
        ]));

        // Description
        if let Some(desc) = MODE_DESCRIPTIONS.get(i) {
            lines.push(Line::from(Span::styled(
                format!("        {desc}"),
                desc_style,
            )));
        }

        // Example
        if let Some(example) = MODE_EXAMPLES.get(i) {
            lines.push(Line::from(Span::styled(
                format!("        {example}"),
                Style::default().fg(Color::DarkGray),
            )));
        }

        lines.push(Line::from("")); // spacing
    }

    let paragraph = Paragraph::new(Text::from(lines));
    frame.render_widget(paragraph, inner);
}

// ── Right panel: cactus + explanation ─────────────────────────────────

fn render_side_panel(frame: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::vertical([
        Constraint::Length(1),  // padding
        Constraint::Length(10), // cactus
        Constraint::Length(1),  // spacing
        Constraint::Length(5),  // tip
        Constraint::Length(1),  // spacing
        Constraint::Length(5),  // theme status
        Constraint::Length(1),  // spacing
        Constraint::Min(3),     // philosophy
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
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            " Pick whatever feels right! You can always change it later.",
            Style::default().fg(Color::White),
        )),
    ]))
    .wrap(Wrap { trim: true });
    frame.render_widget(tip, chunks[3]);

    // Theme info block — read-only for now. Edit in
    // ~/.config/gitcactus/settings under theme=<preset> / theme.*=<color>.
    let override_label = if app.theme.has_overrides() {
        " (overrides active)"
    } else {
        ""
    };
    let theme_panel = Paragraph::new(Text::from(vec![
        Line::from(Span::styled(
            " Theme",
            Style::default()
                .fg(app.theme.primary)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled(" preset: ", Style::default().fg(app.theme.muted)),
            Span::styled(
                app.theme.preset.label(),
                Style::default()
                    .fg(app.theme.highlight)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                override_label,
                Style::default().fg(app.theme.warning),
            ),
        ]),
        Line::from(Span::styled(
            " edit via ~/.config/gitcactus/settings",
            Style::default().fg(app.theme.muted),
        )),
    ]))
    .wrap(Wrap { trim: true });
    frame.render_widget(theme_panel, chunks[5]);

    let philosophy = Paragraph::new(Text::from(vec![
        Line::from(Span::styled(
            " Philosophy",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            " Git semantics stay real.",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            " Only the labels change.",
            Style::default().fg(Color::DarkGray),
        )),
    ]))
    .wrap(Wrap { trim: true });
    frame.render_widget(philosophy, chunks[7]);
}

// ── Footer ───────────────────────────────────────────────────────────

fn render_footer(frame: &mut Frame, area: Rect) {
    render_help_bar(frame, area, &[
        ("\u{2191}/\u{2193}/w/s", "move"),
        ("Enter", "apply"),
        ("Esc", "back"),
        ("q", "quit"),
    ]);
}
