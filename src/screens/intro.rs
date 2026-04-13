//! Retro boot-sequence intro animation.
//!
//! Styled after 80s/90s game console boot screens — monochrome text
//! scrolling in with a system-check vibe, then a logo reveal.
//! Plays once on first launch; skipped on subsequent launches via settings.

use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::App;

/// Boot-log lines revealed one per frame during the first phase.
const BOOT_LINES: &[&str] = &[
    "> SYSTEM CHECK .............. OK",
    "> LOADING MODULE: git ....... OK",
    "> LOADING MODULE: cactus .... OK",
    "> INITIALIZING TUI .......... OK",
    "> CHECKING REPOSITORY ....... OK",
    "> ALL SYSTEMS NOMINAL",
];

/// Number of boot-log frames.
const BOOT_FRAMES: usize = BOOT_LINES.len();

/// Compact retro logo revealed after boot.
const LOGO: &str = "\
  ______ _ _    _____            _
 / _____|_) |_ / ____|          | |
| |  __ _| |_| |     __ _  ___| |_ _   _ ___
| | |_ | | __| |    / _` |/ __| __| | | / __|
| |__| | | |_| |___| (_| | (__| |_| |_| \\__ \\
 \\_____|_|\\__|\\______\\__,_|\\___|\\__|\\__,_|___/";

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let outer = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let f = app.intro.frame;

    if f < BOOT_FRAMES {
        // Phase 1: boot log
        render_boot_phase(frame, inner, f);
    } else {
        // Phase 2: logo + PRESS START
        render_logo_phase(frame, inner, f);
    }
}

fn render_boot_phase(frame: &mut Frame, area: Rect, f: usize) {
    let chunks = Layout::vertical([
        Constraint::Length(2), // top padding
        Constraint::Min(1),    // boot log
    ])
    .split(area);

    let mut lines: Vec<Line> = Vec::new();
    for (i, text) in BOOT_LINES.iter().enumerate() {
        if i > f {
            break;
        }
        let color = if i == f { Color::White } else { Color::DarkGray };
        let style = if i == f {
            Style::default().fg(color).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(color)
        };
        lines.push(Line::from(Span::styled(format!("  {text}"), style)));
    }

    let paragraph = Paragraph::new(Text::from(lines));
    frame.render_widget(paragraph, chunks[1]);
}

fn render_logo_phase(frame: &mut Frame, area: Rect, f: usize) {
    let chunks = Layout::vertical([
        Constraint::Length(2),  // top padding
        Constraint::Length(7),  // boot log (dimmed, complete)
        Constraint::Length(2),  // spacing
        Constraint::Length(7),  // logo
        Constraint::Length(2),  // spacing
        Constraint::Min(1),     // press start / cactus hint
    ])
    .split(area);

    // Dimmed boot log
    let boot_lines: Vec<Line> = BOOT_LINES
        .iter()
        .map(|text| {
            Line::from(Span::styled(
                format!("  {text}"),
                Style::default().fg(Color::DarkGray),
            ))
        })
        .collect();
    let boot = Paragraph::new(Text::from(boot_lines));
    frame.render_widget(boot, chunks[1]);

    // Logo reveal — show lines progressively
    let logo_frame = f - BOOT_FRAMES;
    let logo_lines: Vec<&str> = LOGO.lines().collect();
    let visible = logo_lines.len().min(logo_frame + 1);
    let mut lines: Vec<Line> = Vec::new();
    for (i, text) in logo_lines.iter().take(visible).enumerate() {
        let color = if i + 1 == visible && logo_frame < logo_lines.len() {
            Color::Cyan
        } else {
            Color::White
        };
        lines.push(Line::from(Span::styled(
            *text,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        )));
    }
    let logo_p = Paragraph::new(Text::from(lines)).alignment(Alignment::Center);
    frame.render_widget(logo_p, chunks[3]);

    // "PRESS START" blink after logo is fully revealed
    if logo_frame >= logo_lines.len() {
        let blink_on = (logo_frame / 2) % 2 == 0;
        if blink_on {
            let press = Paragraph::new(Line::from(Span::styled(
                "- PRESS ANY KEY -",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )))
            .alignment(Alignment::Center);
            frame.render_widget(press, chunks[5]);
        }
    }
}
