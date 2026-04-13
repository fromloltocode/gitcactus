mod app;
mod git;
mod input;
mod mascot;
mod screens;
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

use app::{App, Effect, StageState, UpdateState};
use git::stage::stage_files;
use git::status::read_status;
use input::map_key;

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

    // Read repo status once on startup. Refreshed when entering the status
    // screen. This keeps the hot loop fast.
    let mut repo_status = read_status(".");

    loop {
        terminal.draw(|frame| ui::draw(frame, &app, &repo_status))?;

        // Poll for input with a small timeout so we can tick if needed later.
        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                // Only handle key-press events (ignore release/repeat on
                // platforms that send them).
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                let action = map_key(key.code);
                let effect = app.handle_action(action);

                match effect {
                    Effect::None => {}
                    Effect::Quit => break,
                    Effect::RefreshStatus => {
                        repo_status = read_status(".");
                    }
                    Effect::RefreshAndResetStage => {
                        repo_status = read_status(".");
                        app.stage = StageState::from_repo(&repo_status);
                    }
                    Effect::StageFiles(paths) => {
                        match stage_files(".", &paths) {
                            Ok(n) => {
                                app.stage.result_msg = Some((
                                    format!(
                                        "Staged {n} file{}.",
                                        if n == 1 { "" } else { "s" }
                                    ),
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
                }
            }
        }
    }

    Ok(())
}
