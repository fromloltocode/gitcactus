//! Tests for the "New Game" (Create Branch) flow and terminology.

use gitcactus::app::{App, BranchesMode, Effect, Screen};
use gitcactus::input::Action;
use gitcactus::terminology::{TermMode, Terms};

// ── Terminology ──────────────────────────────────────────────────────

#[test]
fn new_branch_action_beginner() {
    let t = Terms::new(TermMode::Beginner);
    assert_eq!(t.new_branch_action(), "New Game");
}

#[test]
fn new_branch_action_hybrid_mentions_both() {
    let t = Terms::new(TermMode::Hybrid);
    let label = t.new_branch_action();
    assert!(label.contains("New Game"));
    assert!(label.contains("New Path"));
}

#[test]
fn new_branch_action_git() {
    let t = Terms::new(TermMode::Git);
    assert_eq!(t.new_branch_action(), "Create Branch");
}

#[test]
fn new_branch_title_non_empty_all_modes() {
    for mode in [TermMode::Beginner, TermMode::Hybrid, TermMode::Git] {
        let t = Terms::new(mode);
        assert!(!t.new_branch_title().is_empty());
        assert!(!t.new_branch_description().is_empty());
    }
}

#[test]
fn beginner_title_is_start_a_new_game() {
    let t = Terms::new(TermMode::Beginner);
    assert!(t.new_branch_title().contains("New Game"));
}

// ── Mode transitions ─────────────────────────────────────────────────

#[test]
fn n_from_browse_enters_creating_mode() {
    let mut app = App::new();
    app.screen = Screen::Branches;
    app.branches_state.new_name.push_str("leftover");

    let effect = app.handle_action(Action::Deny); // 'n' maps to Deny
    assert_eq!(effect, Effect::None);
    assert_eq!(app.branches_state.mode, BranchesMode::Creating);
    // Buffer should be cleared when entering creating mode
    assert!(app.branches_state.new_name.is_empty());
}

#[test]
fn creating_mode_needs_text_input() {
    let mut app = App::new();
    app.screen = Screen::Branches;
    app.branches_state.mode = BranchesMode::Creating;
    assert!(app.needs_text_input());
}

#[test]
fn browse_mode_does_not_need_text_input() {
    let mut app = App::new();
    app.screen = Screen::Branches;
    app.branches_state.mode = BranchesMode::Browse;
    assert!(!app.needs_text_input());
}

#[test]
fn creating_mode_esc_cancels_and_clears() {
    let mut app = App::new();
    app.screen = Screen::Branches;
    app.branches_state.mode = BranchesMode::Creating;
    app.branches_state.new_name.push_str("feature");

    let effect = app.handle_action(Action::Back);
    assert_eq!(effect, Effect::None);
    assert_eq!(app.branches_state.mode, BranchesMode::Browse);
    assert!(app.branches_state.new_name.is_empty());
}

#[test]
fn creating_mode_typing_builds_name() {
    let mut app = App::new();
    app.screen = Screen::Branches;
    app.branches_state.mode = BranchesMode::Creating;

    for c in "feat-xyz".chars() {
        app.handle_action(Action::Char(c));
    }
    assert_eq!(app.branches_state.new_name, "feat-xyz");
}

#[test]
fn creating_mode_backspace_deletes() {
    let mut app = App::new();
    app.screen = Screen::Branches;
    app.branches_state.mode = BranchesMode::Creating;
    app.branches_state.new_name.push_str("typo");

    app.handle_action(Action::Backspace);
    assert_eq!(app.branches_state.new_name, "typ");
}

#[test]
fn creating_mode_enter_triggers_create_effect() {
    let mut app = App::new();
    app.screen = Screen::Branches;
    app.branches_state.mode = BranchesMode::Creating;
    app.branches_state.new_name.push_str("my-branch");

    let effect = app.handle_action(Action::Select);
    assert!(matches!(effect, Effect::CreateBranch(ref n) if n == "my-branch"));
}

#[test]
fn creating_mode_enter_with_empty_name_is_noop() {
    let mut app = App::new();
    app.screen = Screen::Branches;
    app.branches_state.mode = BranchesMode::Creating;

    let effect = app.handle_action(Action::Select);
    assert_eq!(effect, Effect::None);
}

#[test]
fn creating_mode_enter_trims_whitespace() {
    let mut app = App::new();
    app.screen = Screen::Branches;
    app.branches_state.mode = BranchesMode::Creating;
    app.branches_state.new_name.push_str("  spaced  ");

    let effect = app.handle_action(Action::Select);
    assert!(matches!(effect, Effect::CreateBranch(ref n) if n == "spaced"));
}

// ── CreateResult semantics ───────────────────────────────────────────

#[test]
fn create_result_invalid_name_rejects_empty() {
    use gitcactus::git::branches::{create_branch, CreateResult};
    // The function does a trim check; calling with whitespace-only should reject
    let result = create_branch(".", "   ");
    assert!(matches!(result, CreateResult::InvalidName(_)));
}

#[test]
fn create_result_invalid_name_rejects_bad_chars() {
    use gitcactus::git::branches::{create_branch, CreateResult};
    for bad in &["feat with spaces", "feat..bad", "-dash", "feat~bad", "feat?bad"] {
        let result = create_branch(".", bad);
        assert!(
            matches!(result, CreateResult::InvalidName(_)),
            "Expected InvalidName for {bad:?}, got {result:?}"
        );
    }
}

// ── Name length guard ────────────────────────────────────────────────

#[test]
fn name_buffer_bounded() {
    use gitcactus::app::MAX_BRANCH_NAME_LEN;
    let mut app = App::new();
    app.screen = Screen::Branches;
    app.branches_state.mode = BranchesMode::Creating;

    for _ in 0..(MAX_BRANCH_NAME_LEN + 100) {
        app.handle_action(Action::Char('x'));
    }
    assert!(app.branches_state.new_name.len() <= MAX_BRANCH_NAME_LEN);
}
