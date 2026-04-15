//! In-app Settings screen.
//!
//! Two sections, selected with a single flat cursor:
//!   1. **Terminology mode** — Beginner / Hybrid / Git
//!   2. **Theme** — Default / Terminal Blue / Matrix / Retro Danger
//!
//! Enter on any row applies that setting and persists it to the
//! settings file. No separate "Save" button — the operation and the
//! persistence are the same action.

use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::{App, SettingsSelection, SETTINGS_THEME_PRESETS, SETTINGS_TERM_MODES};
use crate::input::Action;
use crate::mascot::cactus;
use crate::mouse::{ClickAction, ClickTarget};
use crate::screens::{help_bar_click_targets, render_help_bar};
use crate::terminology::TermMode;
use crate::theme::ThemePreset;

/// Footer bindings shared between render and click-target computation.
const FOOTER_BINDINGS: &[(&str, &str)] = &[
    ("\u{2191}/\u{2193}/w/s", "move"),
    ("Enter", "apply"),
    ("Esc", "back"),
    ("q", "quit"),
];

/// Terminology mode descriptions.
const MODE_DESCRIPTIONS: &[&str] = &[
    "Friendlier labels that avoid Git jargon entirely",
    "Friendly labels with Git terms in parentheses (recommended)",
    "Standard Git vocabulary, no translation",
];

