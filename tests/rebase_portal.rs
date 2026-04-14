//! Tests for the Rebase Portal preview + confirm + execute flow.
//!
//! Heavy emphasis on *no mutation before explicit confirmation* and on
//! the blocked / conflict / failure / abort paths.

use gitcactus::app::{
    App, Effect, RebaseExecuteMode, RebaseExecuteState, RebasePortalMode,
    RebasePortalState, Screen,
};
use gitcactus::git::branches::{BranchEntry, BranchListResult};
use gitcactus::git::rebase_execute::{
    self, abort_rebase, continue_rebase, execute_rebase, preflight,
    ContinueResult, ExecuteResult,
};
use gitcactus::git::rebase_preview::{
    preview_rebase, PreviewCommit, PreviewKind, RebasePreview,
};
use gitcactus::input::{map_key, Action};
use gitcactus::terminology::{TermMode, Terms};

// ── Terminology ──────────────────────────────────────────────────────

#[test]
fn portal_terminology_three_modes() {
    assert_eq!(Terms::new(TermMode::Beginner).rebase_action(), "Rebase Portal");
    assert_eq!(
        Terms::new(TermMode::Hybrid).rebase_action(),
        "Rebase Portal (Rebase)"
    );
    assert_eq!(Terms::new(TermMode::Git).rebase_action(), "Rebase");
}

#[test]
fn portal_execute_title_non_empty_all_modes() {
    for m in [TermMode::Beginner, TermMode::Hybrid, TermMode::Git] {
        let t = Terms::new(m);
        assert!(!t.title_rebase_execute().is_empty());
        assert!(!t.title_rebase_portal().is_empty());
        assert!(!t.rebase_description().is_empty());
    }
}

// ── Input ────────────────────────────────────────────────────────────

#[test]
fn p_maps_to_portal_action() {
    use crossterm::event::KeyCode;
    assert_eq!(map_key(KeyCode::Char('p')), Action::Portal);
}

// ── Preview safety / edge cases ──────────────────────────────────────

#[test]
fn preview_on_nonexistent_path_returns_not_a_repo() {
    let p = preview_rebase("/tmp/gitcactus-not-a-repo-xyz-9999", "main");
    assert_eq!(p.kind, PreviewKind::NotARepo);
    assert!(p.commits.is_empty());
}

#[test]
fn preview_is_executable_requires_ready_and_clean_tree() {
    let mut p = ready_preview();
    assert!(p.is_executable());

    p.dirty_tree = true;
    assert!(!p.is_executable(), "dirty tree must block execution");

    p.dirty_tree = false;
    p.kind = PreviewKind::NoCommitsToReplay;
    assert!(!p.is_executable(), "no commits must block execution");

    p.kind = PreviewKind::SameBranch;
    assert!(!p.is_executable());

    p.kind = PreviewKind::DetachedHead;
    assert!(!p.is_executable());
}

// ── Portal screen: navigation + confirmation flow ────────────────────

#[test]
fn portal_starts_in_preview_mode() {
    let s = RebasePortalState::new();
    assert_eq!(s.mode, RebasePortalMode::Preview);
    assert!(s.preview.is_none());
}

#[test]
fn portal_enter_does_not_confirm_when_not_executable() {
    let mut app = App::new();
    app.screen = Screen::RebasePortal;
    let mut p = ready_preview();
    p.dirty_tree = true;
    app.rebase_portal.preview = Some(p);

    let e = app.handle_action(Action::Select);
    assert_eq!(e, Effect::None);
    // Still in Preview — no confirmation is allowed.
    assert_eq!(app.rebase_portal.mode, RebasePortalMode::Preview);
}

#[test]
fn portal_enter_advances_to_confirm_when_executable() {
    let mut app = App::new();
    app.screen = Screen::RebasePortal;
    app.rebase_portal.preview = Some(ready_preview());

    let e = app.handle_action(Action::Select);
    assert_eq!(e, Effect::None);
    assert_eq!(app.rebase_portal.mode, RebasePortalMode::Confirm);
}

