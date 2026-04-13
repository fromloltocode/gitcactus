//! Security tests: verify that no screen can produce unintended
//! side-effects from any action. Read-only screens must never return
//! effects that mutate the repository.

use gitcactus::app::{App, Effect, Screen};
use gitcactus::input::Action;

/// All possible actions to test exhaustively.
const ALL_ACTIONS: &[Action] = &[
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
    Action::Char('x'),
    Action::Char(' '),
    Action::Char('\n'),
    Action::Preview,
    Action::Backspace,
    Action::Other,
];

/// Screens that should never produce repo-mutating effects.
const READ_ONLY_SCREENS: &[Screen] = &[
    Screen::Title,
    Screen::Help,
    Screen::DiffPreview,
    Screen::Branches,
    Screen::History,
    Screen::RemoteSync,
];

#[test]
fn read_only_screens_never_return_mutating_effects() {
    for &screen in READ_ONLY_SCREENS {
        for &action in ALL_ACTIONS {
            let mut app = App::new();
            app.screen = screen;

            let effect = app.handle_action(action);

            match effect {
                Effect::None | Effect::Quit => {} // safe
                other => panic!(
                    "Screen {screen:?} returned mutating effect {other:?} for action {action:?}"
                ),
            }
        }
    }
}

#[test]
fn menu_never_stages_or_commits() {
    for &action in ALL_ACTIONS {
        let mut app = App::new();
        app.screen = Screen::Menu;

        let effect = app.handle_action(action);

        match effect {
            Effect::StageFiles(_) => {
                panic!("Menu produced StageFiles for action {action:?}");
            }
            Effect::CreateCommit(_) => {
                panic!("Menu produced CreateCommit for action {action:?}");
            }
            _ => {} // other effects are fine for the menu
        }
    }
}

#[test]
fn status_screen_never_stages_or_commits() {
    for &action in ALL_ACTIONS {
        let mut app = App::new();
        app.screen = Screen::Status;

        let effect = app.handle_action(action);

        match effect {
            Effect::StageFiles(_) | Effect::CreateCommit(_) => {
                panic!("Status screen produced mutating effect for action {action:?}");
            }
            _ => {}
        }
    }
}

#[test]
fn update_screen_never_stages_or_commits() {
    for &action in ALL_ACTIONS {
        let mut app = App::new();
        app.screen = Screen::Update;

        let effect = app.handle_action(action);

        match effect {
            Effect::StageFiles(_) | Effect::CreateCommit(_) => {
                panic!("Update screen produced mutating effect for action {action:?}");
            }
            _ => {}
        }
    }
}

#[test]
fn commit_screen_cannot_commit_with_empty_message() {
    let mut app = App::new();
    app.screen = Screen::Commit;
    app.commit.staged_count = 5;
    app.commit.message.clear();

    // Try to confirm — should not produce CreateCommit
    let effect = app.handle_action(Action::Select);
    assert_eq!(effect, Effect::None, "Empty message should not allow commit");
}

#[test]
fn commit_screen_cannot_commit_with_whitespace_only_message() {
    let mut app = App::new();
    app.screen = Screen::Commit;
    app.commit.staged_count = 5;
    app.commit.message = "   \t  ".to_string();

    let effect = app.handle_action(Action::Select);
    assert_eq!(effect, Effect::None, "Whitespace-only message should not allow commit");
}

#[test]
fn commit_screen_cannot_commit_with_no_staged_files() {
    let mut app = App::new();
    app.screen = Screen::Commit;
    app.commit.staged_count = 0;
    app.commit.message = "some message".to_string();

    let effect = app.handle_action(Action::Select);
    assert_eq!(effect, Effect::None, "No staged files should not allow commit");
}

#[test]
fn stage_screen_cannot_stage_with_no_selection() {
    let mut app = App::new();
    app.screen = Screen::Stage;
    // No entries selected

    let effect = app.handle_action(Action::Select);
    assert_eq!(effect, Effect::None, "No selection should not stage anything");
}

#[test]
fn help_page_bounds_never_overflow() {
    let mut app = App::new();
    app.screen = Screen::Help;

    // Hammer down 1000 times
    for _ in 0..1000 {
        app.handle_action(Action::MoveDown);
    }
    assert!(app.help.page < 100, "Page should be bounded");

    // Hammer up 1000 times
    for _ in 0..1000 {
        app.handle_action(Action::MoveUp);
    }
    assert_eq!(app.help.page, 0, "Page should clamp at 0");
}

#[test]
fn commit_message_length_bounded() {
    let mut app = App::new();
    app.screen = Screen::Commit;
    app.commit.staged_count = 1;

    // Type 500 characters
    for _ in 0..500 {
        app.handle_action(Action::Char('a'));
    }

    assert!(
        app.commit.message.len() <= 200,
        "Commit message should be bounded at 200 chars, got {}",
        app.commit.message.len()
    );
}
