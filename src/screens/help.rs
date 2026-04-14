//! Fighting-game-inspired controls screen.
//!
//! Displays keybindings as a "Command List" with move categories,
//! combo notation, and a character portrait sidebar — like a retro
//! fighting game move list, but for Git.

use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::{App, HELP_PAGE_COUNT, HELP_PAGE_TITLES};
use crate::mascot::cactus;
use crate::screens::render_help_bar;

/// A single "move" entry in the command list.
struct Move {
    /// Key notation (fighting-game style).
    keys: &'static str,
    /// Move name (all-caps, like a fighting game).
    name: &'static str,
    /// Short description.
    desc: &'static str,
}

/// Moves for each page. Kept as functions to avoid complex statics.
fn page_basic() -> Vec<Move> {
    vec![
        Move { keys: "\u{2191} / k / w", name: "SCROLL UP", desc: "Navigate up through menus and lists" },
        Move { keys: "\u{2193} / j / s", name: "SCROLL DOWN", desc: "Navigate down through menus and lists" },
        Move { keys: "Enter", name: "STRIKE", desc: "Select the highlighted item" },
        Move { keys: "Esc", name: "GUARD", desc: "Go back to the previous screen" },
        Move { keys: "q", name: "RETREAT", desc: "Quit GitCactus entirely" },
        Move { keys: "Space", name: "MARK", desc: "Toggle selection on a file" },
    ]
}

fn page_special() -> Vec<Move> {
    vec![
        Move { keys: "a", name: "MARK ALL", desc: "Select or deselect every file at once" },
        Move { keys: "r", name: "RELOAD", desc: "Refresh repository data from disk" },
        Move { keys: "y", name: "CONFIRM", desc: "Accept a confirmation dialog" },
        Move { keys: "n", name: "DENY", desc: "Reject a confirmation dialog" },
        Move { keys: "type", name: "INSCRIBE", desc: "Type characters into the commit message" },
        Move { keys: "Bksp", name: "ERASE", desc: "Delete the last character you typed" },
    ]
}

fn page_power() -> Vec<Move> {
    vec![
        Move { keys: "Space \u{2192} Enter \u{2192} y", name: "STAGE COMBO", desc: "Select files, confirm, then stage them" },
        Move { keys: "type \u{2192} Enter \u{2192} y", name: "COMMIT COMBO", desc: "Write a message, confirm, then commit" },
        Move { keys: "Status \u{2192} Stage \u{2192} Commit", name: "FULL CHAIN", desc: "The complete Git workflow, one screen at a time" },
        Move { keys: "--version", name: "IDENTITY", desc: "Show GitCactus version from the command line" },
    ]
}

fn page_defensive() -> Vec<Move> {
    vec![
        Move { keys: "Esc", name: "BLOCK", desc: "Cancel any operation and go back safely" },
        Move { keys: "n", name: "COUNTER", desc: "Reject a dangerous confirmation" },
        Move { keys: "q", name: "FLEE", desc: "Exit the arena (quit the app)" },
        Move { keys: "(auto)", name: "SHIELD", desc: "GitCactus never changes your repo without asking" },
        Move { keys: "(auto)", name: "BARRIER", desc: "Empty commit messages are blocked" },
        Move { keys: "(auto)", name: "WARD", desc: "Staging requires explicit file selection" },
    ]
}

fn moves_for_page(page: usize) -> Vec<Move> {
    match page {
        0 => page_basic(),
        1 => page_special(),
        2 => page_power(),
        3 => page_defensive(),
        _ => vec![],
    }
}

/// Page accent colors (fighting-game style: each category has a color).
fn page_color(page: usize) -> Color {
    match page {
        0 => Color::Cyan,
        1 => Color::Green,
        2 => Color::Yellow,
        3 => Color::Red,
        _ => Color::White,
    }
}

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let outer = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(" Command List ")
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
        Constraint::Percentage(65),
        Constraint::Percentage(35),
    ])
    .split(vert[0]);

    render_move_list(frame, cols[0], app);
    render_character_panel(frame, cols[1], app);
    render_footer(frame, vert[1], app);
}

// ── Left panel: move list ────────────────────────────────────────────

