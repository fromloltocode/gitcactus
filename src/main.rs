mod app;
mod editor;
mod git;
mod input;
mod mascot;
#[allow(dead_code)]
mod profile;
#[allow(dead_code)]
mod progression;
mod screens;
mod settings;
#[allow(dead_code)]
mod terminology;
#[allow(dead_code)]
mod theme;
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
    app.theme = user_settings.theme;

    // Load local progression profile. Missing / malformed files fall back
    // to an empty profile — progression is lightweight and local-only.
    app.profile = profile::Profile::load();

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

        // Use a shorter poll timeout during intro for smooth animation.
        let poll_ms = if app.screen == Screen::Intro { 120 } else { 250 };

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
        }
    }

    Ok(())
}

/// Apply one or more progression events, save the profile, and surface
/// a single, coalesced "+N XP" banner in `app.editor_msg`.
///
/// Why coalesce? A commit produces two events back-to-back
/// (CommitCreated + ComboChainCompleted) that should read as "+40 XP",
/// not two flashes. Staging N files is already one event, so
/// naturally one banner.
///
/// The banner piggy-backs on the existing editor-message system: it
/// appears at the bottom of the current screen and is dismissed by the
/// next key press. No modals, no input blocking, no stacking.
fn award_xp(app: &mut App, events: &[progression::ProgressionEvent]) {
    if events.is_empty() {
        return;
    }
    let xp_before = app.profile.xp;
    let level_before = app.profile.level;

    for &event in events {
        progression::apply_event(&mut app.profile, event);
    }
    app.profile.save();

    let gained = app.profile.xp.saturating_sub(xp_before);
    if gained == 0 {
        return;
    }

    let leveled_up = app.profile.level > level_before;
    let msg = if leveled_up {
        format!("\u{1F335} +{gained} XP \u{2014} LVL {}!", app.profile.level)
    } else {
        format!("\u{1F335} +{gained} XP")
    };
    // Overwrites any prior transient banner — "only show latest event".
    app.editor_msg = Some((msg, true));
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
                    // Progression: reward meaningful staging only on success.
                    award_xp(
                        app,
                        &[progression::ProgressionEvent::FilesStaged(n as u32)],
                    );
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
                    // Progression: commit earned. Also count a "combo" any
                    // time the user creates a commit — by reaching this
                    // point they have completed Status → Stage → Commit.
                    award_xp(
                        app,
                        &[
                            progression::ProgressionEvent::CommitCreated,
                            progression::ProgressionEvent::ComboChainCompleted,
                        ],
                    );
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
                    award_xp(
                        app,
                        &[progression::ProgressionEvent::BranchCreated],
                    );
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
                    award_xp(
                        app,
                        &[progression::ProgressionEvent::BranchSwitched],
                    );
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
    }
}
