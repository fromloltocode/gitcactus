//! Tests for search/filter on history and branches screens.
//!
//! Phase 1: history filter
//! Phase 2: branches filter

use gitcactus::app::{App, BranchesMode, Effect, Screen};
use gitcactus::git::branches::{BranchEntry, BranchListResult};
use gitcactus::git::history::{HistoryEntry, HistoryResult};
use gitcactus::input::{map_key, Action};

// ─────────────────────────────────────────────────────────────────────
// Phase 1: History search/filter
// ─────────────────────────────────────────────────────────────────────

fn make_history() -> HistoryResult {
    HistoryResult {
        entries: vec![
            HistoryEntry {
                short_hash: "aaaa111".into(),
                full_hash: "aaaa111".into(),
                summary: "Add login feature".into(),
                author: "Alice".into(),
                timestamp: 0,
                relative_time: "1h ago".into(),
            },
            HistoryEntry {
                short_hash: "bbbb222".into(),
                full_hash: "bbbb222".into(),
                summary: "Fix navigation bug".into(),
                author: "Bob".into(),
                timestamp: 0,
                relative_time: "2h ago".into(),
            },
            HistoryEntry {
                short_hash: "cccc333".into(),
                full_hash: "cccc333".into(),
                summary: "Refactor LOGIN module".into(),
                author: "Carol".into(),
                timestamp: 0,
                relative_time: "3h ago".into(),
            },
        ],
        is_real: true,
        error: None,
    }
}

#[test]
fn history_empty_filter_shows_all() {
    let mut app = App::new();
    app.history.result = make_history();
    assert_eq!(app.history.visible_indices().len(), 3);
}

#[test]
fn history_filter_case_insensitive_summary() {
    let mut app = App::new();
    app.history.result = make_history();
    app.history.filter = "login".into();
    let visible = app.history.visible_indices();
    // Matches "Add login feature" and "Refactor LOGIN module"
    assert_eq!(visible.len(), 2);
}

#[test]
fn history_filter_matches_author() {
    let mut app = App::new();
    app.history.result = make_history();
    app.history.filter = "bob".into();
    let visible = app.history.visible_indices();
    assert_eq!(visible.len(), 1);
    assert_eq!(app.history.result.entries[visible[0]].author, "Bob");
}

#[test]
fn history_filter_matches_hash() {
    let mut app = App::new();
    app.history.result = make_history();
    app.history.filter = "bbbb".into();
    let visible = app.history.visible_indices();
    assert_eq!(visible.len(), 1);
}

#[test]
fn history_filter_no_matches() {
    let mut app = App::new();
    app.history.result = make_history();
    app.history.filter = "nonexistent".into();
    assert!(app.history.visible_indices().is_empty());
}

#[test]
fn history_slash_enters_search_mode() {
    let mut app = App::new();
    app.screen = Screen::History;
    app.history.result = make_history();

    let effect = app.handle_action(Action::Search);
    assert_eq!(effect, Effect::None);
    assert!(app.history.search_mode);
}

#[test]
fn history_search_mode_needs_text_input() {
    let mut app = App::new();
    app.screen = Screen::History;
    app.history.search_mode = true;
    assert!(app.needs_text_input());
}

#[test]
fn history_typing_updates_filter_and_resets_cursor() {
    let mut app = App::new();
    app.screen = Screen::History;
    app.history.result = make_history();
    app.history.cursor = 2;
    app.history.search_mode = true;

    app.handle_action(Action::Char('l'));
    app.handle_action(Action::Char('o'));
    app.handle_action(Action::Char('g'));
    assert_eq!(app.history.filter, "log");
    assert_eq!(app.history.cursor, 0);
}

#[test]
fn history_backspace_edits_filter() {
    let mut app = App::new();
    app.screen = Screen::History;
    app.history.search_mode = true;
    app.history.filter = "login".into();

    app.handle_action(Action::Backspace);
    assert_eq!(app.history.filter, "logi");
}

#[test]
fn history_enter_in_search_keeps_filter_exits_search() {
    let mut app = App::new();
    app.screen = Screen::History;
    app.history.result = make_history();
    app.history.search_mode = true;
    app.history.filter = "login".into();

    let effect = app.handle_action(Action::Select);
    assert_eq!(effect, Effect::None);
    assert!(!app.history.search_mode);
    assert_eq!(app.history.filter, "login");
}

#[test]
fn history_esc_in_search_clears_filter() {
    let mut app = App::new();
    app.screen = Screen::History;
    app.history.search_mode = true;
    app.history.filter = "anything".into();

    let effect = app.handle_action(Action::Back);
    assert_eq!(effect, Effect::None);
    assert!(!app.history.search_mode);
    assert!(app.history.filter.is_empty());
}

#[test]
fn history_esc_in_browse_with_filter_clears_first() {
    let mut app = App::new();
    app.screen = Screen::History;
    app.history.filter = "login".into();
    app.history.search_mode = false;

    let effect = app.handle_action(Action::Back);
    assert_eq!(effect, Effect::None);
    assert_eq!(app.screen, Screen::History);
    assert!(app.history.filter.is_empty());
}

