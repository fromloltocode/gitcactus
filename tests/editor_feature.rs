//! Tests for the "Open in Editor" feature.

use crossterm::event::KeyCode;
use gitcactus::app::{App, Effect, Screen, StageEntry, StageFileKind, StageState};
use gitcactus::editor::{self, EditorResult};
use gitcactus::git::commit_details::{ChangeKind, CommitDetails, FileChange};
use gitcactus::input::{map_key, map_key_text, Action};

// ── Input mapping ────────────────────────────────────────────────────

#[test]
fn o_maps_to_open_in_command_mode() {
    assert_eq!(map_key(KeyCode::Char('o')), Action::Open);
}

#[test]
fn o_types_as_char_in_text_mode() {
    assert_eq!(map_key_text(KeyCode::Char('o')), Action::Char('o'));
}

// ── Stage screen: o → OpenEditor on highlighted file ─────────────────

#[test]
fn stage_o_opens_highlighted_file() {
    let mut app = App::new();
    app.screen = Screen::Stage;
    app.stage = StageState {
        entries: vec![
            StageEntry {
                path: "src/main.rs".into(),
                kind: StageFileKind::Modified,
                selected: false,
            },
            StageEntry {
                path: "README.md".into(),
                kind: StageFileKind::Modified,
                selected: false,
            },
        ],
        already_staged: vec![],
        cursor: 1,
        mode: gitcactus::app::StageMode::Browse,
        result_msg: None,
    };

    let effect = app.handle_action(Action::Open);
    assert!(matches!(effect, Effect::OpenEditor(ref p) if p == "README.md"));
    // Stage screen should NOT change to a different screen.
    assert_eq!(app.screen, Screen::Stage);
}

#[test]
fn stage_o_noop_when_empty() {
    let mut app = App::new();
    app.screen = Screen::Stage;

    let effect = app.handle_action(Action::Open);
    assert_eq!(effect, Effect::None);
}

// ── CommitDetails screen: o → OpenEditor on first changed file ───────

#[test]
fn commit_details_o_opens_first_file() {
    let mut app = App::new();
    app.screen = Screen::CommitDetails;
    app.commit_details.details = CommitDetails {
        short_hash: "abc1234".into(),
        full_hash: "abc1234".into(),
        author: "Me".into(),
        email: String::new(),
        timestamp: 0,
        relative_time: "recent".into(),
        message: "msg".into(),
        files: vec![
            FileChange {
                path: "src/lib.rs".into(),
                kind: ChangeKind::Modified,
            },
            FileChange {
                path: "src/main.rs".into(),
                kind: ChangeKind::Modified,
            },
        ],
        files_truncated: false,
        error: None,
    };

    let effect = app.handle_action(Action::Open);
    assert!(matches!(effect, Effect::OpenEditor(ref p) if p == "src/lib.rs"));
    assert_eq!(app.screen, Screen::CommitDetails);
}

#[test]
fn commit_details_o_noop_when_no_files() {
    let mut app = App::new();
    app.screen = Screen::CommitDetails;
    app.commit_details.details = CommitDetails::empty();

    let effect = app.handle_action(Action::Open);
    assert_eq!(effect, Effect::None);
}

// ── Editor module: validation ────────────────────────────────────────

#[test]
fn open_nonexistent_file_returns_file_not_found() {
    let result = editor::open_in_editor("/this/path/definitely/does/not/exist/gitcactus-test.txt");
    assert!(matches!(result, EditorResult::FileNotFound(_)));
}

#[test]
fn open_binary_file_is_refused() {
    // Create a tmp file with a null byte in the first 8KB.
    let tmp = std::env::temp_dir().join("gitcactus_test_binary");
    std::fs::write(&tmp, b"abc\0def").expect("write temp file");

    let result = editor::open_in_editor(tmp.to_str().unwrap());
    let _ = std::fs::remove_file(&tmp);

    assert!(
        matches!(result, EditorResult::BinaryFile(_)),
        "Expected BinaryFile, got {result:?}"
    );
}

#[test]
fn open_text_file_with_no_editor_returns_no_editor() {
    // Force EDITOR and PATH to be empty so no editor can be found.
    let saved_editor = std::env::var_os("EDITOR");
    let saved_path = std::env::var_os("PATH");
    std::env::set_var("EDITOR", "");
    std::env::set_var("PATH", "");

    let tmp = std::env::temp_dir().join("gitcactus_test_text");
    std::fs::write(&tmp, b"hello world\n").expect("write temp file");

    let result = editor::open_in_editor(tmp.to_str().unwrap());

    let _ = std::fs::remove_file(&tmp);
    // Restore env.
    match saved_editor {
        Some(v) => std::env::set_var("EDITOR", v),
        None => std::env::remove_var("EDITOR"),
    }
    match saved_path {
        Some(v) => std::env::set_var("PATH", v),
        None => std::env::remove_var("PATH"),
    }

    assert!(
        matches!(result, EditorResult::NoEditor),
        "Expected NoEditor, got {result:?}"
    );
}

#[test]
fn result_message_all_variants_human_readable() {
    let cases = [
        (EditorResult::Ok, true),
        (EditorResult::FileNotFound("foo".into()), false),
        (EditorResult::NoEditor, false),
        (EditorResult::BinaryFile("img.png".into()), false),
        (EditorResult::LaunchFailed("x".into()), false),
    ];
    for (r, expected_ok) in cases {
        let (msg, is_ok) = editor::result_message(&r);
        assert!(!msg.is_empty());
        assert_eq!(is_ok, expected_ok, "result_message is_ok mismatch for {r:?}");
    }
}

// ── App state: editor_msg ────────────────────────────────────────────

#[test]
fn app_starts_with_no_editor_msg() {
    let app = App::new();
    assert!(app.editor_msg.is_none());
}

// ── Security: OpenEditor never mutates repo state ────────────────────

#[test]
fn open_editor_effect_is_not_a_repo_mutation() {
    // OpenEditor is read-only with respect to the repository — the
    // filesystem-level edit happens in the user's editor, outside our
    // control. The Effect variant itself must not be confused with the
    // repo-mutating variants.
    let open = Effect::OpenEditor("x".into());
    assert!(!matches!(
        open,
        Effect::StageFiles(_)
            | Effect::CreateCommit(_)
            | Effect::SwitchBranch(_)
            | Effect::CreateBranch(_)
    ));
}
