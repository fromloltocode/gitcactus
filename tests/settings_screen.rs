//! Tests for the in-app Settings screen.

use gitcactus::app::{App, Effect, Screen, SettingsState, SETTINGS_TERM_MODES};
use gitcactus::input::Action;
use gitcactus::terminology::TermMode;

// ── SettingsState ────────────────────────────────────────────────────

#[test]
fn settings_state_starts_at_zero() {
    let s = SettingsState::new();
    assert_eq!(s.cursor, 0);
}

#[test]
fn settings_from_active_matches_mode() {
    let s = SettingsState::from_active(TermMode::Hybrid);
    assert_eq!(s.selected_mode(), TermMode::Hybrid);

    let s = SettingsState::from_active(TermMode::Git);
    assert_eq!(s.selected_mode(), TermMode::Git);

    let s = SettingsState::from_active(TermMode::Beginner);
    assert_eq!(s.selected_mode(), TermMode::Beginner);
}

#[test]
fn settings_move_up_clamps_at_zero() {
    let mut s = SettingsState::new();
    s.move_up();
    assert_eq!(s.cursor, 0);
}

#[test]
fn settings_move_down_clamps_at_max() {
    let mut s = SettingsState::new();
    for _ in 0..100 {
        s.move_down();
    }
    assert_eq!(s.cursor, SETTINGS_TERM_MODES.len() - 1);
}

#[test]
fn settings_navigate_all_modes() {
    let mut s = SettingsState::new();
    assert_eq!(s.selected_mode(), TermMode::Beginner);
    s.move_down();
    assert_eq!(s.selected_mode(), TermMode::Hybrid);
    s.move_down();
    assert_eq!(s.selected_mode(), TermMode::Git);
    s.move_up();
    assert_eq!(s.selected_mode(), TermMode::Hybrid);
}

// ── App integration ──────────────────────────────────────────────────

#[test]
fn settings_screen_back_returns_to_menu() {
    let mut app = App::new();
    app.screen = Screen::Settings;

    let effect = app.handle_action(Action::Back);
    assert_eq!(effect, Effect::None);
    assert_eq!(app.screen, Screen::Menu);
}

#[test]
fn settings_screen_quit_works() {
    let mut app = App::new();
    app.screen = Screen::Settings;

    let effect = app.handle_action(Action::Quit);
    assert_eq!(effect, Effect::Quit);
}

#[test]
fn settings_screen_select_applies_mode_and_saves() {
    let mut app = App::new();
    app.screen = Screen::Settings;
    app.settings_state = SettingsState::from_active(TermMode::Hybrid);

    // Move to Git mode (index 2)
    app.handle_action(Action::MoveDown);
    app.handle_action(Action::MoveDown);
    assert_eq!(app.settings_state.selected_mode(), TermMode::Git);

    let effect = app.handle_action(Action::Select);
    // Should update terms immediately
    assert_eq!(app.terms.mode, TermMode::Git);
    // Should request persistence
    assert_eq!(effect, Effect::SaveTermMode(TermMode::Git));
}

#[test]
fn settings_screen_select_beginner() {
    let mut app = App::new();
    app.screen = Screen::Settings;
    app.settings_state = SettingsState::from_active(TermMode::Git);

    // Beginner is index 0 — move up to get there
    app.settings_state.cursor = 0;

    let effect = app.handle_action(Action::Select);
    assert_eq!(app.terms.mode, TermMode::Beginner);
    assert_eq!(effect, Effect::SaveTermMode(TermMode::Beginner));
}

#[test]
fn menu_enters_settings_with_correct_cursor() {
    let mut app = App::new();
    app.screen = Screen::Menu;
    app.terms = gitcactus::terminology::Terms::new(TermMode::Git);

    // Settings is at index 8
    app.menu_index = 8;
    let effect = app.handle_action(Action::Select);
    assert_eq!(effect, Effect::None);
    assert_eq!(app.screen, Screen::Settings);
    // Cursor should be at Git (index 2)
    assert_eq!(app.settings_state.selected_mode(), TermMode::Git);
}

// ── Security: settings screen never mutates repo ─────────────────────

#[test]
fn settings_screen_never_returns_repo_mutating_effects() {
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
        app.screen = Screen::Settings;

        let effect = app.handle_action(action);
        match effect {
            Effect::None | Effect::Quit | Effect::SaveTermMode(_) => {} // safe
            other => panic!(
                "Settings screen returned repo-mutating effect {other:?} for action {action:?}"
            ),
        }
    }
}