#[test]
fn history_esc_in_browse_without_filter_goes_to_menu() {
    let mut app = App::new();
    app.screen = Screen::History;

    let effect = app.handle_action(Action::Back);
    assert_eq!(effect, Effect::None);
    assert_eq!(app.screen, Screen::Menu);
}

#[test]
fn history_cursor_stays_in_bounds_after_filter() {
    let mut app = App::new();
    app.screen = Screen::History;
    app.history.result = make_history();
    app.history.cursor = 2;

    // Applying a filter that only matches 1 entry should clamp cursor.
    app.history.filter = "bob".into();
    app.history.clamp_cursor();
    assert_eq!(app.history.cursor, 0);
}

#[test]
fn history_enter_uses_filtered_entry_not_raw_index() {
    let mut app = App::new();
    app.screen = Screen::History;
    app.history.result = make_history();
    // Filter so only "Bob" commit (index 1 in raw list) is visible.
    app.history.filter = "bob".into();
    app.history.cursor = 0; // visible index 0 -> raw index 1

    let effect = app.handle_action(Action::Select);
    assert!(matches!(effect, Effect::LoadCommitDetails(ref h) if h == "bbbb222"));
}

#[test]
fn history_d_uses_filtered_entry() {
    let mut app = App::new();
    app.screen = Screen::History;
    app.history.result = make_history();
    app.history.filter = "carol".into();
    app.history.cursor = 0;

    let effect = app.handle_action(Action::Preview);
    assert!(matches!(effect, Effect::LoadCommitDiff(ref h) if h == "cccc333"));
}

#[test]
fn history_no_results_any_action_is_safe() {
    let mut app = App::new();
    app.screen = Screen::History;
    app.history.result = make_history();
    app.history.filter = "nope".into();

    let effect = app.handle_action(Action::Select);
    assert_eq!(effect, Effect::None);

    let effect = app.handle_action(Action::Preview);
    assert_eq!(effect, Effect::None);
}

// ─────────────────────────────────────────────────────────────────────
// Phase 2: Branches search/filter
// ─────────────────────────────────────────────────────────────────────

fn make_branches() -> BranchListResult {
    BranchListResult {
        branches: vec![
            BranchEntry {
                name: "main".into(),
                is_current: true,
                short_hash: "aaa0000".into(),
                summary: "Baseline work".into(),
            },
            BranchEntry {
                name: "feat-login".into(),
                is_current: false,
                short_hash: "bbb0000".into(),
                summary: "Auth screen wireframes".into(),
            },
            BranchEntry {
                name: "fix-navigation".into(),
                is_current: false,
                short_hash: "ccc0000".into(),
                summary: "Fix menu arrow keys".into(),
            },
        ],
        error: None,
    }
}

#[test]
fn branches_empty_filter_shows_all() {
    let mut app = App::new();
    app.branches_state.branches = make_branches();
    assert_eq!(app.branches_state.visible_indices().len(), 3);
}

#[test]
fn branches_filter_matches_name() {
    let mut app = App::new();
    app.branches_state.branches = make_branches();
    app.branches_state.filter = "feat".into();
    assert_eq!(app.branches_state.visible_indices().len(), 1);
}

#[test]
fn branches_filter_case_insensitive() {
    let mut app = App::new();
    app.branches_state.branches = make_branches();
    app.branches_state.filter = "LOGIN".into();
    assert_eq!(app.branches_state.visible_indices().len(), 1);
}

#[test]
fn branches_filter_matches_summary() {
    let mut app = App::new();
    app.branches_state.branches = make_branches();
    app.branches_state.filter = "wireframes".into();
    assert_eq!(app.branches_state.visible_indices().len(), 1);
}

#[test]
fn branches_filter_no_matches() {
    let mut app = App::new();
    app.branches_state.branches = make_branches();
    app.branches_state.filter = "xyz999".into();
    assert!(app.branches_state.visible_indices().is_empty());
}

#[test]
fn branches_slash_enters_search_mode() {
    let mut app = App::new();
    app.screen = Screen::Branches;
    app.branches_state.branches = make_branches();

    let effect = app.handle_action(Action::Search);
    assert_eq!(effect, Effect::None);
    assert!(app.branches_state.search_mode);
}

#[test]
fn branches_search_mode_needs_text_input() {
    let mut app = App::new();
    app.screen = Screen::Branches;
    app.branches_state.search_mode = true;
    assert!(app.needs_text_input());
}

#[test]
fn branches_typing_updates_filter() {
    let mut app = App::new();
    app.screen = Screen::Branches;
    app.branches_state.search_mode = true;

    for c in "feat".chars() {
        app.handle_action(Action::Char(c));
    }
    assert_eq!(app.branches_state.filter, "feat");
}

