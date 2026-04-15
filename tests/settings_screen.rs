//! Tests for the in-app Settings screen.
//!
//! The screen now has two sections (terminology + theme) accessed
//! through one flat cursor; these tests exercise both sections and
//! the boundary between them.

use gitcactus::app::{
    App, Effect, Screen, SettingsSelection, SettingsState, SETTINGS_TERM_MODES,
    SETTINGS_THEME_PRESETS,
};
use gitcactus::input::Action;
use gitcactus::terminology::TermMode;
use gitcactus::theme::ThemePreset;

/// Extract the currently-selected terminology mode, or None if the
/// cursor is on the theme section.
fn selected_mode(s: &SettingsState) -> Option<TermMode> {
    match s.selection() {
        SettingsSelection::TermMode(m) => Some(m),
        _ => None,
    }
}

/// Extract the currently-selected theme preset, or None if the cursor
/// is on the terminology section.
fn selected_theme(s: &SettingsState) -> Option<ThemePreset> {
    match s.selection() {
        SettingsSelection::Theme(p) => Some(p),
        _ => None,
    }
}

// ── SettingsState ────────────────────────────────────────────────────

#[test]
fn settings_state_starts_at_zero() {
    let s = SettingsState::new();
    assert_eq!(s.cursor, 0);
}

#[test]
fn settings_from_active_matches_mode() {
    let s = SettingsState::from_active(TermMode::Hybrid);
    assert_eq!(selected_mode(&s), Some(TermMode::Hybrid));

    let s = SettingsState::from_active(TermMode::Git);
    assert_eq!(selected_mode(&s), Some(TermMode::Git));

    let s = SettingsState::from_active(TermMode::Beginner);
    assert_eq!(selected_mode(&s), Some(TermMode::Beginner));
}

#[test]
fn settings_move_up_clamps_at_zero() {
    let mut s = SettingsState::new();
    s.move_up();
    assert_eq!(s.cursor, 0);
}

#[test]
fn settings_move_down_clamps_at_total_rows() {
    let mut s = SettingsState::new();
    for _ in 0..100 {
        s.move_down();
    }
    assert_eq!(s.cursor, SettingsState::total_rows() - 1);
}

#[test]
fn total_rows_is_terminology_plus_theme_rows() {
    assert_eq!(
        SettingsState::total_rows(),
        SETTINGS_TERM_MODES.len() + SETTINGS_THEME_PRESETS.len()
    );
}

#[test]
fn settings_navigate_through_terminology_section() {
    let mut s = SettingsState::new();
    assert_eq!(selected_mode(&s), Some(TermMode::Beginner));
    s.move_down();
    assert_eq!(selected_mode(&s), Some(TermMode::Hybrid));
    s.move_down();
    assert_eq!(selected_mode(&s), Some(TermMode::Git));
    s.move_up();
    assert_eq!(selected_mode(&s), Some(TermMode::Hybrid));
}

#[test]
fn cursor_crosses_cleanly_into_theme_section() {
    let mut s = SettingsState::new();
    // Walk all three terminology rows.
    for _ in 0..SETTINGS_TERM_MODES.len() - 1 {
        s.move_down();
    }
    // One more step lands us on the first theme preset.
    s.move_down();
    assert_eq!(selected_mode(&s), None);
    assert_eq!(selected_theme(&s), Some(ThemePreset::Default));

    // Then each subsequent step walks through the theme presets.
    let remaining: Vec<_> = SETTINGS_THEME_PRESETS
        .iter()
        .skip(1)
        .copied()
        .collect();
    for expected in remaining {
        s.move_down();
        assert_eq!(selected_theme(&s), Some(expected));
    }
}

#[test]
fn moving_back_up_crosses_into_terminology_section() {
    let mut s = SettingsState::new();
    s.cursor = SettingsState::total_rows() - 1; // last theme row
    for _ in 0..SETTINGS_THEME_PRESETS.len() {
        s.move_up();
    }
    // Now we should be on the last terminology row (Git).
    assert_eq!(selected_mode(&s), Some(TermMode::Git));
}

// ── App integration: terminology section ─────────────────────────────

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
fn settings_screen_select_applies_term_mode_and_saves() {
    let mut app = App::new();
    app.screen = Screen::Settings;
    app.settings_state = SettingsState::from_active(TermMode::Beginner);

    // Move to Git mode (cursor 0 → 1 → 2).
    app.handle_action(Action::MoveDown);
    app.handle_action(Action::MoveDown);
    assert_eq!(selected_mode(&app.settings_state), Some(TermMode::Git));

    let effect = app.handle_action(Action::Select);
    assert_eq!(app.terms.mode, TermMode::Git);
    assert_eq!(effect, Effect::SaveTermMode(TermMode::Git));
}

#[test]
fn settings_screen_select_beginner() {
    let mut app = App::new();
    app.screen = Screen::Settings;
    app.settings_state = SettingsState::from_active(TermMode::Git);
    app.settings_state.cursor = 0; // Beginner

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
    assert_eq!(selected_mode(&app.settings_state), Some(TermMode::Git));
}

// ── App integration: theme section ──────────────────────────────────

#[test]
fn selecting_theme_applies_palette_and_requests_save() {
    let mut app = App::new();
    app.screen = Screen::Settings;

    // Jump to the Matrix preset (index 2 in theme list).
    app.settings_state.cursor = SETTINGS_TERM_MODES.len() + 2;
    assert_eq!(
        selected_theme(&app.settings_state),
        Some(ThemePreset::Matrix)
    );

    let effect = app.handle_action(Action::Select);
    assert_eq!(effect, Effect::SaveThemePreset(ThemePreset::Matrix));
    // The bare preset palette is applied optimistically so the UI
    // refreshes immediately; the effect handler reloads to fold in
    // any `theme.*=` overrides afterwards.
    assert_eq!(app.theme.preset, ThemePreset::Matrix);
    assert_eq!(app.theme.primary, ThemePreset::Matrix.palette().primary);
}

#[test]
fn selecting_every_theme_preset_applies_it() {
    for (i, &preset) in SETTINGS_THEME_PRESETS.iter().enumerate() {
        let mut app = App::new();
        app.screen = Screen::Settings;
        app.settings_state.cursor = SETTINGS_TERM_MODES.len() + i;

        let effect = app.handle_action(Action::Select);
        assert_eq!(effect, Effect::SaveThemePreset(preset));
        assert_eq!(app.theme.preset, preset);
    }
}

#[test]
fn terminology_select_does_not_touch_theme() {
    let mut app = App::new();
    app.screen = Screen::Settings;
    let before = app.theme.preset;
    app.settings_state.cursor = 0; // Beginner

    let _ = app.handle_action(Action::Select);
    assert_eq!(app.theme.preset, before, "theme must survive term-mode select");
}

#[test]
fn theme_select_does_not_touch_terminology() {
    let mut app = App::new();
    app.screen = Screen::Settings;
    let before = app.terms.mode;
    app.settings_state.cursor = SETTINGS_TERM_MODES.len(); // first theme row

    let _ = app.handle_action(Action::Select);
    assert_eq!(app.terms.mode, before, "term mode must survive theme select");
}

// ── Security: read-only surface ─────────────────────────────────────

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
            // Safe — no repo side-effects.
            Effect::None
            | Effect::Quit
            | Effect::SaveTermMode(_)
            | Effect::SaveThemePreset(_) => {}
            other => panic!(
                "Settings screen returned repo-mutating effect {other:?} for action {action:?}"
            ),
        }
    }
}
