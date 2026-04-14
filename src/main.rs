mod app;
mod editor;
mod git;
mod input;
mod mascot;
mod screens;
mod settings;
#[allow(dead_code)]
mod terminology;
mod ui;
mod update;

use std::io;
use std::time::Duration;

use crossterm::event::{self, Event, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use app::{App, CommitState, Effect, Screen, StageState, UpdateState};
use git::commit::create_commit;
use git::diff::get_file_diff;
use git::stage::stage_files;
use git::status::read_status;
use input::{map_key, map_key_text};
use settings::Settings;

const HELP_TEXT: &str = "\
GitCactus — a retro-inspired terminal Git assistant.

USAGE:
    gitcactus [FLAGS]

FLAGS:
    -h, --help         Print this help message and exit
    -V, --version      Print version information and exit
        --intro        Show the intro animation even if previously seen
        --skip-intro   Skip the intro animation on startup

ENVIRONMENT:
    EDITOR             Editor used by the \"Open in Editor\" action.
                       Falls back to nvim, vim, vi, nano, code, emacs.

SETTINGS:
    ~/.config/gitcactus/settings
                       Plain-text key=value file. Supported keys:
                         skip_intro=true
                         terminology=beginner|hybrid|git

Run gitcactus inside any git repository to explore and manage it
through a terminal UI. See https://github.com/fromloltocode/gitcactus
";

fn main() -> io::Result<()> {
    // Handle --help flag before entering TUI mode.
    if std::env::args().any(|a| a == "--help" || a == "-h") {
        print!("{HELP_TEXT}");
        return Ok(());
    }
    // Handle --version flag before entering TUI mode.
    if std::env::args().any(|a| a == "--version" || a == "-V") {
        println!("gitcactus {}", update::VERSION);
        return Ok(());
    }

    // --- terminal setup ---
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // --- run ---
    let result = run(&mut terminal);

    // --- terminal teardown (always runs) ---
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    let mut app = App::new();

    // Check settings — skip intro if already seen or --skip-intro flag.
    let user_settings = Settings::load();
    app.terms = terminology::Terms::new(user_settings.term_mode);

    let skip_intro = user_settings.skip_intro
        || std::env::args().any(|a| a == "--skip-intro");
    if skip_intro {
        app.screen = Screen::Title;
    }
    // --intro flag forces the intro even if settings say to skip.
    if std::env::args().any(|a| a == "--intro") {
        app.screen = Screen::Intro;
    }

    // Read repo status once on startup.
    let mut repo_status = read_status(".");

    loop {
        terminal.draw(|frame| ui::draw(frame, &app, &repo_status))?;

        // Use a shorter poll timeout during animated screens for smoothness.
        let poll_ms = if app.screen == Screen::Intro {
            120
        } else if app.screen == Screen::RebaseExecute
            && matches!(app.rebase_execute.mode, app::RebaseExecuteMode::Animating)
        {
            180
        } else {
            250
        };

        if event::poll(Duration::from_millis(poll_ms))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                // Any key press dismisses a lingering editor message.
                app.editor_msg = None;

                let action = if app.needs_text_input() {
                    map_key_text(key.code)
                } else {
                    map_key(key.code)
                };
                let effect = app.handle_action(action);

                // OpenEditor needs terminal access to suspend/restore the TUI,
                // so handle it inline before delegating to handle_effect.
                if let Effect::OpenEditor(path) = effect {
                    let result = suspend_and_open_editor(terminal, &path);
                    app.editor_msg = Some(editor::result_message(&result));
                } else {
                    handle_effect(&mut app, effect, &mut repo_status);
                }
                if app.should_quit {
                    break;
                }
            }
        } else if app.screen == Screen::Intro {
            // No key pressed during intro — advance the animation frame.
            let done = app.intro.tick();
            if done {
                Settings::mark_intro_seen();
                app.screen = Screen::Title;
            }
        } else if app.screen == Screen::RebaseExecute
            && matches!(app.rebase_execute.mode, app::RebaseExecuteMode::Animating)
        {
            // Advance the rebase playback animation one step per tick.
            let _ = app.rebase_execute.tick();
        }
    }

    Ok(())
}