#[test]
fn portal_cancel_from_confirm_returns_to_preview() {
    let mut app = App::new();
    app.screen = Screen::RebasePortal;
    app.rebase_portal.preview = Some(ready_preview());
    app.rebase_portal.mode = RebasePortalMode::Confirm;

    let e = app.handle_action(Action::Deny);
    assert_eq!(e, Effect::None);
    assert_eq!(app.rebase_portal.mode, RebasePortalMode::Preview);

    app.rebase_portal.mode = RebasePortalMode::Confirm;
    let e = app.handle_action(Action::Back);
    assert_eq!(e, Effect::None);
    assert_eq!(app.rebase_portal.mode, RebasePortalMode::Preview);
}

#[test]
fn portal_confirm_fires_execute_effect_and_switches_screen() {
    let mut app = App::new();
    app.screen = Screen::RebasePortal;
    app.rebase_portal.preview = Some(ready_preview());
    app.rebase_portal.mode = RebasePortalMode::Confirm;

    let e = app.handle_action(Action::Confirm);
    assert!(matches!(e, Effect::ExecuteRebase(ref t) if t == "main"));
    assert_eq!(app.screen, Screen::RebaseExecute);
}

#[test]
fn portal_back_from_preview_returns_to_source_screen() {
    let mut app = App::new();
    app.screen = Screen::RebasePortal;
    app.rebase_portal.return_to = Screen::Branches;
    app.rebase_portal.preview = Some(ready_preview());

    let e = app.handle_action(Action::Back);
    assert_eq!(e, Effect::None);
    assert_eq!(app.screen, Screen::Branches);
}

// ── No mutation before explicit confirmation ─────────────────────────

#[test]
fn portal_never_produces_execute_rebase_from_preview_mode() {
    let actions = all_actions();
    for a in actions {
        let mut app = App::new();
        app.screen = Screen::RebasePortal;
        app.rebase_portal.preview = Some(ready_preview());
        app.rebase_portal.mode = RebasePortalMode::Preview;

        let e = app.handle_action(a);
        if matches!(e, Effect::ExecuteRebase(_)) {
            panic!(
                "Preview mode produced ExecuteRebase for action {a:?} — must not fire without \
                 user moving to Confirm mode first."
            );
        }
    }
}

#[test]
fn portal_preview_mode_never_mutates_repo() {
    let actions = all_actions();
    for a in actions {
        let mut app = App::new();
        app.screen = Screen::RebasePortal;
        app.rebase_portal.preview = Some(ready_preview());

        let e = app.handle_action(a);
        if is_mutating(&e) {
            panic!("Preview mode leaked mutating effect {e:?} for action {a:?}");
        }
    }
}

// ── Entry flow from Branches screen ──────────────────────────────────

#[test]
fn branches_p_on_non_current_opens_portal() {
    let mut app = App::new();
    app.screen = Screen::Branches;
    app.branches_state.branches = BranchListResult {
        branches: vec![
            BranchEntry {
                name: "main".into(),
                is_current: true,
                short_hash: "aaa0000".into(),
                summary: "baseline".into(),
            },
            BranchEntry {
                name: "feature".into(),
                is_current: false,
                short_hash: "bbb0000".into(),
                summary: "work".into(),
            },
        ],
        error: None,
    };
    app.branches_state.cursor = 1;

    let e = app.handle_action(Action::Portal);
    assert!(matches!(e, Effect::LoadRebasePreview(ref t) if t == "feature"));
    assert_eq!(app.screen, Screen::RebasePortal);
    assert_eq!(app.rebase_portal.return_to, Screen::Branches);
}

#[test]
fn branches_p_on_current_branch_is_noop() {
    let mut app = App::new();
    app.screen = Screen::Branches;
    app.branches_state.branches = BranchListResult {
        branches: vec![BranchEntry {
            name: "main".into(),
            is_current: true,
            short_hash: "a".into(),
            summary: String::new(),
        }],
        error: None,
    };
    let e = app.handle_action(Action::Portal);
    assert_eq!(e, Effect::None);
    assert_eq!(app.screen, Screen::Branches);
}

