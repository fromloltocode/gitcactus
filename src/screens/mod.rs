pub mod branches;
pub mod commit;
pub mod commit_details;
pub mod diff;
pub mod help;
pub mod history;
pub mod intro;
pub mod menu;
pub mod rebase_execute;
pub mod rebase_portal;
pub mod remote_sync;
pub mod settings_screen;
pub mod skill_tree;
pub mod stage;
pub mod status;
pub mod title;
pub mod update;

use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

/// Render a consistent keybinding help bar at the bottom of any screen.
///
/// Each entry is `(key_label, description)`.  Keys are rendered in white,
/// descriptions in dark-gray, separated by double-spaces.
pub fn render_help_bar(frame: &mut Frame, area: Rect, bindings: &[(&str, &str)]) {
    let mut spans: Vec<Span> = Vec::new();
    for (i, (key, desc)) in bindings.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("  ", Style::default().fg(Color::DarkGray)));
        }
        spans.push(Span::styled(format!(" {key}"), Style::default().fg(Color::White)));
        spans.push(Span::styled(format!(" {desc}"), Style::default().fg(Color::DarkGray)));
    }
    let footer = Paragraph::new(Line::from(spans));
    frame.render_widget(footer, area);
}
