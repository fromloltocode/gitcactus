//! Input handling: maps raw key events to semantic [`Action`]s.
//!
//! The event loop calls [`map_key`] to translate a `KeyCode` into an action,
//! then passes that action to [`App::handle_action`].  This keeps input
//! logic completely separate from UI rendering and state transitions.

use crossterm::event::KeyCode;

/// A user-intent action, decoupled from specific key bindings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Quit the application.
    Quit,
    /// Go back / cancel (Esc).
    Back,
    /// Move the cursor or selection up.
    MoveUp,
    /// Move the cursor or selection down.
    MoveDown,
    /// Confirm or select the current item (Enter).
    Select,
    /// Toggle a checkbox / selection (Space).
    Toggle,
    /// Toggle all items (a).
    ToggleAll,
    /// Refresh data (r).
    Refresh,
    /// Explicit yes / confirm (y).
    Confirm,
    /// Explicit no / deny (n).
    Deny,
    /// A key that doesn't map to a specific command — used for
    /// "press any key to continue" screens.
    Other,
}

/// Convert a raw [`KeyCode`] into a semantic [`Action`].
///
/// Every recognised key produces a specific variant; unrecognised keys
/// produce [`Action::Other`] so that "any key" screens still work.
pub fn map_key(code: KeyCode) -> Action {
    match code {
        KeyCode::Char('q') => Action::Quit,
        KeyCode::Esc => Action::Back,
        KeyCode::Up | KeyCode::Char('k') => Action::MoveUp,
        KeyCode::Down | KeyCode::Char('j') => Action::MoveDown,
        KeyCode::Enter => Action::Select,
        KeyCode::Char(' ') => Action::Toggle,
        KeyCode::Char('a') => Action::ToggleAll,
        KeyCode::Char('r') => Action::Refresh,
        KeyCode::Char('y') => Action::Confirm,
        KeyCode::Char('n') => Action::Deny,
        _ => Action::Other,
    }
}
