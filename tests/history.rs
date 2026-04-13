//! Tests for the commit history screen.

use gitcactus::app::{App, Effect, HistoryState, Screen};
use gitcactus::git::history::{HistoryEntry, HistoryResult};
use gitcactus::input::Action;

// ── HistoryState ─────────────────────────────────────────────────────

#[test]
fn history_state_starts_empty() {
    let s = HistoryState::new();
    assert_eq!(s.cursor, 0);
    assert!(s.result.entries.is_empty());
}

#[test]
fn history_move_up_clamps_at_zero() {
    let mut s = HistoryState::new();
    s.move_up();
    assert_eq!(s.cursor, 0);
}

#[test]
fn history_move_down_clamps_when_empty() {
    let mut s = HistoryState::new();
    s.move_down();
    assert_eq!(s.cursor, 0);
}

#[test]
fn history_navigate_with_entries() {
    let mut s = HistoryState::new();
    s.result = make_result(5);

    s.move_down();
    assert_eq!(s.cursor, 1);
    s.move_down();
    s.move_down();
    s.move_down();
    assert_eq!(s.cursor, 4);
    // Clamp at max
    s.move_down();
    assert_eq!(s.cursor, 4);

    s.move_up();
    assert_eq!(s.cursor, 3);
}

// ── App integration ──────────────────────────────────────────────────

#[test]
fn history_screen_back_returns_to_menu() {
    let mut app = App::new();
    app.screen = Screen::History;

    let effect = app.handle_action(Action::Back);
    assert_eq!(effect, Effect::None);
    assert_eq!(app.screen, Screen::Menu);
}

#[test]
fn history_screen_quit() {
    let mut app = App::new();
    app.screen = Screen::History;

    let effect = app.handle_action(Action::Quit);
    assert_eq!(effect, Effect::Quit);
}

#[test]
fn history_screen_navigation() {
    let mut app = App::new();
    app.screen = Screen::History;
    app.history.result = make_result(10);

    let effect = app.handle_action(Action::MoveDown);
    assert_eq!(effect, Effect::None);
    assert_eq!(app.history.cursor, 1);

    let effect = app.handle_action(Action::MoveUp);
    assert_eq!(effect, Effect::None);
    assert_eq!(app.history.cursor, 0);
}

#[test]
fn history_screen_refresh() {
    let mut app = App::new();
    app.screen = Screen::History;

    let effect = app.handle_action(Action::Refresh);
    assert_eq!(effect, Effect::LoadHistory);
}

#[test]
fn menu_enters_history_with_load_effect() {
    let mut app = App::new();
    app.screen = Screen::Menu;
    app.menu_index = 4; // History

    let effect = app.handle_action(Action::Select);
    assert_eq!(effect, Effect::LoadHistory);
    assert_eq!(app.screen, Screen::History);
}

// ── Security: history screen is read-only ────────────────────────────

#[test]
fn history_screen_never_returns_mutating_effects() {
    let all_actions = [
        Action::Quit,
        Action::Back,
        Action::MoveUp,
        Action::MoveDown,
        Action::Select,
        Action::Toggle,
        Action::ToggleAll,
        Action::Refresh,
        Action::Confirm,
        Action::Deny,
        Action::Preview,
        Action::Char('x'),
        Action::Backspace,
        Action::Other,
    ];

    for action in all_actions {
        let mut app = App::new();
        app.screen = Screen::History;

        let effect = app.handle_action(action);
        match effect {
            Effect::None | Effect::Quit | Effect::LoadHistory => {}
            other => panic!(
                "History screen returned mutating effect {other:?} for action {action:?}"
            ),
        }
    }
}

// ── HistoryEntry data ────────────────────────────────────────────────

#[test]
fn history_entry_fields_accessible() {
    let e = HistoryEntry {
        short_hash: "abc1234".into(),
        full_hash: "abc1234567890".into(),
        summary: "Initial commit".into(),
        author: "Test User".into(),
        timestamp: 1000000,
        relative_time: "2 days ago".into(),
    };
    assert_eq!(e.short_hash, "abc1234");
    assert_eq!(e.author, "Test User");
    assert!(!e.relative_time.is_empty());
}

// ── Helper ───────────────────────────────────────────────────────────

fn make_result(n: usize) -> HistoryResult {
    let entries: Vec<HistoryEntry> = (0..n)
        .map(|i| HistoryEntry {
            short_hash: format!("{i:07}"),
            full_hash: format!("{i:040}"),
            summary: format!("Commit message {i}"),
            author: "Test".into(),
            timestamp: 1000000 - (i as i64 * 3600),
            relative_time: format!("{i} hours ago"),
        })
        .collect();
    HistoryResult {
        entries,
        is_real: true,
        error: None,
    }
}