fn render_move_list(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let page = app.help.page;
    let color = page_color(page);
    let title = HELP_PAGE_TITLES.get(page).unwrap_or(&"???");
    let moves = moves_for_page(page);

    let bar = "\u{2550}".repeat(title.len() + 2);
    let mut lines: Vec<Line> = vec![
        // Page title banner
        Line::from(""),
        Line::from(vec![
            Span::styled("  \u{2554}", Style::default().fg(color)),
            Span::styled(bar.clone(), Style::default().fg(color)),
            Span::styled("\u{2557}", Style::default().fg(color)),
        ]),
        Line::from(vec![
            Span::styled("  \u{2551} ", Style::default().fg(color)),
            Span::styled(
                *title,
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" \u{2551}", Style::default().fg(color)),
        ]),
        Line::from(vec![
            Span::styled("  \u{255A}", Style::default().fg(color)),
            Span::styled(bar, Style::default().fg(color)),
            Span::styled("\u{255D}", Style::default().fg(color)),
        ]),
        Line::from(""),
    ];

    // Move entries
    for m in &moves {
        // Key notation line
        lines.push(Line::from(vec![
            Span::styled("    ", Style::default()),
            Span::styled(
                format!("{:<22}", m.keys),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                m.name,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        // Description line
        lines.push(Line::from(Span::styled(
            format!    ("                          {}", m.desc),
            Style::default().fg(Color::DarkGray),
        )));
        // Separator
        lines.push(Line::from(Span::styled(
            "    \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}",
            Style::default().fg(Color::DarkGray),
        )));
    }

    // Page indicator dots
    lines.push(Line::from(""));
    let mut dots: Vec<Span> = vec![Span::raw("    ")];
    for i in 0..HELP_PAGE_COUNT {
        let dot = if i == page { "\u{25CF}" } else { "\u{25CB}" };
        let dot_color = if i == page { color } else { Color::DarkGray };
        dots.push(Span::styled(format!(" {dot} "), Style::default().fg(dot_color)));
    }
    lines.push(Line::from(dots));

    let paragraph = Paragraph::new(Text::from(lines));
    frame.render_widget(paragraph, inner);
}

// ── Right panel: character portrait + tips ───────────────────────────

fn render_character_panel(frame: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::vertical([
        Constraint::Length(1),  // padding
        Constraint::Length(3),  // character name plate
        Constraint::Length(10), // cactus art (portrait)
        Constraint::Length(1),  // spacing
        Constraint::Length(5),  // flavor text
        Constraint::Length(1),  // spacing
        Constraint::Min(3),     // tips
    ])
    .split(area);

    let page = app.help.page;
    let color = page_color(page);

    // Character name plate
    let plate = Paragraph::new(Text::from(vec![
        Line::from(Span::styled(
            " \u{2500}\u{2500}\u{2500} PLAYER \u{2500}\u{2500}\u{2500}",
            Style::default().fg(color),
        )),
        Line::from(Span::styled(
            "    CACTUS",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )),
    ]))
    .alignment(Alignment::Center);
    frame.render_widget(plate, chunks[1]);

    // Character portrait (cactus)
    let art = Paragraph::new(Text::from(cactus::small()))
        .style(Style::default().fg(color))
        .alignment(Alignment::Center);
    frame.render_widget(art, chunks[2]);

    // Flavor text per page
    let flavor = match page {
        0 => "\"Every great combo\n starts with the basics.\"",
        1 => "\"Now you're getting\n dangerous...\"",
        2 => "\"Chain your moves for\n maximum impact!\"",
        3 => "\"A true warrior knows\n when to hold back.\"",
        _ => "",
    };
    let flavor_p = Paragraph::new(Text::from(flavor))
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });
    frame.render_widget(flavor_p, chunks[4]);

    // Pro tips
    let tips = Paragraph::new(Text::from(vec![
        Line::from(Span::styled(
            " PRO TIP",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            " The full flow is:",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            " Status \u{2192} Stage \u{2192} Commit",
            Style::default().fg(Color::White),
        )),
        Line::from(Span::styled(
            " Master this chain and",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            " you'll never fear Git.",
            Style::default().fg(Color::DarkGray),
        )),
    ]))
    .wrap(Wrap { trim: true });
    frame.render_widget(tips, chunks[6]);
}

// ── Footer ───────────────────────────────────────────────────────────

fn render_footer(frame: &mut Frame, area: Rect, app: &App) {
    let page = app.help.page;
    let total = HELP_PAGE_COUNT;
    let page_label = format!("{}/{total}", page + 1);

    let page_label_ref: &str = &page_label;
    let bindings: Vec<(&str, &str)> = vec![
        ("\u{2191}/\u{2193}", "switch page"),
        ("Esc", "back"),
        ("q", "quit"),
        ("page", page_label_ref),
    ];

    render_help_bar(frame, area, &bindings);
}
