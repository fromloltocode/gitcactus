//! Tests for the intro animation and diff preview features.

use gitcactus::app::{App, DiffState, Effect, IntroState, Screen, INTRO_TOTAL_FRAMES};
use gitcactus::git::diff::{DiffKind, DiffLine, DiffResult};
use gitcactus::input::Action;

// ── Intro state ──────────────────────────────────────────────────────

#[test]
fn intro_starts_at_frame_zero() {
    let state = IntroState::new();
    assert_eq!(state.frame, 0);
    assert!(!state.done);
}

#[test]
fn intro_tick_advances_frame() {
    let mut state = IntroState::new();
    assert!(!state.tick());
    assert_eq!(state.frame, 1);
    assert!(!state.tick());
    assert_eq!(state.frame, 2);
}

#[test]
fn intro_tick_finishes_at_total_frames() {
    let mut state = IntroState::new();
    for _ in 0..INTRO_TOTAL_FRAMES - 1 {
        assert!(!state.tick());
    }
    assert!(state.tick()); // should be done now
    assert!(state.done);
}

#[test]
fn intro_skip_sets_done_immediately() {
    let mut state = IntroState::new();
    state.skip();
    assert!(state.done);
}

#[test]
fn intro_tick_after_done_stays_done() {
    let mut state = IntroState::new();
    state.skip();
    assert!(state.tick());
    assert!(state.tick());
    assert!(state.done);
}

#[test]
fn app_starts_on_intro_screen() {
    let app = App::new();
    assert_eq!(app.screen, Screen::Intro);
}

#[test]
fn intro_screen_any_key_skips() {
    let mut app = App::new();
    assert_eq!(app.screen, Screen::Intro);

    let effect = app.handle_action(Action::Other);
    assert_eq!(effect, Effect::IntroFinished);
    assert!(app.intro.done);
}

#[test]
fn intro_screen_quit_also_skips() {
    let mut app = App::new();
    let effect = app.handle_action(Action::Quit);
    // Intro treats all keys as skip, including quit
    assert_eq!(effect, Effect::IntroFinished);
}

// ── Diff state ───────────────────────────────────────────────────────

#[test]
fn diff_state_starts_empty() {
    let state = DiffState::new();
    assert_eq!(state.scroll, 0);
    assert_eq!(state.result.kind, DiffKind::Empty);
    assert!(state.result.lines.is_empty());
}

#[test]
fn diff_scroll_up_clamps_at_zero() {
    let mut state = DiffState::new();
    state.scroll_up();
    assert_eq!(state.scroll, 0);
}

#[test]
fn diff_scroll_down_clamps_at_max() {
    let mut state = DiffState::new();
    // With empty lines, scroll should stay at 0
    state.scroll_down();
    assert_eq!(state.scroll, 0);
}

#[test]
fn diff_scroll_works_with_content() {
    let mut state = DiffState::new();
    state.result.lines = (0..100)
        .map(|i| DiffLine {
            origin: ' ',
            content: format!("line {i}"),
        })
        .collect();

    state.scroll_down();
    assert_eq!(state.scroll, 3);
    state.scroll_down();
    assert_eq!(state.scroll, 6);
    state.scroll_up();
    assert_eq!(state.scroll, 3);
    state.scroll_up();
    assert_eq!(state.scroll, 0);
}

// ── Diff preview screen: read-only guarantee ─────────────────────────

#[test]
fn diff_preview_never_returns_mutating_effects() {
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
        app.screen = Screen::DiffPreview;

        let effect = app.handle_action(action);
        match effect {
            Effect::None | Effect::Quit => {} // safe
            other => panic!(
                "DiffPreview returned mutating effect {other:?} for action {action:?}"
            ),
        }
    }
}

#[test]
fn diff_preview_back_returns_to_stage() {
    let mut app = App::new();
    app.screen = Screen::DiffPreview;

    let effect = app.handle_action(Action::Back);
    assert_eq!(effect, Effect::None);
    assert_eq!(app.screen, Screen::Stage);
}

// ── Diff data: edge cases ────────────────────────────────────────────

#[test]
fn diff_result_truncation_flag() {
    let result = DiffResult {
        file_path: "big.txt".to_string(),
        kind: DiffKind::Modified,
        lines: vec![],
        truncated: true,
    };
    assert!(result.truncated);
}

#[test]
fn diff_result_binary_has_no_lines() {
    let result = DiffResult {
        file_path: "image.png".to_string(),
        kind: DiffKind::Binary,
        lines: vec![],
        truncated: false,
    };
    assert_eq!(result.kind, DiffKind::Binary);
    assert!(result.lines.is_empty());
}

#[test]
fn diff_result_error_preserves_message() {
    let result = DiffResult {
        file_path: "missing.txt".to_string(),
        kind: DiffKind::Error("file not found".to_string()),
        lines: vec![],
        truncated: false,
    };
    assert!(matches!(result.kind, DiffKind::Error(ref s) if s.contains("not found")));
}

// ── Stage screen: 'd' opens diff preview ─────────────────────────────

#[test]
fn stage_preview_opens_diff_for_highlighted_file() {
    let mut app = App::new();
    app.screen = Screen::Stage;
    app.stage = gitcactus::app::StageState::from_repo(
        &gitcactus::git::status::RepoStatus {
            branch: "main".into(),
            modified: Some(1),
            staged: Some(0),
            untracked: Some(0),
            staged_files: vec![],
            modified_files: vec!["src/main.rs".to_string()],
            untracked_files: vec![],
            is_real: true,
        },
    );

    let effect = app.handle_action(Action::Preview);
    assert!(matches!(effect, Effect::LoadDiff(ref p) if p == "src/main.rs"));
    assert_eq!(app.screen, Screen::DiffPreview);
}

#[test]
fn stage_preview_does_nothing_when_empty() {
    let mut app = App::new();
    app.screen = Screen::Stage;
    // No entries

    let effect = app.handle_action(Action::Preview);
    assert_eq!(effect, Effect::None);
    assert_eq!(app.screen, Screen::Stage);
}

// ── Input: 'd' maps to Preview ──────────────────────────────────────

#[test]
fn d_maps_to_preview() {
    use crossterm::event::KeyCode;
    use gitcactus::input::map_key;
    assert_eq!(map_key(KeyCode::Char('d')), Action::Preview);
}

#[test]
fn d_types_char_in_text_mode() {
    use crossterm::event::KeyCode;
    use gitcactus::input::map_key_text;
    assert_eq!(map_key_text(KeyCode::Char('d')), Action::Char('d'));
}
