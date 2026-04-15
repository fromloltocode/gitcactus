//! Mouse click support.
//!
//! Mouse is strictly keyboard-first adjacent: clicks never invent new
//! semantics, never bypass confirmations, and never touch Git state
//! except through the existing [`Action`]/[`Effect`] pipeline.
//!
//! The pipeline for a click is:
//!
//! 1. Main loop receives a `crossterm::event::Event::Mouse`.
//! 2. `ui::click_targets(area, &app)` asks the active screen which
//!    rectangles are currently clickable and what each one does.
//! 3. [`resolve_click`] finds the first rect (searched in reverse
//!    order, so dialog overlays win) whose bounds contain the click.
//! 4. [`App::handle_click_action`] (see `app.rs`) applies any small
//!    cursor mutation the click implies, then calls `handle_action`
//!    with an existing keyboard-equivalent [`Action`].
//!
//! A click can therefore never do anything a keyboard user couldn't —
//! the semantic surface is the same, only the input shape differs.

use ratatui::layout::Rect;

use crate::input::Action;

/// A clickable region plus what to do when it's clicked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClickTarget {
    pub rect: Rect,
    pub action: ClickAction,
}

/// Narrow, exhaustive list of what a mouse click can do.
///
/// Each variant maps onto an existing keyboard flow. The `SelectXxx`
/// / `ToggleXxx` variants move the relevant list cursor to the given
/// index first, then dispatch the single [`Action`] that a keyboard
/// user would press after navigating to that row — so a click is
/// indistinguishable from "navigate + Enter".
///
/// `Fire` covers the remaining footer / dialog buttons whose effect
/// is a single plain [`Action`] (e.g. Esc → Back, q → Quit,
/// y → Confirm).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClickAction {
    /// Fire a single Action with no cursor mutation.
    Fire(Action),
    /// Set the menu cursor to this index, then fire Select.
    SelectMenu(usize),
    /// Set the Settings cursor, then fire Select.
    SelectSettings(usize),
    /// Set the stage cursor, then fire Toggle.
    ToggleStage(usize),
}

/// Hit-test `targets` against a (col, row) click position.
///
/// Iterated in reverse order: later-pushed targets (e.g. dialog
/// overlay buttons) win over earlier ones (the underlying screen).
/// Returns the matched `ClickAction` by value — variants are cheap
/// to copy.
pub fn resolve_click(targets: &[ClickTarget], col: u16, row: u16) -> Option<ClickAction> {
    targets
        .iter()
        .rev()
        .find(|t| contains(&t.rect, col, row))
        .map(|t| t.action)
}

/// `Rect::contains` isn't on ratatui's `Rect`, so we inline the
/// usual half-open-interval check. Clicks on the right / bottom
/// edge belong to the next cell over (standard TUI convention).
fn contains(r: &Rect, col: u16, row: u16) -> bool {
    col >= r.x
        && row >= r.y
        && col < r.x.saturating_add(r.width)
        && row < r.y.saturating_add(r.height)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: u16, y: u16, w: u16, h: u16) -> Rect {
        Rect { x, y, width: w, height: h }
    }

    fn tgt(r: Rect, a: ClickAction) -> ClickTarget {
        ClickTarget { rect: r, action: a }
    }

    #[test]
    fn contains_is_half_open() {
        let r = rect(2, 3, 4, 2);
        assert!(contains(&r, 2, 3));
        assert!(contains(&r, 5, 4));
        assert!(!contains(&r, 1, 3), "left edge - 1 misses");
        assert!(!contains(&r, 6, 3), "right edge is exclusive");
        assert!(!contains(&r, 2, 2), "above misses");
        assert!(!contains(&r, 2, 5), "bottom edge is exclusive");
    }

    #[test]
    fn resolve_click_returns_none_when_no_hit() {
        let targets = vec![tgt(rect(0, 0, 2, 2), ClickAction::Fire(Action::Back))];
        assert_eq!(resolve_click(&targets, 10, 10), None);
    }

    #[test]
    fn resolve_click_returns_matching_action() {
        let targets = vec![
            tgt(rect(0, 0, 5, 1), ClickAction::Fire(Action::Back)),
            tgt(rect(6, 0, 5, 1), ClickAction::Fire(Action::Quit)),
        ];
        assert_eq!(
            resolve_click(&targets, 2, 0),
            Some(ClickAction::Fire(Action::Back))
        );
        assert_eq!(
            resolve_click(&targets, 8, 0),
            Some(ClickAction::Fire(Action::Quit))
        );
    }

    #[test]
    fn later_targets_win_over_earlier_ones() {
        // Simulates a dialog overlay: menu items registered first,
        // dialog button registered second. A click in the dialog
        // area hits the dialog, not the underlying menu.
        let targets = vec![
            tgt(rect(0, 0, 20, 10), ClickAction::SelectMenu(3)),
            tgt(rect(5, 5, 5, 2), ClickAction::Fire(Action::Confirm)),
        ];
        assert_eq!(
            resolve_click(&targets, 7, 6),
            Some(ClickAction::Fire(Action::Confirm))
        );
    }
}