#[test]
fn branches_esc_in_search_clears_filter() {
    let mut app = App::new();
    app.screen = Screen::Branches;
    app.branches_state.search_mode = true;
    app.branches_state.filter = "anything".into();

    app.handle_action(Action::Back);
    assert!(!app.branches_state.search_mode);
    assert!(app.branches_state.filter.is_empty());
}

#[test]
fn branches_enter_in_search_applies_filter() {
    let mut app = App::new();
    app.screen = Screen::Branches;
    app.branches_state.branches = make_branches();
    app.branches_state.search_mode = true;
    app.branches_state.filter = "feat".into();

    app.handle_action(Action::Select);
    assert!(!app.branches_state.search_mode);
    assert_eq!(app.branches_state.filter, "feat");
}

#[test]
fn branches_switch_uses_filtered_selection() {
    let mut app = App::new();
    app.screen = Screen::Branches;
    app.branches_state.branches = make_branches();
    // Filter only "feat-login" visible.
    app.branches_state.filter = "feat".into();
    app.branches_state.cursor = 0;

    // Enter on visible item (not current) opens confirm.
    app.handle_action(Action::Select);
    assert_eq!(app.branches_state.mode, BranchesMode::ConfirmSwitch);

    // Confirm switch — should target feat-login (the filtered entry).
    let effect = app.handle_action(Action::Confirm);
    assert!(matches!(effect, Effect::SwitchBranch(ref n) if n == "feat-login"));
}

#[test]
fn branches_cursor_clamped_after_filter_shrink() {
    let mut app = App::new();
    app.branches_state.branches = make_branches();
    app.branches_state.cursor = 2;
    app.branches_state.filter = "main".into(); // only 1 match
    app.branches_state.clamp_cursor();
    assert_eq!(app.branches_state.cursor, 0);
}

#[test]
fn branches_new_game_still_works_with_filter() {
    let mut app = App::new();
    app.screen = Screen::Branches;
    app.branches_state.branches = make_branches();
    app.branches_state.filter = "feat".into();

    // 'n' triggers new game creation — must still work.
    app.handle_action(Action::Deny);
    assert_eq!(app.branches_state.mode, BranchesMode::Creating);
}

#[test]
fn branches_no_results_any_action_is_safe() {
    let mut app = App::new();
    app.screen = Screen::Branches;
    app.branches_state.branches = make_branches();
    app.branches_state.filter = "none".into();

    // No visible items → Select is a no-op (safe).
    let effect = app.handle_action(Action::Select);
    assert_eq!(effect, Effect::None);
}

// ─────────────────────────────────────────────────────────────────────
// Input mapping: `/` triggers Search in command mode and types normally in text
// ─────────────────────────────────────────────────────────────────────

#[test]
fn slash_maps_to_search() {
    use crossterm::event::KeyCode;
    assert_eq!(map_key(KeyCode::Char('/')), Action::Search);
}

#[test]
fn slash_types_as_char_in_text_mode() {
    use crossterm::event::KeyCode;
    use gitcactus::input::map_key_text;
    assert_eq!(map_key_text(KeyCode::Char('/')), Action::Char('/'));
}

// ─────────────────────────────────────────────────────────────────────
// Safety: search/filter never produces repo-mutating effects
// ─────────────────────────────────────────────────────────────────────

#[test]
fn history_search_mode_never_mutates_repo() {
    let actions = [
        Action::Quit, Action::Back, Action::MoveUp, Action::MoveDown,
        Action::Select, Action::Toggle, Action::ToggleAll, Action::Refresh,
        Action::Confirm, Action::Deny, Action::Preview, Action::Search,
        Action::Char('x'), Action::Backspace, Action::Other,
    ];
    for action in actions {
        let mut app = App::new();
        app.screen = Screen::History;
        app.history.search_mode = true;
        app.history.filter = "abc".into();

        let effect = app.handle_action(action);
        if matches!(effect, Effect::StageFiles(_) | Effect::CreateCommit(_)
                         | Effect::SwitchBranch(_) | Effect::CreateBranch(_)) {
            panic!("History search returned mutating effect for {action:?}");
        }
    }
}

#[test]
fn branches_search_mode_never_mutates_repo() {
    let actions = [
        Action::Quit, Action::Back, Action::MoveUp, Action::MoveDown,
        Action::Select, Action::Toggle, Action::ToggleAll, Action::Refresh,
        Action::Confirm, Action::Deny, Action::Preview, Action::Search,
        Action::Char('x'), Action::Backspace, Action::Other,
    ];
    for action in actions {
        let mut app = App::new();
        app.screen = Screen::Branches;
        app.branches_state.search_mode = true;
        app.branches_state.filter = "abc".into();

        let effect = app.handle_action(action);
        if matches!(effect, Effect::StageFiles(_) | Effect::CreateCommit(_)
                         | Effect::SwitchBranch(_) | Effect::CreateBranch(_)) {
            panic!("Branches search returned mutating effect for {action:?}");
        }
    }
}