/// Suspend the TUI, run the user's editor for `path`, then restore the TUI.
fn suspend_and_open_editor(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    path: &str,
) -> editor::EditorResult {
    // Tear down the alternate screen and raw mode so the editor has full
    // control of the terminal. Use `let _ =` so a failure here doesn't
    // prevent the editor attempt — we'll still report any error.
    let _ = disable_raw_mode();
    let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
    let _ = terminal.show_cursor();

    let result = editor::open_in_editor(path);

    // Restore the TUI. If any of these fail, the next draw call will
    // typically surface the error and exit cleanly via the normal path.
    let _ = enable_raw_mode();
    let _ = execute!(terminal.backend_mut(), EnterAlternateScreen);
    let _ = terminal.clear();
    let _ = terminal.hide_cursor();

    result
}

fn handle_effect(app: &mut App, effect: Effect, repo_status: &mut git::status::RepoStatus) {
    match effect {
        Effect::None => {}
        Effect::Quit => {
            // Handled by the loop — set should_quit for the next iteration.
            app.should_quit = true;
        }
        Effect::RefreshStatus => {
            *repo_status = read_status(".");
        }
        Effect::RefreshAndResetStage => {
            *repo_status = read_status(".");
            app.stage = StageState::from_repo(repo_status);
        }
        Effect::StageFiles(paths) => {
            match stage_files(".", &paths) {
                Ok(n) => {
                    app.stage.result_msg = Some((
                        format!("Staged {n} file{}.", if n == 1 { "" } else { "s" }),
                        true,
                    ));
                }
                Err(e) => {
                    app.stage.result_msg = Some((e, false));
                }
            }
            app.stage.mode = app::StageMode::Result;
        }
        Effect::InitUpdate => {
            app.update = UpdateState::new();
            app.update.info = Some(update::check_for_updates());
        }
        Effect::InitCommit => {
            *repo_status = read_status(".");
            app.commit = CommitState::from_repo(repo_status);
        }
        Effect::CreateCommit(msg) => {
            match create_commit(".", &msg) {
                Ok(hash) => {
                    app.commit.result_msg =
                        Some((format!("Commit {hash} created successfully!"), true));
                }
                Err(e) => {
                    app.commit.result_msg = Some((e, false));
                }
            }
            app.commit.mode = app::CommitMode::Result;
        }
        Effect::RefreshAfterCommit => {
            *repo_status = read_status(".");
            app.back_to_menu();
        }
        Effect::LoadDiff(path) => {
            app.diff.result = get_file_diff(".", &path);
            app.diff.scroll = 0;
        }
        Effect::IntroFinished => {
            Settings::mark_intro_seen();
            app.screen = Screen::Title;
        }
        Effect::SaveTermMode(mode) => {
            Settings::save_term_mode(mode);
        }
        Effect::LoadHistory => {
            app.history.result = git::history::load_history(".");
            app.history.cursor = 0;
            // Preserve filter across refresh, but clamp cursor in case
            // the new result set has fewer matches.
            app.history.clamp_cursor();
        }
        Effect::LoadBranches => {
            app.branches_state.branches = git::branches::load_branches(".");
            // Preserve filter across refresh, but clamp cursor in case
            // the new result set has fewer matches.
            app.branches_state.clamp_cursor();
        }
        Effect::LoadCommitDetails(hash) => {
            app.commit_details.details = git::commit_details::load_commit_details(".", &hash);
            app.commit_details.scroll = 0;
            app.commit_details.source_hash = hash;
        }
        Effect::LoadCommitDiff(hash) => {
            app.diff.result = git::diff::get_commit_diff(".", &hash);
            app.diff.scroll = 0;
        }
        Effect::CreateBranch(name) => {
            use git::branches::CreateResult;
            match git::branches::create_branch(".", &name) {
                CreateResult::Ok(created) => {
                    app.branches_state.result_msg =
                        Some((format!("Created '{created}'."), true));
                }
                CreateResult::InvalidName(msg) => {
                    app.branches_state.result_msg = Some((msg, false));
                }
                CreateResult::AlreadyExists => {
                    app.branches_state.result_msg = Some((
                        format!("A path named '{name}' already exists."),
                        false,
                    ));
                }
                CreateResult::Error(e) => {
                    app.branches_state.result_msg = Some((e, false));
                }
            }
            app.branches_state.new_name.clear();
            app.branches_state.mode = app::BranchesMode::Result;
        }
        Effect::SwitchBranch(name) => {
            use git::branches::SwitchResult;
            match git::branches::switch_branch(".", &name) {
                SwitchResult::Ok(branch) => {
                    app.branches_state.result_msg = Some((
                        format!("Switched to '{branch}'."),
                        true,
                    ));
                }
                SwitchResult::DirtyWorkingTree => {
                    app.branches_state.result_msg = Some((
                        "Can't switch — commit or stage changes first.".into(),
                        false,
                    ));
                }
                SwitchResult::Error(e) => {
                    app.branches_state.result_msg = Some((e, false));
                }
            }
            app.branches_state.mode = app::BranchesMode::Result;
        }
        Effect::OpenEditor(_) => {
            // Handled inline in the event loop so we have terminal access
            // to suspend/restore the TUI. This arm should be unreachable.
        }
        Effect::LoadRebasePreview(target) => {
            app.rebase_portal.preview =
                Some(git::rebase_preview::preview_rebase(".", &target));
            app.rebase_portal.scroll = 0;
            app.rebase_portal.mode = app::RebasePortalMode::Preview;
        }
        Effect::ExecuteRebase(target) => {
            // Preflight check first — never run rebase if safety fails.
            let preflight_result = git::rebase_execute::preflight(".", &target);
            let result = match preflight_result {
                Some(blocked) => blocked,
                None => git::rebase_execute::execute_rebase(".", &target),
            };

            // Seed the execute screen state from the preview we already have.
            // If preview is absent (shouldn't happen via normal UI flow),
            // fall back to an empty stub that still shows the outcome.
            let preview_snapshot = app
                .rebase_portal
                .preview
                .clone()
                .unwrap_or_else(|| git::rebase_preview::RebasePreview {
                    source: String::new(),
                    target: target.clone(),
                    source_tip: String::new(),
                    target_tip: String::new(),
                    merge_base: None,
                    commits: vec![],
                    truncated: false,
                    has_merge_commits: false,
                    dirty_tree: false,
                    kind: git::rebase_preview::PreviewKind::Ready,
                });
            app.rebase_execute =
                app::RebaseExecuteState::from_outcome(&preview_snapshot, &result);
            app.screen = app::Screen::RebaseExecute;
        }
        Effect::AbortRebase => {
            match git::rebase_execute::abort_rebase(".") {
                Ok(()) => {
                    app.rebase_execute.mode = app::RebaseExecuteMode::Aborted {
                        message: "Rebase aborted. Your branch is back where it was.".into(),
                        ok: true,
                    };
                }
                Err(e) => {
                    app.rebase_execute.mode = app::RebaseExecuteMode::Aborted {
                        message: format!("Abort failed: {e}"),
                        ok: false,
                    };
                }
            }
        }
        Effect::ContinueRebase => {
            use git::rebase_execute::ContinueResult;
            match git::rebase_execute::continue_rebase(".") {
                ContinueResult::Finished => {
                    // The rebase is fully done. Advance the animation
                    // playback to its last frame and show Success.
                    app.rebase_execute.fell = false;
                    let last = app
                        .rebase_execute
                        .commits
                        .len()
                        .saturating_sub(1);
                    app.rebase_execute.step = last;
                    app.rebase_execute.mode = app::RebaseExecuteMode::Success;
                }
                ContinueResult::AnotherConflict { stderr } => {
                    // Stay in a conflict state, but make it explicit that
                    // a *new* conflict followed the continue attempt.
                    app.rebase_execute.fell = true;
                    let annotated = if stderr.is_empty() {
                        "Another conflict after continue. Resolve and press 'c' again.".to_string()
                    } else {
                        format!(
                            "Another conflict after continue:\n{stderr}"
                        )
                    };
                    app.rebase_execute.mode =
                        app::RebaseExecuteMode::Conflict { stderr: annotated };
                }
                ContinueResult::Blocked { stderr } => {
                    // Git refused to continue. Keep the user in the
                    // conflict state so they can either resolve+stage and
                    // retry, or abort.
                    app.rebase_execute.fell = true;
                    let annotated = if stderr.is_empty() {
                        "Continue blocked. Resolve conflicts and `git add` them before retrying.".to_string()
                    } else {
                        format!(
                            "Continue blocked:\n{stderr}\n\nResolve conflicts and `git add` the files, then press 'c' again."
                        )
                    };
                    app.rebase_execute.mode =
                        app::RebaseExecuteMode::Conflict { stderr: annotated };
                }
                ContinueResult::NotInRebase => {
                    // Defensive — the screen thought we were in a rebase
                    // but the on-disk state says otherwise.
                    app.rebase_execute.mode = app::RebaseExecuteMode::Failure {
                        message: "No rebase is currently in progress.".into(),
                    };
                }
                ContinueResult::Error { message } => {
                    app.rebase_execute.mode = app::RebaseExecuteMode::Failure {
                        message: format!("Continue failed: {message}"),
                    };
                }
            }
        }
    }
}
