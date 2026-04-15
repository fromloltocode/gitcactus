mod app;
mod editor;
mod git;
mod input;
mod mascot;
mod mouse;
mod platform;
mod pricklings;
#[allow(dead_code)]
mod profile;
#[allow(dead_code)]
mod progression;
mod screens;
mod self_update;
mod settings;
#[allow(dead_code)]
mod terminology;
#[allow(dead_code)]
mod theme;
mod ui;
mod update;

use std::io;
use std::time::Duration;

use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind, MouseButton,
    MouseEventKind,
};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use app::{App, CommitState, Effect, PRICKLINGS_INDEX, Screen, StageState, UpdateState};
use git::commit::create_commit;
use git::diff::get_file_diff;
use git::stage::stage_files;
use git::status::read_status;
use input::{map_key, map_key_text};
use settings::Settings;

/// Build the `--help` text. Some paths and editor names differ between
/// Windows and Unix, so it's computed at call time from
/// [`platform`] helpers rather than a single `const`.
fn help_text() -> String {
    let settings_path = platform::config_dir_display("settings");
    let editors = platform::default_editor_candidates().join(", ");
    format!(
        "\
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
                       Falls back to: {editors}.

SETTINGS:
    {settings_path}
                       Plain-text key=value file. Supported keys:
                         skip_intro=true
                         terminology=beginner|hybrid|git
                         animations=on|off

Run gitcactus inside any git repository to explore and manage it
through a terminal UI. See https://github.com/fromloltocode/gitcactus
"
    )
}

