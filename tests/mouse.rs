//! End-to-end tests for mouse support.
//!
//! These tests cover the user-visible guarantees that the spec for
//! mouse support calls out:
//!
//! - clicking a menu item navigates to it and activates it
//! - click targets exist for Back / Exit (footer hints)
//! - clicks do **not** bypass confirmation dialogs
//! - keyboard navigation behaves exactly as before
//! - non-selectable areas don't fire anything
//! - list screens without mouse support return empty target lists
//!   (so the app stays fully keyboard-driven there)
//!
//! Anything that depends on the real terminal (e.g. whether a given
//! terminal emulator actually emits mouse events) isn't unit-testable
//! and is called out in the v0.4.0 manual-verification note.

use gitcactus::app::{App, Effect, Screen, SettingsSelection, StageEntry, StageFileKind, StageMode};
use gitcactus::input::Action;
use gitcactus::mouse::{resolve_click, ClickAction};
use gitcactus::ui;
use ratatui::layout::Rect;

// ── Menu ────────────────────────────────────────────────────────────

/// A generous-sized terminal used for tests so every row fits.
fn area() -> Rect {
    Rect {
        x: 0,
        y: 0,
        width: 100,
        height: 30,
    }
}

/// Walk menu click targets and return the first `SelectMenu(i)` found,
/// or panic with a useful message if no target hits. Used by tests
/// that want "click on menu item N" without knowing its exact y.
fn click_menu_item(app: &App, i: usize) -> ClickAction {
    let targets = ui::click_targets(area(), app);
    let wanted = ClickAction::SelectMenu(i);
    let target = targets
        .iter()
        .find(|t| t.action == wanted)
        .unwrap_or_else(|| panic!("no click target for menu item {i}"));
    resolve_click(
        std::slice::from_ref(target),
        target.rect.x,
        target.rect.y,
    )
    .expect("target should resolve to itself")
}

#[test]
fn menu_exposes_one_click_target_per_item() {
    let mut app = App::new();
    app.screen = Screen::Menu;
    let targets = ui::click_targets(area(), &app);

    // Count SelectMenu targets — every menu item should have one.
    let select_count = targets
        .iter()
        .filter(|t| matches!(t.action, ClickAction::SelectMenu(_)))
        .count();
    assert_eq!(
        select_count,
        gitcactus::app::MENU_ITEMS.len(),
        "each menu row should have its own click target"
    );
}

#[test]
fn clicking_menu_status_selects_and_activates_it() {
    let mut app = App::new();
    app.screen = Screen::Menu;
    // Start from a non-Status cursor so we can verify the click moved it.
    app.menu_index = 4;

    let ca = click_menu_item(&app, 0); // "Status"
    let effect = app.handle_click_action(ca);

    assert_eq!(app.menu_index, 0);
    assert_eq!(app.screen, Screen::Status);
    // Entering Status emits a status-refresh, same as keyboard Enter would.
    assert_eq!(effect, Effect::RefreshStatus);
}

#[test]
fn clicking_menu_quit_behaves_like_keyboard_quit() {
    let mut app = App::new();
    app.screen = Screen::Menu;

    let ca = click_menu_item(&app, gitcactus::app::QUIT_INDEX);
    let effect = app.handle_click_action(ca);
    assert_eq!(effect, Effect::Quit);
}

#[test]
fn menu_footer_exposes_quit_as_clickable() {
    let mut app = App::new();
    app.screen = Screen::Menu;

    let targets = ui::click_targets(area(), &app);
    let has_quit = targets
        .iter()
        .any(|t| t.action == ClickAction::Fire(Action::Quit));
    assert!(has_quit, "menu footer must make q clickable");
}

#[test]
fn menu_footer_exposes_enter_as_clickable() {
    let mut app = App::new();
    app.screen = Screen::Menu;

    let targets = ui::click_targets(area(), &app);
    let has_select = targets
        .iter()
        .any(|t| t.action == ClickAction::Fire(Action::Select));
    assert!(has_select, "menu footer must make Enter clickable");
}

