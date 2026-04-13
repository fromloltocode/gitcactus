//! Central application state for GitCactus.
//!
//! All mutable state lives here. The UI layer reads from `App` to render,
//! and the event loop writes to it based on user input. State transitions
//! are driven by [`Action`]s (see [`App::handle_action`]).

use crate::input::Action;

/// A side-effect requested by [`App::handle_action`].
///
/// The event loop inspects the returned effect and performs the
/// corresponding I/O (git reads, staging, etc.).
#[derive(Debug, PartialEq, Eq)]
pub enum Effect {
    /// Nothing to do.
    None,
    /// The app should exit.
    Quit,
    /// Re-read the repository status.
    RefreshStatus,
    /// Re-read the repository status *and* rebuild stage state from it.
    RefreshAndResetStage,
    /// Stage the given file paths, then show the result.
    StageFiles(Vec<String>),
    /// Initialize the update screen (check for updates).
    InitUpdate,
    /// Initialize the commit screen (refresh status for staged file list).
    InitCommit,
    /// Create a commit with the given message.
    CreateCommit(String),
    /// Refresh status after a successful commit.
    RefreshAfterCommit,
}

/// The screens the app can display.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Title,
    Menu,
    Status,
    Stage,
    Commit,
    Branches,
    History,
    RemoteSync,
    Help,
    Update,
}

/// Main menu items, in display order.
pub const MENU_ITEMS: &[(&str, Screen)] = &[
    ("Status", Screen::Status),
    ("Stage Changes", Screen::Stage),
    ("Commit Changes", Screen::Commit),
    ("Branches", Screen::Branches),
    ("History", Screen::History),
    ("Remote Sync", Screen::RemoteSync),
    ("Help", Screen::Help),
    ("Check for Updates", Screen::Update),
    ("Quit", Screen::Title), // sentinel — handled specially
];

/// Index of the "Quit" entry in MENU_ITEMS.
pub const QUIT_INDEX: usize = 8;

// ── Stage screen state ───────────────────────────────────────────────

/// The sub-mode the stage screen can be in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StageMode {
    /// Normal file browsing / selection.
    Browse,
    /// Confirmation dialog is visible.
    Confirm,
    /// An operation just completed; showing result message.
    Result,
}

/// Kind of file shown in the stage list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StageFileKind {
    Modified,
    Untracked,
}

/// A single entry in the stageable file list.
#[derive(Debug, Clone)]
pub struct StageEntry {
    pub path: String,
    pub kind: StageFileKind,
    pub selected: bool,
}

/// All state for the Stage Changes screen.
pub struct StageState {
    /// Stageable files (modified + untracked).
    pub entries: Vec<StageEntry>,
    /// Already-staged file paths (read-only display).
    pub already_staged: Vec<String>,
    /// Index of the highlighted entry.
    pub cursor: usize,
    /// Current sub-mode.
    pub mode: StageMode,
    /// Result message after staging (message, is_success).
    pub result_msg: Option<(String, bool)>,
}

impl StageState {
    /// Build stage state from a `RepoStatus`.
    pub fn from_repo(repo: &crate::git::status::RepoStatus) -> Self {
        let mut entries = Vec::new();
        for path in &repo.modified_files {
            entries.push(StageEntry {
                path: path.clone(),
                kind: StageFileKind::Modified,
                selected: false,
            });
        }
        for path in &repo.untracked_files {
            entries.push(StageEntry {
                path: path.clone(),
                kind: StageFileKind::Untracked,
                selected: false,
            });
        }
        Self {
            entries,
            already_staged: repo.staged_files.clone(),
            cursor: 0,
            mode: StageMode::Browse,
            result_msg: None,
        }
    }

    pub fn move_up(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if !self.entries.is_empty() && self.cursor < self.entries.len() - 1 {
            self.cursor += 1;
        }
    }

    pub fn toggle_current(&mut self) {
        if let Some(e) = self.entries.get_mut(self.cursor) {
            e.selected = !e.selected;
        }
    }

    pub fn toggle_all(&mut self) {
        let all_selected = self.entries.iter().all(|e| e.selected);
        for e in &mut self.entries {
            e.selected = !all_selected;
        }
    }

    pub fn selected_count(&self) -> usize {
        self.entries.iter().filter(|e| e.selected).count()
    }

    pub fn selected_paths(&self) -> Vec<String> {
        self.entries
            .iter()
            .filter(|e| e.selected)
            .map(|e| e.path.clone())
            .collect()
    }
}

// ── Update screen state ──────────────────────────────────────────────

/// Sub-mode for the update screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateMode {
    /// Main view with version info and action menu.
    Main,
    /// Viewing release notes.
    ReleaseNotes,
    /// Showing a "not yet implemented" result.
    Result,
}