fn main() -> io::Result<()> {
    // Handle --help flag before entering TUI mode.
    if std::env::args().any(|a| a == "--help" || a == "-h") {
        print!("{}", help_text());
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
    // `EnableMouseCapture` enables the terminal's mouse-tracking
    // escape sequence. Crossterm handles both the Unix X10/SGR
    // flavours and the Windows console API underneath. Terminals
    // that don't support mouse tracking will simply never emit
    // `Event::Mouse` — the app stays fully keyboard-driven.
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // --- run ---
    let result = run(&mut terminal);

    // --- terminal teardown (always runs) ---
    disable_raw_mode()?;
    // Disable mouse capture before leaving the alternate screen so
    // the user's normal shell gets its click-to-select behaviour back.
    execute!(terminal.backend_mut(), DisableMouseCapture, LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    let mut app = App::new();

    // Check settings — skip intro if already seen or --skip-intro flag.
    let user_settings = Settings::load();
    app.terms = terminology::Terms::new(user_settings.term_mode);
    app.theme = user_settings.theme;
    app.animations_enabled = user_settings.animations;

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
    // Route outside-repo launches through the Launchpad instead of
    // dumping the user into a main menu full of "not a git repo"
    // messages. `outside_repo` stays on App so the Title screen
    // picks the right post-splash destination and so `OpenPrickling`
    // can flip it off once a repo context is established.
    app.outside_repo = !repo_status.is_real;
    if app.outside_repo && app.screen == Screen::Title {
        // skip_intro path: land on the main menu with Pricklings
        // pre-selected so the most useful action is one Enter away.
        app.screen = Screen::Menu;
        app.menu_index = PRICKLINGS_INDEX;
    }

    loop {
        terminal.draw(|frame| ui::draw(frame, &app, &repo_status))?;

        // Use a shorter poll timeout during animated screens for smoothness.
        let poll_ms = if app.screen == Screen::Intro {
            120
        } else if app.screen == Screen::RebaseExecute
            && matches!(app.rebase_execute.mode, app::RebaseExecuteMode::Animating)
        {
            180
        } else if app
            .animation
            .as_ref()
            .is_some_and(|a| a.phase == mascot::animations::AnimationPhase::Playing)
        {
            // Overlay animations run at ~8 fps so the full sequence
            // lands in under 1 s before moving to the teaching panel.
            120
        } else {
            250
        };

        if event::poll(Duration::from_millis(poll_ms))? {
            match event::read()? {
                Event::Key(key) => {
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

                    // Some effects need terminal access to suspend/restore
                    // the TUI (editor, self-update). Dispatch them inline
                    // before falling through to the generic handler.
                    dispatch_effect(&mut app, effect, terminal, &mut repo_status);
                    if app.should_quit {
                        break;
                    }
                }
                Event::Mouse(m) => {
                    // Only react to the primary-button *press*. Everything
                    // else — drags, releases, middle/right buttons, scroll
                    // wheels — is ignored so the mouse surface is strictly
                    // "click to activate".
                    if !matches!(m.kind, MouseEventKind::Down(MouseButton::Left)) {
                        continue;
                    }

                    // Mouse clicks also dismiss the editor banner, same as keys.
                    app.editor_msg = None;

                    let area = terminal.size().map(|s| ratatui::layout::Rect {
                        x: 0,
                        y: 0,
                        width: s.width,
                        height: s.height,
                    });
                    if let Ok(area) = area {
                        let targets = ui::click_targets(area, &app);
                        if let Some(click_action) =
                            mouse::resolve_click(&targets, m.column, m.row)
                        {
                            let effect = app.handle_click_action(click_action);
                            dispatch_effect(&mut app, effect, terminal, &mut repo_status);
                            if app.should_quit {
                                break;
                            }
                        }
                    }
                }
                _ => {}
            }
        } else if app.screen == Screen::Intro {
            // No key pressed during intro — advance the animation frame.
            let done = app.intro.tick();
            if done {
                Settings::mark_intro_seen();
                if app.outside_repo {
                    app.screen = Screen::Menu;
                    app.menu_index = PRICKLINGS_INDEX;
                } else {
                    app.screen = Screen::Title;
                }
            }
        } else if app.screen == Screen::RebaseExecute
            && matches!(app.rebase_execute.mode, app::RebaseExecuteMode::Animating)
        {
            // Advance the rebase playback animation one step per tick.
            let _ = app.rebase_execute.tick();
        } else if let Some(anim) = app.animation.as_mut() {
            // Advance the overlay animation one frame per tick.
            // tick() flips to Teaching on the last frame and then
            // waits for a user key press (see handle_action).
            let _ = anim.tick();
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
    // Mouse capture has to be released while the editor runs —
    // leaving it on would eat the user's mouse clicks in the
    // editor's own UI. Re-enabled below after the editor exits.
    let _ = execute!(
        terminal.backend_mut(),
        DisableMouseCapture,
        LeaveAlternateScreen
    );
    let _ = terminal.show_cursor();

    let result = editor::open_in_editor(path);

    // Restore the TUI. If any of these fail, the next draw call will
    // typically surface the error and exit cleanly via the normal path.
    let _ = enable_raw_mode();
    let _ = execute!(
        terminal.backend_mut(),
        EnterAlternateScreen,
        EnableMouseCapture
    );
    let _ = terminal.clear();
    let _ = terminal.hide_cursor();

    result
}

/// Dispatch an `Effect`, routing the few variants that need direct
/// terminal access (TUI suspend/restore, streamed subprocess output)
/// before falling through to the generic `handle_effect` path.
fn dispatch_effect(
    app: &mut App,
    effect: Effect,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    repo_status: &mut git::status::RepoStatus,
) {
    match effect {
        Effect::OpenEditor(path) => {
            let result = suspend_and_open_editor(terminal, &path);
            app.editor_msg = Some(editor::result_message(&result));
        }
        Effect::RunUpdate => {
            run_self_update(app, terminal);
        }
        other => handle_effect(app, other, repo_status),
    }
}

/// Suspend the TUI, stream the self-update command's output to the
/// real terminal, then restore the TUI. Writes the result into
/// `app.update.outcome` and flips the screen into `Result` mode.
fn run_self_update(
    app: &mut App,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) {
    use std::process::Command;

    let plan = match app.update.plan.clone() {
        Some(p) => p,
        None => {
            // Defensive: should never be reached from the UI because
            // RunUpdate only fires in ConfirmUpdate mode where a plan
            // is required.
            app.update.outcome = Some(app::UpdateOutcome::Unsupported);
            app.update.mode = app::UpdateMode::Result;
            return;
        }
    };
    let cmd_spec = match &plan.command {
        Some(c) => c.clone(),
        None => {
            app.update.outcome = Some(app::UpdateOutcome::Unsupported);
            app.update.mode = app::UpdateMode::Result;
            return;
        }
    };

    // --- suspend TUI so the subprocess can take the terminal ---
    let _ = disable_raw_mode();
    let _ = execute!(
        terminal.backend_mut(),
        DisableMouseCapture,
        LeaveAlternateScreen
    );
    let _ = terminal.show_cursor();

    // Print a small header so the user sees what's happening.
    println!();
    println!("\u{2192} Running: {}", cmd_spec.render());
    println!();

    // Run inheriting stdio — streams straight to the user's terminal,
    // no buffering, no messy panel of captured text. If the command
    // fails to launch at all, that's the only case where we capture
    // its error for the Result screen.
    let status = Command::new(&cmd_spec.program)
        .args(&cmd_spec.args)
        .status();

    let outcome = match status {
        Ok(s) if s.success() => {
            println!();
            println!("\u{2713} Update finished successfully.");
            if plan.requires_relaunch {
                println!("  Relaunch gitcactus to use the new version.");
            }
            println!();
            app::UpdateOutcome::Success {
                message: if plan.requires_relaunch {
                    "Update finished. Relaunch gitcactus to use the new version.".into()
                } else {
                    "Update finished successfully.".into()
                },
            }
        }
        Ok(s) => {
            let code = s.code().unwrap_or(-1);
            println!();
            println!("\u{2717} Update command exited with status {code}.");
            println!();
            app::UpdateOutcome::Failure {
                message: format!(
                    "`{}` exited with status {code}.",
                    cmd_spec.render()
                ),
            }
        }
        Err(e) => {
            println!();
            println!("\u{2717} Could not launch updater: {e}");
            println!();
            app::UpdateOutcome::Failure {
                message: format!("Could not launch `{}`: {e}", cmd_spec.program),
            }
        }
    };

    // Tiny pause so the user can read the final line before we
    // snap the TUI back on top of their terminal scrollback.
    print!("(press Enter to return to GitCactus) ");
    use std::io::Write as _;
    let _ = io::stdout().flush();
    let mut _s = String::new();
    let _ = io::stdin().read_line(&mut _s);

    // --- restore TUI ---
    let _ = enable_raw_mode();
    let _ = execute!(
        terminal.backend_mut(),
        EnterAlternateScreen,
        EnableMouseCapture
    );
    let _ = terminal.clear();
    let _ = terminal.hide_cursor();

    app.update.outcome = Some(outcome);
    app.update.mode = app::UpdateMode::Result;
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
        Effect::PrepareUpdate => {
            // Detect the install kind and build a plan. If the plan
            // has no command (unsupported install kind), route the UI
            // straight to Result mode with an honest Unsupported
            // outcome so the screen shows the right instructions.
            let kind = self_update::detect_install();
            let plan = self_update::UpdatePlan::for_kind(kind);
            if plan.command.is_some() {
                app.update.plan = Some(plan);
                app.update.mode = app::UpdateMode::ConfirmUpdate;
            } else {
                app.update.plan = Some(plan);
                app.update.outcome = Some(app::UpdateOutcome::Unsupported);
                app.update.mode = app::UpdateMode::Result;
            }
        }
        Effect::RunUpdate => {
            // Handled inline in `dispatch_effect` so we have terminal
            // access for TUI suspend / restore. Reaching this arm
            // means the effect escaped the inline path — surface it
            // as a soft failure rather than silently dropping it.
            app.update.outcome = Some(app::UpdateOutcome::Failure {
                message: "Internal error: update runner did not dispatch.".into(),
            });
            app.update.mode = app::UpdateMode::Result;
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
            if app.outside_repo {
                app.screen = Screen::Menu;
                app.menu_index = PRICKLINGS_INDEX;
            } else {
                app.screen = Screen::Title;
            }
        }
        Effect::SaveTermMode(mode) => {
            Settings::save_term_mode(mode);
        }
        Effect::SaveThemePreset(preset) => {
            Settings::save_theme_preset(preset);
            // Reload so any `theme.*=` overrides the user may have in
            // their settings file fold back in on top of the new preset.
            // This keeps the optimistically-applied in-memory palette
            // (preset.palette()) in sync with what the next launch
            // would see.
            let fresh = Settings::load();
            app.theme = fresh.theme;
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
        Effect::LoadRemoteSync => {
            app.remote_sync.info = git::remote::load_remote_info(".");
            app.remote_sync.clamp_cursor();
        }
        Effect::LoadPricklings => {
            app.pricklings_hub.store = crate::pricklings::PricklingsStore::load();
            app.pricklings_hub.clamp_cursor();
        }
        Effect::ScanPricklings => {
            let result =
                crate::pricklings::discovery::scan_roots(&app.pricklings_hub.store.scan_roots);
            app.pricklings_results.results = result.found;
            app.pricklings_results.errors = result.errors;
            app.pricklings_results.cursor = 0;
            app.screen = Screen::PricklingsResults;
        }
        Effect::OpenPrickling(path) => {
            if !path.exists() {
                app.pricklings_hub.result_msg = Some((
                    format!("Path no longer exists: {}", path.display()),
                    false,
                ));
            } else if std::env::set_current_dir(&path).is_err() {
                app.pricklings_hub.result_msg = Some((
                    format!("Could not open: {}", path.display()),
                    false,
                ));
            } else {
                *repo_status = read_status(".");
                if repo_status.is_real {
                    app.stage = StageState::from_repo(repo_status);
                    app.screen = Screen::Menu;
                    app.pricklings_hub.result_msg = None;
                } else {
                    app.pricklings_hub.result_msg = Some((
                        format!("Not a git repository: {}", path.display()),
                        false,
                    ));
                }
            }
        }
        Effect::SavePrickling(p) => {
            let added = app.pricklings_hub.store.save_prickling(p);
            app.pricklings_hub.store.save();
            app.pricklings_results.result_msg = Some((
                if added {
                    "Saved to your pricklings.".into()
                } else {
                    "Already in your pricklings.".into()
                },
                added,
            ));
        }
        Effect::RemoveSavedPrickling(idx) => {
            app.pricklings_hub.store.remove_saved(idx);
            app.pricklings_hub.store.save();
            app.pricklings_hub.clamp_cursor();
        }
        Effect::AddScanRoot(path) => {
            let added = app.pricklings_hub.store.add_scan_root(&path);
            app.pricklings_hub.store.save();
            app.scan_locations.roots = app.pricklings_hub.store.scan_roots.clone();
            app.scan_locations.input_buffer.clear();
            app.scan_locations.result_msg = Some((
                if added {
                    format!("Added: {}", path.display())
                } else {
                    format!("Could not add: {}", path.display())
                },
                added,
            ));
            app.scan_locations.mode = app::ScanLocationsMode::Result;
        }
        Effect::RemoveScanRoot(idx) => {
            app.pricklings_hub.store.remove_scan_root(idx);
            app.pricklings_hub.store.save();
            app.scan_locations.roots = app.pricklings_hub.store.scan_roots.clone();
            app.scan_locations.clamp_cursor();
        }
        Effect::FetchFromRemote(remote_name) => {
            use git::remote::FetchResult;
            let result = git::remote::fetch(".", &remote_name);
            app.remote_sync.result_msg = Some(match result {
                FetchResult::Ok { remote } => (
                    format!("Fetched from '{remote}'. Remote view is up to date."),
                    true,
                ),
                FetchResult::NoRemote => (
                    "No remotes are configured in this repository.".into(),
                    false,
                ),
                FetchResult::NoSuchRemote(name) => (
                    format!("Remote '{name}' is not configured."),
                    false,
                ),
                FetchResult::AuthFailed(msg) => (msg, false),
                FetchResult::NetworkError(msg) => (msg, false),
                FetchResult::Error(msg) => (msg, false),
            });
            app.remote_sync.mode = app::RemoteSyncMode::Result;
        }
    }
}