// ── Execute state: outcome handling ──────────────────────────────────

#[test]
fn execute_state_success_starts_in_animating_mode_when_commits_present() {
    let preview = ready_preview();
    let state = RebaseExecuteState::from_outcome(&preview, &ExecuteResult::Success);
    assert!(matches!(state.mode, RebaseExecuteMode::Animating));
    assert!(!state.fell);
}

#[test]
fn execute_state_conflict_sets_fell_and_captures_stderr() {
    let preview = ready_preview();
    let state = RebaseExecuteState::from_outcome(
        &preview,
        &ExecuteResult::Conflict {
            stderr: "Auto-merging file.txt\nCONFLICT (content): Merge conflict".into(),
        },
    );
    assert!(matches!(state.mode, RebaseExecuteMode::Conflict { .. }));
    assert!(state.fell);
}

#[test]
fn execute_state_failure_is_non_animating() {
    let preview = ready_preview();
    let state = RebaseExecuteState::from_outcome(
        &preview,
        &ExecuteResult::Failure { message: "fatal: boom".into() },
    );
    assert!(matches!(state.mode, RebaseExecuteMode::Failure { .. }));
}

#[test]
fn execute_state_blocked_surfaces_reason_as_failure() {
    let preview = ready_preview();
    let state = RebaseExecuteState::from_outcome(
        &preview,
        &ExecuteResult::Blocked {
            reason: "Working tree has uncommitted changes.".into(),
        },
    );
    assert!(matches!(state.mode, RebaseExecuteMode::Failure { .. }));
    // Blocked path didn't actually run anything, so don't mark as fallen.
    assert!(!state.fell);
}

// ── Animation behavior ───────────────────────────────────────────────

#[test]
fn animation_advances_one_step_at_a_time() {
    let preview = ready_preview_with_n(3);
    let mut state = RebaseExecuteState::from_outcome(&preview, &ExecuteResult::Success);
    assert_eq!(state.step, 0);

    let done = state.tick();
    assert!(!done);
    assert_eq!(state.step, 1);

    let done = state.tick();
    assert!(!done);
    assert_eq!(state.step, 2);

    let done = state.tick();
    assert!(done);
    assert!(matches!(state.mode, RebaseExecuteMode::Success));
}

#[test]
fn animation_skip_jumps_to_end() {
    let preview = ready_preview_with_n(10);
    let mut state = RebaseExecuteState::from_outcome(&preview, &ExecuteResult::Success);
    state.skip_animation();
    assert!(matches!(state.mode, RebaseExecuteMode::Success));
    assert_eq!(state.step, 9);
}

#[test]
fn animation_tick_is_noop_in_non_animating_mode() {
    let preview = ready_preview();
    let mut state = RebaseExecuteState::from_outcome(
        &preview,
        &ExecuteResult::Failure { message: "x".into() },
    );
    assert!(state.tick(), "tick in non-animating mode should report done immediately");
    assert!(matches!(state.mode, RebaseExecuteMode::Failure { .. }));
}

// ── Execute screen: navigation / abort / safety ──────────────────────

#[test]
fn execute_success_enter_returns_to_branches() {
    let mut app = App::new();
    app.screen = Screen::RebaseExecute;
    app.rebase_execute.mode = RebaseExecuteMode::Success;

    let e = app.handle_action(Action::Select);
    assert!(matches!(e, Effect::LoadBranches));
    assert_eq!(app.screen, Screen::Branches);
}

#[test]
fn execute_conflict_a_fires_abort_effect() {
    let mut app = App::new();
    app.screen = Screen::RebaseExecute;
    app.rebase_execute.mode = RebaseExecuteMode::Conflict {
        stderr: "conflict".into(),
    };

    let e = app.handle_action(Action::ToggleAll); // 'a' maps to ToggleAll
    assert_eq!(e, Effect::AbortRebase);
}

