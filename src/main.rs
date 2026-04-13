mod app;
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

fn main() -> io::Result<()> {
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

        // Use a shorter poll timeout during intro for smooth animation.
        let poll_ms = if app.screen == Screen::Intro { 120 } else { 250 };

        if event::poll(Duration::from_millis(poll_ms))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                let action = if app.needs_text_input() {
                    map_key_text(key.code)
                } else {
                    map_key(key.code)
                };
                let effect = app.handle_action(action);
                handle_effect(&mut app, effect, &mut repo_status);
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
    }
}