/// Menu options on the update screen.
pub const UPDATE_ACTIONS: &[&str] = &[
    "Update Now",
    "View Release Notes",
    "Remind Later",
    "Back",
];

/// State for the Check for Updates screen.
pub struct UpdateState {
    pub cursor: usize,
    pub mode: UpdateMode,
    pub info: Option<crate::update::UpdateInfo>,
}

impl UpdateState {
    pub fn new() -> Self {
        Self {
            cursor: 0,
            mode: UpdateMode::Main,
            info: None,
        }
    }

    pub fn move_up(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.cursor < UPDATE_ACTIONS.len() - 1 {
            self.cursor += 1;
        }
    }
}

// ── Commit screen state ─────────────────────────────────────────────

/// Sub-mode for the commit screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitMode {
    /// Typing the commit message.
    Editing,
    /// Confirmation dialog visible.
    Confirm,
    /// Showing result (success/error).
    Result,
}

/// Maximum commit message length (single-line for now).
pub const MAX_COMMIT_MSG_LEN: usize = 200;

/// State for the Commit Changes screen.
pub struct CommitState {
    /// The commit message being composed.
    pub message: String,
    /// Files currently in the staging area (read from repo status).
    pub staged_files: Vec<String>,
    /// Total count of staged files.
    pub staged_count: usize,
    /// Current sub-mode.
    pub mode: CommitMode,
    /// Result message after committing (message, is_success).
    pub result_msg: Option<(String, bool)>,
}

impl CommitState {
    pub fn new() -> Self {
        Self {
            message: String::new(),
            staged_files: Vec::new(),
            staged_count: 0,
            mode: CommitMode::Editing,
            result_msg: None,
        }
    }

    /// Populate from a `RepoStatus`.
    pub fn from_repo(repo: &crate::git::status::RepoStatus) -> Self {
        Self {
            message: String::new(),
            staged_files: repo.staged_files.clone(),
            staged_count: repo.staged.unwrap_or(0),
            mode: CommitMode::Editing,
            result_msg: None,
        }
    }

    pub fn push_char(&mut self, c: char) {
        if self.message.len() < MAX_COMMIT_MSG_LEN {
            self.message.push(c);
        }
    }

    pub fn pop_char(&mut self) {
        self.message.pop();
    }

    pub fn can_commit(&self) -> bool {
        self.staged_count > 0 && !self.message.trim().is_empty()
    }
}

pub struct App {
    /// Which screen is currently shown.
    pub screen: Screen,
    /// Selected index in the main menu.
    pub menu_index: usize,
    /// Whether the app should exit on the next tick.
    pub should_quit: bool,
    /// State for the Stage Changes screen.
    pub stage: StageState,
    /// State for the Check for Updates screen.
    pub update: UpdateState,
    /// State for the Commit Changes screen.
    pub commit: CommitState,
}

impl App {
    pub fn new() -> Self {
        Self {
            screen: Screen::Title,
            menu_index: 0,
            should_quit: false,
            stage: StageState {
                entries: Vec::new(),
                already_staged: Vec::new(),
                cursor: 0,
                mode: StageMode::Browse,
                result_msg: None,
            },
            update: UpdateState::new(),
            commit: CommitState::new(),
        }
    }

    /// Whether the app is in a text-input mode that needs raw characters.
    ///
    /// When true, the event loop should use [`input::map_key_text`] instead
    /// of [`input::map_key`] so that typed characters reach the app as
    /// [`Action::Char`] rather than being mapped to navigation actions.
    pub fn needs_text_input(&self) -> bool {
        self.screen == Screen::Commit && self.commit.mode == CommitMode::Editing
    }

    /// Move menu selection up.
    pub fn menu_up(&mut self) {
        if self.menu_index > 0 {
            self.menu_index -= 1;
        }
    }

    /// Move menu selection down.
    pub fn menu_down(&mut self) {
        if self.menu_index < MENU_ITEMS.len() - 1 {
            self.menu_index += 1;
        }
    }

    /// Confirm the current menu selection.
    pub fn menu_select(&mut self) {
        if self.menu_index == QUIT_INDEX {
            self.should_quit = true;
            return;
        }
        self.screen = MENU_ITEMS[self.menu_index].1;
    }

    /// Go back to the main menu from any sub-screen.
    pub fn back_to_menu(&mut self) {
        self.screen = Screen::Menu;
    }

