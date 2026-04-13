use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::app::{App, MENU_ITEMS};
use crate::mascot::cactus;
use crate::screens::render_help_bar;

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

    let art = Paragraph::new(Text::from(cactus::CACTUS_SMALL))
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
        .map(|(i, (label, _))| {
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
    render_help_bar(frame, chunks[2], &[
        ("\u{2191}/\u{2193}/j/k", "navigate"),
        ("Enter", "select"),
        ("q", "quit"),
    ]);
}
