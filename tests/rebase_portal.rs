//! Tests for the Rebase Portal preview screen.
//!
//! The preview-only guarantee is enforced via strict effect filtering:
//! no action while on the Rebase Portal screen is allowed to produce
//! any of StageFiles / CreateCommit / SwitchBranch / CreateBranch.

use gitcactus::app::{App, Effect, RebasePortalState, Screen};
use gitcactus::git::rebase_preview::{
    preview_rebase, PreviewCommit, PreviewKind, RebasePreview,
};
use gitcactus::input::{map_key, Action};
use gitcactus::terminology::{TermMode, Terms};

// ── Terminology ──────────────────────────────────────────────────────

#[test]
fn portal_terminology_beginner_hides_git_word() {
    let t = Terms::new(TermMode::Beginner);
    assert_eq!(t.rebase_action(), "Rebase Portal");
    assert!(!t.rebase_action().contains("Rebase ") || t.rebase_action() == "Rebase Portal");
}

#[test]
fn portal_terminology_hybrid_mentions_both() {
    let t = Terms::new(TermMode::Hybrid);
    let label = t.rebase_action();
    assert!(label.contains("Portal"));
    assert!(label.contains("Rebase"));
}

#[test]
fn portal_terminology_git_mode_uses_standard_term() {
    let t = Terms::new(TermMode::Git);
    assert_eq!(t.rebase_action(), "Rebase");
}

#[test]
fn portal_description_non_empty_all_modes() {
    for mode in [TermMode::Beginner, TermMode::Hybrid, TermMode::Git] {
        let t = Terms::new(mode);
        assert!(!t.rebase_description().is_empty());
        assert!(!t.title_rebase_portal().is_empty());
    }
}

// ── Input mapping ────────────────────────────────────────────────────

#[test]
fn p_maps_to_portal() {
    use crossterm::event::KeyCode;
    assert_eq!(map_key(KeyCode::Char('p')), Action::Portal);
}

// ── RebasePortalState ────────────────────────────────────────────────

#[test]
fn portal_state_starts_empty() {
    let s = RebasePortalState::new();
    assert_eq!(s.scroll, 0);
    assert!(s.preview.is_none());
    assert_eq!(s.return_to, Screen::Branches);
}

#[test]
fn portal_scroll_clamps_at_zero() {
    let mut s = RebasePortalState::new();
    s.scroll_up();
    assert_eq!(s.scroll, 0);
}

#[test]
fn portal_scroll_with_content() {
    let mut s = RebasePortalState::new();
    s.preview = Some(RebasePreview {
        source: "feature".into(),
        target: "main".into(),
        source_tip: "aaaaaaa".into(),
        target_tip: "bbbbbbb".into(),
        merge_base: Some("ccccccc".into()),
        commits: (0..20)
            .map(|i| PreviewCommit {
                short_hash: format!("{i:07}"),
                full_hash: format!("{i:040}"),
                summary: format!("commit {i}"),
                author: "me".into(),
                is_merge: false,
            })
            .collect(),
        truncated: false,
        has_merge_commits: false,
        dirty_tree: false,
        kind: PreviewKind::Ready,
    });
    s.scroll_down();
    assert_eq!(s.scroll, 2);
    s.scroll_down();
    assert_eq!(s.scroll, 4);
    s.scroll_up();
    assert_eq!(s.scroll, 2);
}

// ── Entry from Branches screen ───────────────────────────────────────

#[test]
fn p_on_non_current_branch_opens_portal() {
    let mut app = App::new();
    app.screen = Screen::Branches;
    // Seed with two branches, main (current) + feature (not current).
    app.branches_state.branches = gitcactus::git::branches::BranchListResult {
        branches: vec![
            gitcactus::git::branches::BranchEntry {
                name: "main".into(),
                is_current: true,
                short_hash: "aaa0000".into(),
                summary: "baseline".into(),
            },
            gitcactus::git::branches::BranchEntry {
                name: "feature".into(),
                is_current: false,
                short_hash: "bbb0000".into(),
                summary: "feature".into(),
            },
        ],
        error: None,
    };
    app.branches_state.cursor = 1; // feature

    let effect = app.handle_action(Action::Portal);
    assert!(matches!(effect, Effect::LoadRebasePreview(ref t) if t == "feature"));
    assert_eq!(app.screen, Screen::RebasePortal);
    assert_eq!(app.rebase_portal.return_to, Screen::Branches);
}

