//! Tests for the Help / Controls screen state and edge cases.

use gitcactus::app::{App, Effect, HelpState, Screen, HELP_PAGE_COUNT};
use gitcactus::input::Action;

#[test]
fn help_state_starts_at_page_zero() {
    let state = HelpState::new();
    assert_eq!(state.page, 0);
}

#[test]
fn help_next_page_increments() {
    let mut state = HelpState::new();
    state.next_page();
    assert_eq!(state.page, 1);
    state.next_page();
    assert_eq!(state.page, 2);
}

#[test]
fn help_next_page_clamps_at_max() {
    let mut state = HelpState::new();
    for _ in 0..100 {
        state.next_page();
    }
    assert_eq!(state.page, HELP_PAGE_COUNT - 1);
}

#[test]
fn help_prev_page_clamps_at_zero() {
    let mut state = HelpState::new();
    state.prev_page(); // already at 0
    assert_eq!(state.page, 0);
}

#[test]
fn help_prev_page_decrements() {
    let mut state = HelpState::new();
    state.next_page();
    state.next_page();
    assert_eq!(state.page, 2);
    state.prev_page();
    assert_eq!(state.page, 1);
}

#[test]
fn help_page_round_trip() {
    let mut state = HelpState::new();
    // Go to last page and back to first
    for _ in 0..HELP_PAGE_COUNT {
        state.next_page();
    }
    assert_eq!(state.page, HELP_PAGE_COUNT - 1);
    for _ in 0..HELP_PAGE_COUNT {
        state.prev_page();
    }
    assert_eq!(state.page, 0);
}

// ── App integration tests ────────────────────────────────────────────

#[test]
fn app_help_screen_navigation() {
    let mut app = App::new();
    app.screen = Screen::Help;

    // Down moves to next page
    let effect = app.handle_action(Action::MoveDown);
    assert_eq!(effect, Effect::None);
    assert_eq!(app.help.page, 1);

    // Up moves back
    let effect = app.handle_action(Action::MoveUp);
    assert_eq!(effect, Effect::None);
    assert_eq!(app.help.page, 0);
}

#[test]
fn app_help_screen_back_returns_to_menu() {
    let mut app = App::new();
    app.screen = Screen::Help;

    let effect = app.handle_action(Action::Back);
    assert_eq!(effect, Effect::None);
    assert_eq!(app.screen, Screen::Menu);
}

#[test]
fn app_help_screen_quit() {
    let mut app = App::new();
    app.screen = Screen::Help;

    let effect = app.handle_action(Action::Quit);
    assert_eq!(effect, Effect::Quit);
}

#[test]
fn app_help_screen_select_advances_page() {
    let mut app = App::new();
    app.screen = Screen::Help;
    assert_eq!(app.help.page, 0);

    let effect = app.handle_action(Action::Select);
    assert_eq!(effect, Effect::None);
    assert_eq!(app.help.page, 1);
}

#[test]
fn app_help_screen_ignores_irrelevant_actions() {
    let mut app = App::new();
    app.screen = Screen::Help;

    // These should do nothing
    for action in [
        Action::Toggle,
        Action::ToggleAll,
        Action::Refresh,
        Action::Confirm,
        Action::Deny,
        Action::Other,
    ] {
        let effect = app.handle_action(action);
        assert_eq!(effect, Effect::None);
        assert_eq!(app.screen, Screen::Help);
        assert_eq!(app.help.page, 0);
    }
}

#[test]
fn app_menu_selects_help_and_resets_page() {
    let mut app = App::new();
    app.screen = Screen::Menu;

    // Navigate to Help (index 6 in MENU_ITEMS)
    app.menu_index = 6;
    app.help.page = 3; // simulate leftover state from previous visit

    let effect = app.handle_action(Action::Select);
    assert_eq!(effect, Effect::None);
    assert_eq!(app.screen, Screen::Help);
    assert_eq!(app.help.page, 0); // reset on entry
}

// ── Security: help screen cannot mutate repo ─────────────────────────

#[test]
fn help_screen_never_returns_destructive_effects() {
    let mut app = App::new();
    app.screen = Screen::Help;

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
        Action::Char('x'),
        Action::Backspace,
        Action::Other,
    ];

    for action in all_actions {
        app.screen = Screen::Help; // reset each time
        let effect = app.handle_action(action);
        // Help screen should only ever produce None or Quit
        assert!(
            effect == Effect::None || effect == Effect::Quit,
            "Help screen produced unexpected effect {effect:?} for action {action:?}"
        );
    }
}
