use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::app::{App, MENU_ITEMS};
use crate::input::Action;
use crate::mascot::cactus;
use crate::mouse::{ClickAction, ClickTarget};
use crate::screens::{help_bar_click_targets, render_help_bar};

/// The keybindings shown in the menu's footer help bar. Shared between
/// `render` (for display) and `click_targets` (for mouse hit-testing)
/// so the two can never drift.
const FOOTER_BINDINGS: &[(&str, &str)] = &[
    ("\u{2191}/\u{2193}/j/k/w/s", "navigate"),
    ("Enter", "select"),
    ("q", "quit"),
];

/// Fully-computed menu geometry — returned by [`layout`] and consumed
/// by both the renderer and the click-target builder.
struct MenuLayout {
    items_area: Rect,
    footer_area: Rect,
}

fn layout(area: Rect) -> MenuLayout {
    // Mirror the outer block's `.borders(ALL)` inset (1 cell on every
    // side) so clicks line up with what the user sees.
    let outer = Block::default().borders(Borders::ALL);
    let inner = outer.inner(area);

    let columns = Layout::horizontal([
        Constraint::Length(22), // sidebar
        Constraint::Min(30),    // main
    ])
    .split(inner);

    let main = columns[1];
    let chunks = Layout::vertical([
        Constraint::Length(2), // header
        Constraint::Min(1),    // menu items
        Constraint::Length(2), // footer
    ])
    .split(main);

    MenuLayout {
        items_area: chunks[1],
        footer_area: chunks[2],
    }
}

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let outer = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(" GitCactus ")
        .title_alignment(Alignment::Center);
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    // Two-column layout: sidebar (cactus) | main (menu)
    let columns = Layout::horizontal([
        Constraint::Length(22), // sidebar
        Constraint::Min(30),    // main
    ])
    .split(inner);

    render_sidebar(frame, columns[0]);
    render_menu(frame, columns[1], app);
}

fn render_sidebar(frame: &mut Frame, area: Rect) {
    let block = Block::default()
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::vertical([
        Constraint::Length(1),  // padding
        Constraint::Length(10), // cactus
        Constraint::Length(1),  // padding
        Constraint::Min(1),     // tip
    ])
    .split(inner);

    let art = Paragraph::new(Text::from(cactus::small()))
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Center);
    frame.render_widget(art, chunks[1]);

    // Pick a tip (for now, just the first one — TODO: rotate)
    let tip = Paragraph::new(Text::from(cactus::TIPS[0]))
        .style(Style::default().fg(Color::DarkGray))
        .wrap(ratatui::widgets::Wrap { trim: true });
    frame.render_widget(tip, chunks[3]);
}

fn render_menu(frame: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::vertical([
        Constraint::Length(2), // header
        Constraint::Min(1),    // menu items
        Constraint::Length(2), // footer
    ])
    .split(area);

    // Header
    let header = Paragraph::new(Line::from(vec![Span::styled(
        "  Main Menu",
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    )]));
    frame.render_widget(header, chunks[0]);

    // Menu items
    let items: Vec<ListItem> = MENU_ITEMS
        .iter()
        .enumerate()
        .map(|(i, _)| {
            let label = app.terms.menu_label(i);
            let style = if i == app.menu_index {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };
            let prefix = if i == app.menu_index { " > " } else { "   " };
            ListItem::new(Line::from(Span::styled(format!("{prefix}{label}"), style)))
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, chunks[1]);

    // Footer keybind hints
    render_help_bar(frame, chunks[2], FOOTER_BINDINGS);
}

/// Click targets for the main menu.
///
/// Each visible menu row maps to a `SelectMenu(i)` — clicking a row
/// moves the cursor there and fires Select, exactly as if the user
/// had navigated with `j/k` and pressed Enter. The footer exposes
/// Enter and q as clickable hints.
pub fn click_targets(area: Rect, _app: &App) -> Vec<ClickTarget> {
    let l = layout(area);
    let mut out = Vec::with_capacity(MENU_ITEMS.len() + 2);

    // One target per visible menu row. Rows are rendered one-per-line
    // from the top of `items_area`, so the Nth row sits at y + N.
    for i in 0..MENU_ITEMS.len() {
        let y_offset = i as u16;
        if y_offset >= l.items_area.height {
            break;
        }
        out.push(ClickTarget {
            rect: Rect {
                x: l.items_area.x,
                y: l.items_area.y.saturating_add(y_offset),
                width: l.items_area.width,
                height: 1,
            },
            action: ClickAction::SelectMenu(i),
        });
    }

    // Footer: Enter / q are clickable, arrows are not.
    out.extend(help_bar_click_targets(l.footer_area, FOOTER_BINDINGS, |k| {
        match k {
            "Enter" => Some(Action::Select),
            "q" => Some(Action::Quit),
            _ => None,
        }
    }));

    out
}
