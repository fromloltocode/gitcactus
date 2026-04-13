//! Tests for the Branches / Saved Paths screen.

use gitcactus::app::{App, BranchesMode, BranchesState, Effect, Screen};
use gitcactus::git::branches::{BranchEntry, BranchListResult};
use gitcactus::input::Action;

// ── BranchesState ────────────────────────────────────────────────────

#[test]
fn branches_state_starts_empty() {
    let s = BranchesState::new();
    assert_eq!(s.cursor, 0);
    assert!(s.branches.branches.is_empty());
    assert_eq!(s.mode, BranchesMode::Browse);
}

#[test]
fn branches_move_up_clamps() {
    let mut s = BranchesState::new();
    s.move_up();
    assert_eq!(s.cursor, 0);
}

#[test]
fn branches_move_down_clamps_when_empty() {
    let mut s = BranchesState::new();
    s.move_down();
    assert_eq!(s.cursor, 0);
}

#[test]
fn branches_navigate_with_entries() {
    let mut s = BranchesState::new();
    s.branches = make_result(3, 0);

    s.move_down();
    assert_eq!(s.cursor, 1);
    s.move_down();
    assert_eq!(s.cursor, 2);
    s.move_down();
    assert_eq!(s.cursor, 2); // clamped
    s.move_up();
    assert_eq!(s.cursor, 1);
}

#[test]
fn branches_selected_name_and_current() {
    let mut s = BranchesState::new();
    s.branches = make_result(3, 1); // current is at index 1

    assert_eq!(s.selected_name(), Some("branch-0"));
    assert!(!s.selected_is_current());

    s.move_down();
    assert_eq!(s.selected_name(), Some("branch-1"));
    assert!(s.selected_is_current());
}

#[test]
fn branches_selected_name_empty_list() {
    let s = BranchesState::new();
    assert_eq!(s.selected_name(), None);
    assert!(!s.selected_is_current());
}

// ── App integration ──────────────────────────────────────────────────

#[test]
fn branches_screen_back_returns_to_menu() {
    let mut app = App::new();
    app.screen = Screen::Branches;

    let effect = app.handle_action(Action::Back);
    assert_eq!(effect, Effect::None);
    assert_eq!(app.screen, Screen::Menu);
}

#[test]
fn branches_screen_quit() {
    let mut app = App::new();
    app.screen = Screen::Branches;

    let effect = app.handle_action(Action::Quit);
    assert_eq!(effect, Effect::Quit);
}

#[test]
fn branches_screen_refresh() {
    let mut app = App::new();
    app.screen = Screen::Branches;

    let effect = app.handle_action(Action::Refresh);
    assert_eq!(effect, Effect::LoadBranches);
}

#[test]
fn menu_enters_branches_with_load_effect() {
    let mut app = App::new();
    app.screen = Screen::Menu;
    app.menu_index = 3; // Branches

    let effect = app.handle_action(Action::Select);
    assert_eq!(effect, Effect::LoadBranches);
    assert_eq!(app.screen, Screen::Branches);
}

#[test]
fn branches_select_non_current_opens_confirm() {
    let mut app = App::new();
    app.screen = Screen::Branches;
    app.branches_state.branches = make_result(3, 0); // current is branch-0
    app.branches_state.cursor = 1; // highlight branch-1

    let effect = app.handle_action(Action::Select);
    assert_eq!(effect, Effect::None);
    assert_eq!(app.branches_state.mode, BranchesMode::ConfirmSwitch);
}

#[test]
fn branches_select_current_shows_friendly_result() {
    let mut app = App::new();
    app.screen = Screen::Branches;
    app.branches_state.branches = make_result(3, 0);
    app.branches_state.cursor = 0; // on current

    let effect = app.handle_action(Action::Select);
    assert_eq!(effect, Effect::None);
    assert_eq!(app.branches_state.mode, BranchesMode::Result);
    let msg = &app.branches_state.result_msg.as_ref().unwrap().0;
    assert!(msg.contains("already"));
}

#[test]
fn confirm_switch_confirm_triggers_switch_effect() {
    let mut app = App::new();
    app.screen = Screen::Branches;
    app.branches_state.branches = make_result(3, 0);
    app.branches_state.cursor = 2; // branch-2
    app.branches_state.mode = BranchesMode::ConfirmSwitch;

    let effect = app.handle_action(Action::Confirm);
    assert!(matches!(effect, Effect::SwitchBranch(ref n) if n == "branch-2"));
}

#[test]
fn confirm_switch_deny_returns_to_browse() {
    let mut app = App::new();
    app.screen = Screen::Branches;
    app.branches_state.branches = make_result(2, 0);
    app.branches_state.cursor = 1;
    app.branches_state.mode = BranchesMode::ConfirmSwitch;

    let effect = app.handle_action(Action::Deny);
    assert_eq!(effect, Effect::None);
    assert_eq!(app.branches_state.mode, BranchesMode::Browse);
}

#[test]
fn confirm_switch_esc_returns_to_browse() {
    let mut app = App::new();
    app.screen = Screen::Branches;
    app.branches_state.branches = make_result(2, 0);
    app.branches_state.cursor = 1;
    app.branches_state.mode = BranchesMode::ConfirmSwitch;

    let effect = app.handle_action(Action::Back);
    assert_eq!(effect, Effect::None);
    assert_eq!(app.branches_state.mode, BranchesMode::Browse);
}

#[test]
fn result_mode_any_key_refreshes_and_returns_to_browse() {
    let mut app = App::new();
    app.screen = Screen::Branches;
    app.branches_state.mode = BranchesMode::Result;

    let effect = app.handle_action(Action::Other);
    assert_eq!(effect, Effect::LoadBranches);
    assert_eq!(app.branches_state.mode, BranchesMode::Browse);
}

// ── Security: no destructive effects from unexpected actions ─────────

#[test]
fn branches_screen_never_returns_stage_or_commit_effects() {
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

    for mode in [BranchesMode::Browse, BranchesMode::ConfirmSwitch, BranchesMode::Result] {
        for action in all_actions {
            let mut app = App::new();
            app.screen = Screen::Branches;
            app.branches_state.branches = make_result(2, 0);
            app.branches_state.mode = mode;

            let effect = app.handle_action(action);
            match effect {
                Effect::StageFiles(_) | Effect::CreateCommit(_) => {
                    panic!("Branches screen returned stage/commit effect for action {action:?} in mode {mode:?}");
                }
                _ => {}
            }
        }
    }
}

// ── Helper ───────────────────────────────────────────────────────────

fn make_result(n: usize, current_idx: usize) -> BranchListResult {
    let branches: Vec<BranchEntry> = (0..n)
        .map(|i| BranchEntry {
            name: format!("branch-{i}"),
            is_current: i == current_idx,
            short_hash: format!("{i:07}"),
            summary: format!("Commit for branch {i}"),
        })
        .collect();
    BranchListResult {
        branches,
        error: None,
    }
}
