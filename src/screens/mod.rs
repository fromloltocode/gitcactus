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

use crate::input::Action;
use crate::mouse::{ClickAction, ClickTarget};

/// Lay out a help bar's bindings and return the styled `Span`s plus
/// the column range each `(key, desc)` pair occupies.
///
/// Both `render_help_bar` and `help_bar_click_targets` go through
/// this helper so their layouts can never drift — the same column
/// math drives what the user sees and what the mouse hits.
///
/// Returns `(spans, ranges)` where `ranges[i]` is `(x, width)` for
/// binding `i` within `area`. Ranges are clamped to `area` width.
fn layout_help_bar(area: Rect, bindings: &[(&str, &str)]) -> (Vec<Span<'static>>, Vec<(u16, u16)>) {
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut ranges: Vec<(u16, u16)> = Vec::with_capacity(bindings.len());

    // The area's left edge is the starting cursor. We count *columns*
    // in terminal cells. ASCII keybinding labels (Esc, q, Enter, …)
    // occupy one cell per char; wide Unicode glyphs would need
    // `unicode_width::UnicodeWidthStr` — not yet used anywhere in
    // the existing footer strings, so a plain char count is accurate
    // here. If ever a non-ASCII footer label gets added, this helper
    // is the one place to extend.
    let mut col: u16 = area.x;
    let right_edge = area.x.saturating_add(area.width);

    for (i, (key, desc)) in bindings.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("  ", Style::default().fg(Color::DarkGray)));
            col = col.saturating_add(2);
        }

        let key_str = format!(" {key}");
        let desc_str = format!(" {desc}");
        let pair_width = (key_str.chars().count() + desc_str.chars().count()) as u16;

        // Clamp the rect to the footer area so an overflowing binding
        // still produces a sensible (if partial) hit zone.
        let start = col.min(right_edge);
        let width = pair_width.min(right_edge.saturating_sub(start));
        ranges.push((start, width));

        spans.push(Span::styled(key_str, Style::default().fg(Color::White)));
        spans.push(Span::styled(desc_str, Style::default().fg(Color::DarkGray)));
        col = col.saturating_add(pair_width);
    }

    (spans, ranges)
}

/// Render a consistent keybinding help bar at the bottom of any screen.
///
/// Each entry is `(key_label, description)`. Keys are rendered in white,
/// descriptions in dark-gray, separated by double-spaces.
pub fn render_help_bar(frame: &mut Frame, area: Rect, bindings: &[(&str, &str)]) {
    let (spans, _ranges) = layout_help_bar(area, bindings);
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

/// Return the set of clickable footer hints for a given help-bar.
///
/// `resolve` is invoked with each binding's key string; return `Some(action)`
/// for hints that should be clickable (e.g. `"q"` → `Action::Quit`), or
/// `None` for hints that are display-only (e.g. arrow-key navigation
/// hints like `"↑/↓/j/k"`).
///
/// The rects returned are flush with the given `area`'s `y` and
/// `height`, laid out left-to-right using the same column math
/// [`render_help_bar`] uses, so the user clicks the exact span of
/// characters they see.
pub fn help_bar_click_targets(
    area: Rect,
    bindings: &[(&str, &str)],
    resolve: impl Fn(&str) -> Option<Action>,
) -> Vec<ClickTarget> {
    if area.width == 0 || area.height == 0 {
        return Vec::new();
    }
    let (_spans, ranges) = layout_help_bar(area, bindings);
    bindings
        .iter()
        .zip(ranges.iter())
        .filter_map(|((key, _desc), &(x, w))| {
            if w == 0 {
                return None;
            }
            resolve(key).map(|action| ClickTarget {
                rect: Rect {
                    x,
                    y: area.y,
                    width: w,
                    height: area.height,
                },
                action: ClickAction::Fire(action),
            })
        })
        .collect()
}
