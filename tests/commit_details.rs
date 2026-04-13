//! Tests for the Commit Details screen.

use gitcactus::app::{App, CommitDetailsState, Effect, Screen};
use gitcactus::git::commit_details::{ChangeKind, CommitDetails, FileChange};
use gitcactus::git::history::{HistoryEntry, HistoryResult};
use gitcactus::input::Action;

// ── CommitDetailsState ───────────────────────────────────────────────

#[test]
fn commit_details_state_starts_empty() {
    let s = CommitDetailsState::new();
    assert_eq!(s.scroll, 0);
    assert!(s.details.full_hash.is_empty());
    assert!(s.source_hash.is_empty());
}

#[test]
fn commit_details_scroll_up_clamps() {
    let mut s = CommitDetailsState::new();
    s.scroll_up();
    assert_eq!(s.scroll, 0);
}

#[test]
fn commit_details_scroll_with_content() {
    let mut s = CommitDetailsState::new();
    s.details.message = (0..50).map(|i| format!("line {i}\n")).collect();
    s.details.files = (0..20)
        .map(|i| FileChange {
            path: format!("file{i}.rs"),
            kind: ChangeKind::Modified,
        })
        .collect();

    s.scroll_down();
    assert_eq!(s.scroll, 2);
    s.scroll_down();
    assert_eq!(s.scroll, 4);
    s.scroll_up();
    assert_eq!(s.scroll, 2);
}

// ── History → CommitDetails navigation ───────────────────────────────

#[test]
fn history_enter_opens_commit_details() {
    let mut app = App::new();
    app.screen = Screen::History;
    app.history.result = make_history(3);

    let effect = app.handle_action(Action::Select);
    let expected = format!("{:040}", 0);
    assert!(matches!(effect, Effect::LoadCommitDetails(ref h) if *h == expected));
    assert_eq!(app.screen, Screen::CommitDetails);
}

#[test]
fn history_enter_no_op_when_empty() {
    let mut app = App::new();
    app.screen = Screen::History;

    let effect = app.handle_action(Action::Select);
    assert_eq!(effect, Effect::None);
    assert_eq!(app.screen, Screen::History);
}

#[test]
fn history_d_opens_commit_diff() {
    let mut app = App::new();
    app.screen = Screen::History;
    app.history.result = make_history(3);
    app.history.cursor = 1;

    let effect = app.handle_action(Action::Preview);
    let expected = format!("{:040}", 1);
    assert!(matches!(effect, Effect::LoadCommitDiff(ref h) if *h == expected));
    assert_eq!(app.screen, Screen::DiffPreview);
    assert_eq!(app.diff.return_to, Screen::History);
}

// ── CommitDetails navigation ─────────────────────────────────────────

#[test]
fn commit_details_back_returns_to_history() {
    let mut app = App::new();
    app.screen = Screen::CommitDetails;

    let effect = app.handle_action(Action::Back);
    assert_eq!(effect, Effect::None);
    assert_eq!(app.screen, Screen::History);
}

#[test]
fn commit_details_quit_works() {
    let mut app = App::new();
    app.screen = Screen::CommitDetails;

    let effect = app.handle_action(Action::Quit);
    assert_eq!(effect, Effect::Quit);
}

#[test]
fn commit_details_d_opens_commit_diff_returns_to_details() {
    let mut app = App::new();
    app.screen = Screen::CommitDetails;
    app.commit_details.source_hash = "abc123".into();

    let effect = app.handle_action(Action::Preview);
    assert!(matches!(effect, Effect::LoadCommitDiff(ref h) if h == "abc123"));
    assert_eq!(app.screen, Screen::DiffPreview);
    assert_eq!(app.diff.return_to, Screen::CommitDetails);
}

#[test]
fn commit_details_d_no_op_when_no_source_hash() {
    let mut app = App::new();
    app.screen = Screen::CommitDetails;
    app.commit_details.source_hash.clear();

    let effect = app.handle_action(Action::Preview);
    assert_eq!(effect, Effect::None);
    // Screen should not change since there's nothing to load
    assert_eq!(app.screen, Screen::CommitDetails);
}

#[test]
fn commit_details_scroll_keys() {
    let mut app = App::new();
    app.screen = Screen::CommitDetails;
    app.commit_details.details.message = "line1\nline2\nline3".into();
    app.commit_details.details.files = vec![FileChange {
        path: "x.rs".into(),
        kind: ChangeKind::Modified,
    }];

    app.handle_action(Action::MoveDown);
    assert_eq!(app.commit_details.scroll, 2);
    app.handle_action(Action::MoveUp);
    assert_eq!(app.commit_details.scroll, 0);
}

// ── Diff return-to-source ────────────────────────────────────────────

#[test]
fn diff_back_returns_to_stage_by_default() {
    let mut app = App::new();
    app.screen = Screen::DiffPreview;
    // return_to defaults to Stage

    let effect = app.handle_action(Action::Back);
    assert_eq!(effect, Effect::None);
    assert_eq!(app.screen, Screen::Stage);
}

#[test]
fn diff_back_returns_to_history_when_set() {
    let mut app = App::new();
    app.screen = Screen::DiffPreview;
    app.diff.return_to = Screen::History;

    let effect = app.handle_action(Action::Back);
    assert_eq!(effect, Effect::None);
    assert_eq!(app.screen, Screen::History);
}

#[test]
fn diff_back_returns_to_commit_details_when_set() {
    let mut app = App::new();
    app.screen = Screen::DiffPreview;
    app.diff.return_to = Screen::CommitDetails;

    let effect = app.handle_action(Action::Back);
    assert_eq!(effect, Effect::None);
    assert_eq!(app.screen, Screen::CommitDetails);
}

// ── Security: commit details screen is read-only ─────────────────────

#[test]
fn commit_details_never_returns_mutating_effects() {
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
        app.screen = Screen::CommitDetails;
        app.commit_details.source_hash = "abc".into();

        let effect = app.handle_action(action);
        match effect {
            Effect::StageFiles(_) | Effect::CreateCommit(_) | Effect::SwitchBranch(_) => {
                panic!("CommitDetails returned mutating effect for action {action:?}");
            }
            _ => {}
        }
    }
}

// ── Data structures ──────────────────────────────────────────────────

#[test]
fn commit_details_empty_has_no_error() {
    let d = CommitDetails::empty();
    assert!(d.error.is_none());
    assert!(d.files.is_empty());
    assert!(!d.files_truncated);
}

// ── Helper ───────────────────────────────────────────────────────────

fn make_history(n: usize) -> HistoryResult {
    let entries: Vec<HistoryEntry> = (0..n)
        .map(|i| HistoryEntry {
            short_hash: format!("{i:07}"),
            full_hash: format!("{:040}", i),
            summary: format!("Commit {i}"),
            author: "Test".into(),
            timestamp: 0,
            relative_time: "recent".into(),
        })
        .collect();
    HistoryResult {
        entries,
        is_real: true,
        error: None,
    }
}