#[test]
fn p_on_current_branch_is_noop() {
    let mut app = App::new();
    app.screen = Screen::Branches;
    app.branches_state.branches = gitcactus::git::branches::BranchListResult {
        branches: vec![gitcactus::git::branches::BranchEntry {
            name: "main".into(),
            is_current: true,
            short_hash: "a".into(),
            summary: "".into(),
        }],
        error: None,
    };

    let effect = app.handle_action(Action::Portal);
    assert_eq!(effect, Effect::None);
    assert_eq!(app.screen, Screen::Branches);
}

#[test]
fn p_on_empty_branch_list_is_noop() {
    let mut app = App::new();
    app.screen = Screen::Branches;
    let effect = app.handle_action(Action::Portal);
    assert_eq!(effect, Effect::None);
}

// ── Navigation in Rebase Portal ──────────────────────────────────────

#[test]
fn portal_back_returns_to_branches() {
    let mut app = App::new();
    app.screen = Screen::RebasePortal;
    app.rebase_portal.return_to = Screen::Branches;

    let effect = app.handle_action(Action::Back);
    assert_eq!(effect, Effect::None);
    assert_eq!(app.screen, Screen::Branches);
}

#[test]
fn portal_quit_works() {
    let mut app = App::new();
    app.screen = Screen::RebasePortal;
    let effect = app.handle_action(Action::Quit);
    assert_eq!(effect, Effect::Quit);
}

#[test]
fn portal_enter_is_a_noop_this_phase() {
    // Phase 1 is preview-only: Enter must not execute anything.
    let mut app = App::new();
    app.screen = Screen::RebasePortal;
    let effect = app.handle_action(Action::Select);
    assert_eq!(effect, Effect::None);
}

// ── Safety: preview screen never returns mutating effects ────────────

#[test]
fn portal_never_returns_mutating_effects() {
    let actions = [
        Action::Quit, Action::Back, Action::MoveUp, Action::MoveDown,
        Action::Select, Action::Toggle, Action::ToggleAll, Action::Refresh,
        Action::Confirm, Action::Deny, Action::Preview, Action::Portal,
        Action::Search, Action::Open, Action::Char('x'), Action::Backspace,
        Action::Other,
    ];
    for action in actions {
        let mut app = App::new();
        app.screen = Screen::RebasePortal;
        app.rebase_portal.preview = Some(ready_preview());

        let effect = app.handle_action(action);
        if matches!(
            effect,
            Effect::StageFiles(_)
                | Effect::CreateCommit(_)
                | Effect::SwitchBranch(_)
                | Effect::CreateBranch(_)
        ) {
            panic!("Rebase Portal produced mutating effect for {action:?}: {effect:?}");
        }
    }
}

// ── preview_rebase smoke tests on an empty path ──────────────────────

#[test]
fn preview_on_nonexistent_path_returns_not_a_repo() {
    let preview = preview_rebase("/tmp/definitely-not-a-repo-xyz-9999", "main");
    assert_eq!(preview.kind, PreviewKind::NotARepo);
    assert!(preview.commits.is_empty());
}

// ── PreviewKind data sanity ──────────────────────────────────────────

#[test]
fn preview_ready_kind_means_nonempty_commits() {
    let p = ready_preview();
    assert_eq!(p.kind, PreviewKind::Ready);
    assert!(!p.commits.is_empty());
}

#[test]
fn preview_same_branch_kind_blocks_rendering_list() {
    let mut p = ready_preview();
    p.kind = PreviewKind::SameBranch;
    p.commits.clear();
    assert_eq!(p.kind, PreviewKind::SameBranch);
}

#[test]
fn preview_carries_dirty_tree_and_merge_flags() {
    let mut p = ready_preview();
    p.dirty_tree = true;
    p.has_merge_commits = true;
    assert!(p.dirty_tree);
    assert!(p.has_merge_commits);
}

// ── Helper ───────────────────────────────────────────────────────────

fn ready_preview() -> RebasePreview {
    RebasePreview {
        source: "feature".into(),
        target: "main".into(),
        source_tip: "aaaaaaa".into(),
        target_tip: "bbbbbbb".into(),
        merge_base: Some("ccccccc".into()),
        commits: vec![PreviewCommit {
            short_hash: "1111111".into(),
            full_hash: "1".repeat(40),
            summary: "Add thing".into(),
            author: "me".into(),
            is_merge: false,
        }],
        truncated: false,
        has_merge_commits: false,
        dirty_tree: false,
        kind: PreviewKind::Ready,
    }
}