#[test]
fn execute_conflict_esc_returns_without_aborting() {
    let mut app = App::new();
    app.screen = Screen::RebaseExecute;
    app.rebase_execute.mode = RebaseExecuteMode::Conflict {
        stderr: String::new(),
    };

    let e = app.handle_action(Action::Back);
    // Esc returns to branches but does NOT abort — user may resolve manually.
    assert!(matches!(e, Effect::LoadBranches));
    assert_eq!(app.screen, Screen::Branches);
    assert!(!matches!(e, Effect::AbortRebase));
}

#[test]
fn execute_animating_any_key_skips_not_exits() {
    let mut app = App::new();
    app.screen = Screen::RebaseExecute;
    let preview = ready_preview_with_n(4);
    app.rebase_execute = RebaseExecuteState::from_outcome(&preview, &ExecuteResult::Success);
    assert!(matches!(app.rebase_execute.mode, RebaseExecuteMode::Animating));

    let e = app.handle_action(Action::Back);
    assert_eq!(e, Effect::None, "any key while animating skips; does not leave the screen");
    assert!(matches!(app.rebase_execute.mode, RebaseExecuteMode::Success));
    assert_eq!(app.screen, Screen::RebaseExecute);
}

#[test]
fn execute_failure_enter_returns_to_branches() {
    let mut app = App::new();
    app.screen = Screen::RebaseExecute;
    app.rebase_execute.mode = RebaseExecuteMode::Failure {
        message: "boom".into(),
    };

    let e = app.handle_action(Action::Select);
    assert!(matches!(e, Effect::LoadBranches));
    assert_eq!(app.screen, Screen::Branches);
}

// ── Security: no unsafe shortcuts ────────────────────────────────────

#[test]
fn execute_screen_never_fires_execute_rebase_from_any_mode() {
    // The only path to Effect::ExecuteRebase is the explicit Confirm
    // in RebasePortalMode::Confirm. The execute screen itself must never
    // re-trigger a rebase — even by accident on Refresh or Confirm.
    let modes = [
        RebaseExecuteMode::Success,
        RebaseExecuteMode::Conflict { stderr: String::new() },
        RebaseExecuteMode::Failure { message: String::new() },
        RebaseExecuteMode::Aborted { message: String::new(), ok: true },
    ];
    for mode in modes {
        for a in all_actions() {
            let mut app = App::new();
            app.screen = Screen::RebaseExecute;
            app.rebase_execute.mode = mode.clone();

            let e = app.handle_action(a);
            if matches!(e, Effect::ExecuteRebase(_)) {
                panic!("Execute screen leaked ExecuteRebase for action {a:?} in mode {mode:?}");
            }
            if matches!(e, Effect::StageFiles(_) | Effect::CreateCommit(_)
                            | Effect::SwitchBranch(_) | Effect::CreateBranch(_)) {
                panic!("Execute screen leaked other mutating effect {e:?} for action {a:?}");
            }
        }
    }
}

#[test]
fn execute_animating_mode_never_fires_any_mutation() {
    for a in all_actions() {
        let mut app = App::new();
        app.screen = Screen::RebaseExecute;
        let preview = ready_preview_with_n(3);
        app.rebase_execute = RebaseExecuteState::from_outcome(&preview, &ExecuteResult::Success);

        let e = app.handle_action(a);
        if is_mutating(&e) {
            panic!("Animating mode leaked mutating effect {e:?} for action {a:?}");
        }
    }
}

// ── Preflight / execute / abort (shell-outs) ─────────────────────────
//
// These verify that the public API is reachable and that preflight
// rejects invalid paths without ever invoking `git`. The real rebase
// behaviour is exercised manually (see docs/manual-rebase-check.md).

#[test]
fn preflight_rejects_nonexistent_path() {
    let r = preflight("/tmp/gitcactus-not-a-repo-xyz-9999", "main");
    // Must be Some(_) — nothing should silently proceed.
    assert!(r.is_some(), "preflight must block non-repo paths");
}