/// Theme preset descriptions.
const THEME_DESCRIPTIONS: &[&str] = &[
    "Restrained grayscale with cyan accents",
    "Cool cyan/blue palette for a calm terminal vibe",
    "Bright green-on-black for maximum hacker energy",
    "Red + yellow arcade-warning palette, high contrast",
];

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let outer = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(" Settings ")
        .title_alignment(Alignment::Center);
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let vert = Layout::vertical([Constraint::Min(1), Constraint::Length(2)]).split(inner);

    let cols = Layout::horizontal([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(vert[0]);

    render_main_panel(frame, cols[0], app);
    render_side_panel(frame, cols[1], app);
    render_footer(frame, vert[1]);
}

// ── Left panel: terminology section + theme section ─────────────────

fn render_main_panel(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let cursor = app.settings_state.cursor;
    let active_mode = app.terms.mode;
    let active_preset = app.theme.preset;
    let theme_has_overrides = app.theme.has_overrides();

    let mut lines: Vec<Line> = Vec::new();

    // ── Terminology section ──
    lines.push(Line::from(Span::styled(
        "  Terminology Mode",
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::styled(
        "  How GitCactus labels Git concepts.",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(""));

    for (i, &mode) in SETTINGS_TERM_MODES.iter().enumerate() {
        let row_index = i;
        let is_cursor = row_index == cursor;
        let is_active = mode == active_mode;
        let label = match mode {
            TermMode::Beginner => "Beginner",
            TermMode::Hybrid => "Hybrid",
            TermMode::Git => "Git",
        };
        let desc = MODE_DESCRIPTIONS.get(i).copied().unwrap_or("");
        push_option_row(&mut lines, is_cursor, is_active, label, desc, "");
    }

    lines.push(Line::from(""));

    // ── Theme section ──
    lines.push(Line::from(Span::styled(
        "  Theme",
        Style::default()
            .fg(app.theme.primary)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::styled(
        "  Color palette used across screens.",
        Style::default().fg(Color::DarkGray),
    )));
    if theme_has_overrides {
        lines.push(Line::from(Span::styled(
            "  (custom overrides from settings file are active)",
            Style::default().fg(app.theme.warning),
        )));
    }
    lines.push(Line::from(""));

    for (i, &preset) in SETTINGS_THEME_PRESETS.iter().enumerate() {
        let row_index = SETTINGS_TERM_MODES.len() + i;
        let is_cursor = row_index == cursor;
        let is_active = preset == active_preset;
        let desc = THEME_DESCRIPTIONS.get(i).copied().unwrap_or("");
        // Tiny inline swatch so each row hints at its palette even
        // before the user selects it.
        let swatch = swatch_for(preset);
        push_option_row(&mut lines, is_cursor, is_active, preset.label(), desc, swatch);
    }

    let paragraph = Paragraph::new(Text::from(lines));
    frame.render_widget(paragraph, inner);
}

/// Push one option row (`> ● Label (active) ▪▪▪` + description line).
fn push_option_row<'a>(
    lines: &mut Vec<Line<'a>>,
    is_cursor: bool,
    is_active: bool,
    label: &'a str,
    desc: &'a str,
    swatch: &'static str,
) {
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
    let swatch_part = if swatch.is_empty() {
        String::new()
    } else {
        format!("   {swatch}")
    };

    lines.push(Line::from(vec![
        Span::styled(format!("  {cursor_char} {marker} "), label_style),
        Span::styled(format!("{label}{active_badge}"), label_style),
        Span::styled(
            swatch_part,
            Style::default().fg(Color::DarkGray),
        ),
    ]));
    if !desc.is_empty() {
        lines.push(Line::from(Span::styled(
            format!("        {desc}"),
            desc_style,
        )));
    }
    lines.push(Line::from("")); // spacing
}

/// A short text-only "swatch" so each theme row has a visual hint.
fn swatch_for(preset: ThemePreset) -> &'static str {
    match preset {
        ThemePreset::Default => "[grayscale + cyan]",
        ThemePreset::TerminalBlue => "[blue + cyan]",
        ThemePreset::Matrix => "[green on black]",
        ThemePreset::RetroDanger => "[red + yellow]",
    }
}

// ── Right panel: cactus + tip + philosophy ──────────────────────────

fn render_side_panel(frame: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::vertical([
        Constraint::Length(1),  // padding
        Constraint::Length(10), // cactus
        Constraint::Length(1),  // spacing
        Constraint::Length(5),  // tip
        Constraint::Length(1),  // spacing
        Constraint::Min(3),     // philosophy
    ])
    .split(area);

    let art = Paragraph::new(Text::from(cactus::small()))
        .style(Style::default().fg(app.theme.cactus))
        .alignment(Alignment::Center);
    frame.render_widget(art, chunks[1]);

    // Context-sensitive tip — changes depending on which section the
    // cursor is on so the side panel always reinforces what's in focus.
    let tip_text = if matches!(
        app.settings_state.selection(),
        SettingsSelection::Theme(_)
    ) {
        " Press Enter to apply this theme. Changes take effect instantly."
    } else {
        " Pick whatever feels right! You can always change it later."
    };
    let tip = Paragraph::new(Text::from(vec![
        Line::from(Span::styled(
            " Cactus says:",
            Style::default()
                .fg(app.theme.success)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            tip_text,
            Style::default().fg(Color::White),
        )),
    ]))
    .wrap(Wrap { trim: true });
    frame.render_widget(tip, chunks[3]);

    let philosophy = Paragraph::new(Text::from(vec![
        Line::from(Span::styled(
            " Philosophy",
            Style::default()
                .fg(app.theme.primary)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            " Git semantics stay real.",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            " Only the labels and colors change.",
            Style::default().fg(Color::DarkGray),
        )),
    ]))
    .wrap(Wrap { trim: true });
    frame.render_widget(philosophy, chunks[5]);
}

// ── Footer ───────────────────────────────────────────────────────────

fn render_footer(frame: &mut Frame, area: Rect) {
    render_help_bar(frame, area, FOOTER_BINDINGS);
}

// ── Mouse click targets ──────────────────────────────────────────────

/// Recompute the main panel's geometry to match the render path.
/// Kept in sync manually with [`render`] — the layout is trivial and
/// stable enough that a shared helper would add more noise than it saves.
fn layout(area: Rect) -> (Rect, Rect) {
    let outer = Block::default().borders(Borders::ALL);
    let inner = outer.inner(area);
    let vert = Layout::vertical([Constraint::Min(1), Constraint::Length(2)]).split(inner);
    let cols = Layout::horizontal([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(vert[0]);
    let main_panel = cols[0];

    // Match the `Borders::RIGHT` inset inside the main panel.
    let main_inner = Block::default().borders(Borders::RIGHT).inner(main_panel);
    (main_inner, vert[1])
}

/// Count the vertical offset inside the main panel used by each row
/// emitted by [`render_main_panel`]. Mirrored here so mouse hit
/// regions sit on the same lines as the rendered text.
fn row_line_counts() -> (Vec<u16>, Vec<u16>, u16, u16) {
    // Header rows before the terminology options:
    //   [title, subtitle, blank] = 3 lines
    let term_header = 3u16;
    // Theme header: [title, subtitle, optional override-hint, blank]
    // We can't know at layout time whether overrides are active, but
    // the click math only needs the worst-case offset — callers that
    // need precision compute it from `app.theme.has_overrides()`.
    let theme_header_base = 3u16;

    // Each option row occupies 3 display lines (row + description + blank).
    let term_rows: Vec<u16> = (0..SETTINGS_TERM_MODES.len())
        .map(|i| term_header + (i as u16) * 3)
        .collect();
    // After the terminology section: its block of rows (3 lines each)
    // + 1 blank line separator before the theme header.
    let theme_rows_start = term_header
        + SETTINGS_TERM_MODES.len() as u16 * 3
        + 1
        + theme_header_base;

    let theme_rows: Vec<u16> = (0..SETTINGS_THEME_PRESETS.len())
        .map(|i| theme_rows_start + (i as u16) * 3)
        .collect();

    (term_rows, theme_rows, term_header, theme_header_base)
}

pub fn click_targets(area: Rect, app: &App) -> Vec<ClickTarget> {
    let (main_inner, footer) = layout(area);
    let (term_rows, theme_rows, _term_header, _theme_header) = row_line_counts();

    // If overrides are active, everything in the theme section shifts
    // down by one line to accommodate the extra warning row.
    let theme_shift: u16 = if app.theme.has_overrides() { 1 } else { 0 };

    let mut out = Vec::new();

    // Terminology section rows.
    for (i, &y_off) in term_rows.iter().enumerate() {
        let row_index = i; // cursor index
        if y_off >= main_inner.height {
            break;
        }
        out.push(ClickTarget {
            rect: Rect {
                x: main_inner.x,
                y: main_inner.y.saturating_add(y_off),
                width: main_inner.width,
                height: 1,
            },
            action: ClickAction::SelectSettings(row_index),
        });
    }

    // Theme section rows.
    for (i, &y_off) in theme_rows.iter().enumerate() {
        let row_index = SETTINGS_TERM_MODES.len() + i;
        let y_off = y_off + theme_shift;
        if y_off >= main_inner.height {
            break;
        }
        out.push(ClickTarget {
            rect: Rect {
                x: main_inner.x,
                y: main_inner.y.saturating_add(y_off),
                width: main_inner.width,
                height: 1,
            },
            action: ClickAction::SelectSettings(row_index),
        });
    }

    // Footer: Enter/Esc/q clickable; arrow hint is not.
    out.extend(help_bar_click_targets(footer, FOOTER_BINDINGS, |k| match k {
        "Enter" => Some(Action::Select),
        "Esc" => Some(Action::Back),
        "q" => Some(Action::Quit),
        _ => None,
    }));

    out
}