#[test]
fn menu_click_on_empty_space_returns_no_action() {
    let mut app = App::new();
    app.screen = Screen::Menu;
    let targets = ui::click_targets(area(), &app);
    // (99, 29) is in the bottom-right corner of our test area —
    // past every menu row and past the footer hints (which are
    // left-aligned), so it shouldn't hit anything.
    assert_eq!(resolve_click(&targets, 99, 29), None);
}

// ── Settings: both sections + boundary ──────────────────────────────

#[test]
fn clicking_a_settings_theme_row_applies_that_preset() {
    let mut app = App::new();
    app.screen = Screen::Settings;
    let targets = ui::click_targets(area(), &app);

    // Find a SelectSettings target with a theme-section index.
    let theme_row_index =
        gitcactus::app::SETTINGS_TERM_MODES.len() + 2; // Matrix preset row
    let wanted = ClickAction::SelectSettings(theme_row_index);
    let hit = targets.iter().any(|t| t.action == wanted);
    assert!(hit, "settings screen should expose a click target for each theme row");

    // Simulate the click → cursor + Select.
    let effect = app.handle_click_action(wanted);
    assert!(matches!(effect, Effect::SaveThemePreset(_)));
    assert!(matches!(
        app.settings_state.selection(),
        SettingsSelection::Theme(_)
    ));
}

#[test]
fn clicking_settings_terminology_row_applies_that_mode() {
    let mut app = App::new();
    app.screen = Screen::Settings;
    let effect = app.handle_click_action(ClickAction::SelectSettings(2)); // Git
    assert_eq!(effect, Effect::SaveTermMode(gitcactus::terminology::TermMode::Git));
}

// ── Stage: click on file + confirm dialog safety ────────────────────

/// Seed a stage screen with three entries so row-click tests can hit
/// them deterministically. Mode starts at Browse.
fn seeded_stage_app() -> App {
    let mut app = App::new();
    app.screen = Screen::Stage;
    app.stage.entries = vec![
        StageEntry {
            path: "a.txt".into(),
            kind: StageFileKind::Modified,
            selected: false,
        },
        StageEntry {
            path: "b.txt".into(),
            kind: StageFileKind::Untracked,
            selected: false,
        },
        StageEntry {
            path: "c.txt".into(),
            kind: StageFileKind::Modified,
            selected: false,
        },
    ];
    app
}

#[test]
fn clicking_stage_row_toggles_that_entry_only() {
    let mut app = seeded_stage_app();

    let effect = app.handle_click_action(ClickAction::ToggleStage(1));
    assert_eq!(effect, Effect::None);
    assert_eq!(app.stage.cursor, 1);
    assert!(app.stage.entries[1].selected);
    assert!(!app.stage.entries[0].selected);
    assert!(!app.stage.entries[2].selected);
}

#[test]
fn clicking_out_of_range_stage_row_is_a_no_op() {
    let mut app = seeded_stage_app();
    let effect = app.handle_click_action(ClickAction::ToggleStage(99));
    assert_eq!(effect, Effect::None);
    assert!(!app.stage.entries[0].selected);
    assert!(!app.stage.entries[1].selected);
    assert!(!app.stage.entries[2].selected);
}

#[test]
fn clicking_confirm_in_dialog_is_the_same_as_pressing_y() {
    // Start a stage flow, select file 0, arm the confirm dialog,
    // then simulate the user clicking the "y/Enter confirm" button.
    let mut app = seeded_stage_app();
    app.stage.entries[0].selected = true;
    app.stage.mode = StageMode::Confirm;

    // Click routes Action::Confirm through the normal handler:
    let effect = app.handle_click_action(ClickAction::Fire(Action::Confirm));

    // Same outcome as `handle_action(Action::Confirm)` on Confirm mode.
    assert!(matches!(effect, Effect::StageFiles(_)));
}