#[test]
fn preflight_returns_blocked_for_detached_head_or_missing_target_etc() {
    // We can at least verify that for a non-repo path we get Failure
    // (mapping to Blocked in the screen).
    let r = preflight("/tmp/gitcactus-not-a-repo-xyz-9999", "main");
    match r {
        Some(rebase_execute::ExecuteResult::Failure { .. }) => {}
        Some(rebase_execute::ExecuteResult::Blocked { .. }) => {}
        other => panic!("expected Failure or Blocked, got {other:?}"),
    }
}

#[test]
fn execute_on_nonexistent_path_fails_cleanly() {
    let r = execute_rebase("/tmp/gitcactus-not-a-repo-xyz-9999", "main");
    // Invoking git in a non-repo directory fails; we must surface that.
    assert!(matches!(
        r,
        ExecuteResult::Failure { .. } | ExecuteResult::Conflict { .. }
    ));
}

#[test]
fn abort_on_nonexistent_path_returns_error() {
    let r = abort_rebase("/tmp/gitcactus-not-a-repo-xyz-9999");
    assert!(r.is_err());
}

// ── Continue Rebase: safety gate + input mapping ─────────────────────

#[test]
fn c_maps_to_continue_action() {
    use crossterm::event::KeyCode;
    assert_eq!(map_key(KeyCode::Char('c')), Action::Continue);
}

#[test]
fn continue_refuses_when_not_in_rebase() {
    // No rebase in progress under a nonexistent path → must refuse,
    // never try to "recover" into some other state.
    let r = continue_rebase("/tmp/gitcactus-not-a-repo-xyz-9999");
    assert_eq!(r, ContinueResult::NotInRebase);
}

#[test]
fn conflict_screen_c_fires_continue_rebase_effect() {
    let mut app = App::new();
    app.screen = Screen::RebaseExecute;
    app.rebase_execute.mode = RebaseExecuteMode::Conflict {
        stderr: "CONFLICT (content): merge conflict in a.txt".into(),
    };

    let e = app.handle_action(Action::Continue);
    assert_eq!(e, Effect::ContinueRebase);
}

#[test]
fn conflict_screen_a_still_fires_abort_rebase_effect() {
    let mut app = App::new();
    app.screen = Screen::RebaseExecute;
    app.rebase_execute.mode = RebaseExecuteMode::Conflict {
        stderr: String::new(),
    };

    // 'a' maps to ToggleAll in command-mode key mapping.
    let e = app.handle_action(Action::ToggleAll);
    assert_eq!(e, Effect::AbortRebase);
}

#[test]
fn conflict_screen_esc_leaves_without_aborting_or_continuing() {
    let mut app = App::new();
    app.screen = Screen::RebaseExecute;
    app.rebase_execute.mode = RebaseExecuteMode::Conflict {
        stderr: String::new(),
    };

    let e = app.handle_action(Action::Back);
    // Esc returns to branches. It never fires Abort or Continue.
    assert!(matches!(e, Effect::LoadBranches));
    assert!(!matches!(e, Effect::AbortRebase | Effect::ContinueRebase));
}

// ── State-machine sanity: every mode reacts correctly ────────────────

#[test]
fn conflict_mode_only_fires_abort_or_continue_on_right_keys() {
    for action in all_actions() {
        let mut app = App::new();
        app.screen = Screen::RebaseExecute;
        app.rebase_execute.mode = RebaseExecuteMode::Conflict {
            stderr: String::new(),
        };

        let e = app.handle_action(action);
        match (action, &e) {
            (Action::Continue, Effect::ContinueRebase) => {}
            (Action::ToggleAll, Effect::AbortRebase) => {}
            (_, Effect::ContinueRebase) => {
                panic!("ContinueRebase leaked for non-Continue action {action:?}");
            }
            (_, Effect::AbortRebase) => {
                panic!("AbortRebase leaked for non-ToggleAll action {action:?}");
            }
            _ => {}
        }
    }
}

