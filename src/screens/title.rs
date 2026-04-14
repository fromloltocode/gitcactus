use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::mascot::cactus;
use crate::screens::render_help_bar;

/// The big ASCII title banner.
const TITLE_ART: &str = "\
  ______ _ _    _____            _
 / _____|_) |_ / ____|          | |
| |  __ _| |_| |     __ _  ___| |_ _   _ ___
| | |_ | | __| |    / _` |/ __| __| | | / __|
| |__| | | |_| |___| (_| | (__| |_| |_| \\__ \\
 \\_____|_|\\__|\\______\\__,_|\\___|\\__|\\__,_|___/";

pub fn render(frame: &mut Frame, area: Rect) {
    let outer = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(" GitCactus ")
        .title_alignment(Alignment::Center);
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    // Vertical layout: spacing, title art, tagline, cactus, prompt, footer
    let chunks = Layout::vertical([
        Constraint::Length(2), // top padding
        Constraint::Length(7), // title art
        Constraint::Length(2), // tagline
        Constraint::Min(10),   // cactus art
        Constraint::Length(2), // press any key
        Constraint::Length(2), // footer help bar
    ])
    .split(inner);

    // Title art
    let title = Paragraph::new(Text::from(TITLE_ART))
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Center);
    frame.render_widget(title, chunks[1]);

    // Tagline
    let tagline = Paragraph::new(Line::from(cactus::TITLE_TAGLINE))
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center);
    frame.render_widget(tagline, chunks[2]);

    // Cactus
    let cactus_art = Paragraph::new(Text::from(cactus::large()))
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    frame.render_widget(cactus_art, chunks[3]);

    // Prompt
    let prompt = Paragraph::new(Line::from("[ Press any key to continue ]"))
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center);
    frame.render_widget(prompt, chunks[4]);

    // Footer help bar
    render_help_bar(frame, chunks[5], &[
        ("Any key", "continue"),
        ("q", "quit"),
    ]);
}