    /// Process a semantic [`Action`] and return an [`Effect`] for the
    /// event loop to execute.
    ///
    /// This is the single place where user intent meets application state.
    /// The event loop is responsible for carrying out any returned effect
    /// (I/O, git operations, etc.) and feeding the results back in.
    pub fn handle_action(&mut self, action: Action) -> Effect {
        match self.screen {
            // ── Title ────────────────────────────────────────────
            Screen::Title => {
                // Any key (including Quit/Back) advances to menu.
                self.screen = Screen::Menu;
                Effect::None
            }

            // ── Menu ─────────────────────────────────────────────
            Screen::Menu => match action {
                Action::Quit => Effect::Quit,
                Action::MoveUp => {
                    self.menu_up();
                    Effect::None
                }
                Action::MoveDown => {
                    self.menu_down();
                    Effect::None
                }
                Action::Select => {
                    self.menu_select();
                    if self.should_quit {
                        return Effect::Quit;
                    }
                    match self.screen {
                        Screen::Status => Effect::RefreshStatus,
                        Screen::Stage => Effect::RefreshAndResetStage,
                        Screen::Update => Effect::InitUpdate,
                        Screen::Commit => Effect::InitCommit,
                        _ => Effect::None,
                    }
                }
                _ => Effect::None,
            },

            // ── Stage ────────────────────────────────────────────
            Screen::Stage => match self.stage.mode {
                StageMode::Browse => match action {
                    Action::Quit => Effect::Quit,
                    Action::Back => {
                        self.back_to_menu();
                        Effect::None
                    }
                    Action::MoveUp => {
                        self.stage.move_up();
                        Effect::None
                    }
                    Action::MoveDown => {
                        self.stage.move_down();
                        Effect::None
                    }
                    Action::Toggle => {
                        self.stage.toggle_current();
                        Effect::None
                    }
                    Action::ToggleAll => {
                        self.stage.toggle_all();
                        Effect::None
                    }
                    Action::Refresh => Effect::RefreshAndResetStage,
                    Action::Select => {
                        if self.stage.selected_count() > 0 {
                            self.stage.mode = StageMode::Confirm;
                        }
                        Effect::None
                    }
                    _ => Effect::None,
                },
                StageMode::Confirm => match action {
                    Action::Confirm | Action::Select => {
                        let paths = self.stage.selected_paths();
                        Effect::StageFiles(paths)
                    }
                    Action::Deny | Action::Back => {
                        self.stage.mode = StageMode::Browse;
                        Effect::None
                    }
                    _ => Effect::None,
                },
                StageMode::Result => {
                    // Any key dismisses the result and refreshes.
                    Effect::RefreshAndResetStage
                }
            },

            // ── Commit ────────────────────────────────────────────
            Screen::Commit => match self.commit.mode {
                CommitMode::Editing => match action {
                    Action::Back => {
                        self.back_to_menu();
                        Effect::None
                    }
                    Action::Char(c) => {
                        self.commit.push_char(c);
                        Effect::None
                    }
                    Action::Backspace => {
                        self.commit.pop_char();
                        Effect::None
                    }
                    Action::Select => {
                        if self.commit.can_commit() {
                            self.commit.mode = CommitMode::Confirm;
                        }
                        Effect::None
                    }
                    _ => Effect::None,
                },
                CommitMode::Confirm => match action {
                    Action::Confirm | Action::Select => {
                        let msg = self.commit.message.trim().to_string();
                        Effect::CreateCommit(msg)
                    }
                    Action::Deny | Action::Back => {
                        self.commit.mode = CommitMode::Editing;
                        Effect::None
                    }
                    _ => Effect::None,
                },
                CommitMode::Result => {
                    // Any key: refresh and go back to menu.
                    Effect::RefreshAfterCommit
                }
            },

            // ── Update ────────────────────────────────────────────
            Screen::Update => match self.update.mode {
                UpdateMode::Main => match action {
                    Action::Quit => Effect::Quit,
                    Action::Back => {
                        self.back_to_menu();
                        Effect::None
                    }
                    Action::MoveUp => {
                        self.update.move_up();
                        Effect::None
                    }
                    Action::MoveDown => {
                        self.update.move_down();
                        Effect::None
                    }
                    Action::Select => {
                        match self.update.cursor {
                            0 => self.update.mode = UpdateMode::Result,
                            1 => self.update.mode = UpdateMode::ReleaseNotes,
                            2 | 3 => self.back_to_menu(),
                            _ => {}
                        }
                        Effect::None
                    }
                    _ => Effect::None,
                },
                UpdateMode::ReleaseNotes => {
                    // Any key dismisses the release notes overlay.
                    self.update.mode = UpdateMode::Main;
                    Effect::None
                }
                UpdateMode::Result => {
                    // Any key dismisses the result overlay.
                    self.update.mode = UpdateMode::Main;
                    Effect::None
                }
            },

            // ── All other sub-screens ────────────────────────────
            _ => match action {
                Action::Quit => Effect::Quit,
                Action::Back => {
                    self.back_to_menu();
                    Effect::None
                }
                _ => Effect::None,
            },
        }
    }
}