#[test]
fn success_and_aborted_modes_never_fire_continue() {
    // Only the Conflict state is allowed to issue Continue. Once the
    // rebase is done (Success) or rolled back (Aborted) a stray 'c'
    // press must be a no-op.
    let non_conflict_modes = [
        RebaseExecuteMode::Success,
        RebaseExecuteMode::Aborted {
            message: String::new(),
            ok: true,
        },
        RebaseExecuteMode::Aborted {
            message: String::new(),
            ok: false,
        },
        RebaseExecuteMode::Failure {
            message: String::new(),
        },
    ];
    for mode in non_conflict_modes {
        let mut app = App::new();
        app.screen = Screen::RebaseExecute;
        app.rebase_execute.mode = mode.clone();
        let e = app.handle_action(Action::Continue);
        assert_ne!(
            e,
            Effect::ContinueRebase,
            "ContinueRebase leaked from mode {mode:?}"
        );
    }
}

#[test]
fn animating_mode_any_input_just_skips_never_mutates() {
    for action in all_actions() {
        let mut app = App::new();
        app.screen = Screen::RebaseExecute;
        let preview = ready_preview_with_n(3);
        app.rebase_execute =
            RebaseExecuteState::from_outcome(&preview, &ExecuteResult::Success);
        assert!(matches!(app.rebase_execute.mode, RebaseExecuteMode::Animating));

        let e = app.handle_action(action);
        assert_eq!(e, Effect::None);
        if is_mutating(&e) {
            panic!("Animating mode leaked mutating effect {e:?} for action {action:?}");
        }
    }
}

// ── Preview states: exhaustive safety sweep (strengthened) ───────────

#[test]
fn no_preview_kind_lets_preview_mode_execute() {
    let kinds = [
        PreviewKind::NoCommitsToReplay,
        PreviewKind::SameBranch,
        PreviewKind::DetachedHead,
        PreviewKind::BranchMissing,
        PreviewKind::EmptyRepo,
        PreviewKind::NotARepo,
        PreviewKind::Error("boom".into()),
    ];
    for kind in kinds {
        for action in all_actions() {
            let mut app = App::new();
            app.screen = Screen::RebasePortal;
            let mut p = ready_preview();
            p.kind = kind.clone();
            p.commits.clear();
            app.rebase_portal.preview = Some(p);
            app.rebase_portal.mode = RebasePortalMode::Preview;

            let e = app.handle_action(action);
            assert!(
                !matches!(e, Effect::ExecuteRebase(_)),
                "ExecuteRebase leaked for preview kind {kind:?} with action {action:?}"
            );
        }
    }
}

#[test]
fn dirty_tree_blocks_execute_even_with_ready_kind() {
    let mut app = App::new();
    app.screen = Screen::RebasePortal;
    let mut p = ready_preview();
    p.dirty_tree = true;
    app.rebase_portal.preview = Some(p);

    let e = app.handle_action(Action::Select);
    assert_eq!(e, Effect::None);
    assert_eq!(app.rebase_portal.mode, RebasePortalMode::Preview);
}

// ── Helpers ──────────────────────────────────────────────────────────

fn ready_preview() -> RebasePreview {
    ready_preview_with_n(1)
}

fn ready_preview_with_n(n: usize) -> RebasePreview {
    RebasePreview {
        source: "feature".into(),
        target: "main".into(),
        source_tip: "aaaaaaa".into(),
        target_tip: "bbbbbbb".into(),
        merge_base: Some("ccccccc".into()),
        commits: (0..n)
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
    }
}

fn all_actions() -> Vec<Action> {
    vec![
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
        Action::Search,
        Action::Open,
        Action::Portal,
        Action::Continue,
        Action::Char('x'),
        Action::Backspace,
        Action::Other,
    ]
}

fn is_mutating(e: &Effect) -> bool {
    matches!(
        e,
        Effect::StageFiles(_)
            | Effect::CreateCommit(_)
            | Effect::SwitchBranch(_)
            | Effect::CreateBranch(_)
            | Effect::ExecuteRebase(_)
            | Effect::AbortRebase
            | Effect::ContinueRebase
    )
}