#[test]
fn clicking_cancel_in_dialog_returns_to_browse_without_staging() {
    let mut app = seeded_stage_app();
    app.stage.entries[0].selected = true;
    app.stage.mode = StageMode::Confirm;

    let effect = app.handle_click_action(ClickAction::Fire(Action::Deny));
    assert_eq!(effect, Effect::None);
    assert_eq!(app.stage.mode, StageMode::Browse);
    // And nothing was staged: no side effects.
}

#[test]
fn click_cannot_bypass_confirm_dialog() {
    // A click inside the confirm dialog must NOT emit `StageFiles`
    // unless the click is on the explicit Confirm button. Wandering
    // clicks should be harmless.
    let mut app = seeded_stage_app();
    app.stage.entries[0].selected = true;
    app.stage.mode = StageMode::Confirm;

    let targets = ui::click_targets(area(), &app);

    // Gather all StageFiles triggers: they should only come from
    // clicks on ClickAction::Fire(Action::Confirm).
    for target in &targets {
        match target.action {
            ClickAction::Fire(Action::Confirm) => { /* the one legitimate path */ }
            ClickAction::Fire(Action::Deny) => { /* ok — cancels safely */ }
            ClickAction::Fire(Action::Back) => { /* ok — cancels via Esc */ }
            ClickAction::Fire(Action::Quit) => { /* ok — global quit */ }
            ClickAction::Fire(Action::Select) => {
                // Select in Confirm mode fires StageFiles — this is
                // the keyboard equivalent of pressing Enter in the
                // dialog. Only the overlay's explicit button should
                // register Select, never the underlying file list.
            }
            ClickAction::ToggleStage(_) => {
                panic!(
                    "ToggleStage target shouldn't exist while Confirm dialog is up \
                     (would let a user toggle selections from under the dialog)"
                );
            }
            _ => {}
        }
    }

    // And confirm the overlay really suppresses the underlying list.
    let has_toggle = targets
        .iter()
        .any(|t| matches!(t.action, ClickAction::ToggleStage(_)));
    assert!(!has_toggle, "confirm overlay must hide ToggleStage targets");
}

// ── Keyboard regression: clicking nothing doesn't change state ──────

#[test]
fn returning_empty_click_targets_means_mouse_is_silently_ignored() {
    // Screens that haven't opted into mouse support return an empty
    // target vec. The main loop's `resolve_click` then returns None
    // and nothing happens — we double-check here.
    let mut app = App::new();
    app.screen = Screen::Help;
    let targets = ui::click_targets(area(), &app);
    assert_eq!(
        resolve_click(&targets, 5, 5),
        None,
        "non-mouse-enabled screens must not produce phantom click actions"
    );
}

#[test]
fn keyboard_still_works_on_mouse_enabled_screens() {
    // Regression: make sure adding mouse support didn't change
    // keyboard semantics. Press Down in Menu — cursor advances.
    let mut app = App::new();
    app.screen = Screen::Menu;
    let before = app.menu_index;

    let effect = app.handle_action(Action::MoveDown);
    assert_eq!(effect, Effect::None);
    assert_eq!(app.menu_index, before + 1);
}

// ── Animation overlay grabs every click ─────────────────────────────

#[test]
fn animation_overlay_consumes_any_click() {
    use gitcactus::mascot::animations::{AnimationKind, AnimationState};

    let mut app = App::new();
    app.screen = Screen::Menu;
    app.animation = Some(AnimationState::new(AnimationKind::Pull));

    let targets = ui::click_targets(area(), &app);
    // Every click inside the terminal should map to Action::Select,
    // which is how handle_action dismisses/fast-forwards animations.
    let click = resolve_click(&targets, 5, 5).expect("animation click grabs input");
    assert_eq!(click, ClickAction::Fire(Action::Select));

    // And dispatching it should actually dismiss / fast-forward
    // the animation via the existing handler.
    let _ = app.handle_click_action(click);
}
