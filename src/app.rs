//! Central application state for GitCactus.
//!
//! All mutable state lives here. The UI layer reads from `App` to render,
//! and the event loop writes to it based on user input. State transitions
//! are driven by [`Action`]s (see [`App::handle_action`]).

use crate::input::Action;
use crate::terminology::Terms;

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
    /// Load a diff for the given file path (read-only).
    LoadDiff(String),
    /// The intro animation completed — mark it as seen and advance.
    IntroFinished,
    /// Persist the selected terminology mode to the settings file.
    SaveTermMode(crate::terminology::TermMode),
    /// Load commit history (read-only).
    LoadHistory,
    /// Load the branch list (read-only).
    LoadBranches,
    /// Switch to the given branch name (after confirmation).
    SwitchBranch(String),
}

/// The screens the app can display.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Intro,
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
    DiffPreview,
    Settings,
}

/// Main menu items, in display order.
pub const MENU_ITEMS: &[(&str, Screen)] = &[
    ("Status", Screen::Status),
    ("Stage Changes", Screen::Stage),
    ("Commit Changes", Screen::Commit),
    ("Branches", Screen::Branches),
    ("History", Screen::History),
    ("Remote Sync", Screen::RemoteSync),
    ("Controls", Screen::Help),
    ("Check for Updates", Screen::Update),
    ("Settings", Screen::Settings),
    ("Quit", Screen::Title), // sentinel — handled specially
];

/// Index of the "Quit" entry in MENU_ITEMS.
pub const QUIT_INDEX: usize = 9;

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

impl Default for UpdateState {
    fn default() -> Self { Self::new() }
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

impl Default for CommitState {
    fn default() -> Self { Self::new() }
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

// ── Help / Controls screen state ────────────────────────────────────

/// Total number of pages in the controls screen.
pub const HELP_PAGE_COUNT: usize = 4;

/// Page titles for the controls screen (fighting-game categories).
pub const HELP_PAGE_TITLES: &[&str] = &[
    "BASIC MOVES",
    "SPECIAL MOVES",
    "POWER MOVES",
    "DEFENSIVE MOVES",
];

/// State for the Help / Controls screen.
pub struct HelpState {
    /// Currently visible page (0-indexed).
    pub page: usize,
}

impl Default for HelpState {
    fn default() -> Self { Self::new() }
}

impl HelpState {
    pub fn new() -> Self {
        Self { page: 0 }
    }

    pub fn next_page(&mut self) {
        if self.page < HELP_PAGE_COUNT - 1 {
            self.page += 1;
        }
    }

    pub fn prev_page(&mut self) {
        if self.page > 0 {
            self.page -= 1;
        }
    }
}

// ── Intro animation state ───────────────────────────────────────────

/// Total number of intro frames (boot log + logo reveal + blink cycles).
pub const INTRO_TOTAL_FRAMES: usize = 20;

/// State for the retro intro animation.
pub struct IntroState {
    /// Current animation frame.
    pub frame: usize,
    /// Whether the animation has finished or been skipped.
    pub done: bool,
}

impl Default for IntroState {
    fn default() -> Self { Self::new() }
}

impl IntroState {
    pub fn new() -> Self {
        Self { frame: 0, done: false }
    }

    /// Advance to the next frame. Returns true if animation is now done.
    pub fn tick(&mut self) -> bool {
        if self.done {
            return true;
        }
        self.frame += 1;
        if self.frame >= INTRO_TOTAL_FRAMES {
            self.done = true;
        }
        self.done
    }

    /// Skip the animation immediately.
    pub fn skip(&mut self) {
        self.done = true;
    }
}

// ── Diff preview state ──────────────────────────────────────────────

/// State for the read-only diff preview screen.
pub struct DiffState {
    /// The diff result to display.
    pub result: crate::git::diff::DiffResult,
    /// Scroll offset within the diff lines.
    pub scroll: usize,
}

impl Default for DiffState {
    fn default() -> Self { Self::new() }
}

impl DiffState {
    pub fn new() -> Self {
        Self {
            result: crate::git::diff::DiffResult {
                file_path: String::new(),
                kind: crate::git::diff::DiffKind::Empty,
                lines: vec![],
                truncated: false,
            },
            scroll: 0,
        }
    }

    pub fn scroll_up(&mut self) {
        self.scroll = self.scroll.saturating_sub(3);
    }

    pub fn scroll_down(&mut self) {
        let max = self.result.lines.len().saturating_sub(1);
        self.scroll = (self.scroll + 3).min(max);
    }
}

// ── History screen state ────────────────────────────────────────────

/// State for the commit history screen.
pub struct HistoryState {
    /// Loaded history data.
    pub result: crate::git::history::HistoryResult,
    /// Currently highlighted entry index.
    pub cursor: usize,
}

impl Default for HistoryState {
    fn default() -> Self { Self::new() }
}

impl HistoryState {
    pub fn new() -> Self {
        Self {
            result: crate::git::history::HistoryResult {
                entries: vec![],
                is_real: false,
                error: None,
            },
            cursor: 0,
        }
    }

    pub fn move_up(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    pub fn move_down(&mut self) {
        let max = self.result.entries.len().saturating_sub(1);
        if self.cursor < max {
            self.cursor += 1;
        }
    }
}

// ── Branches screen state ───────────────────────────────────────────

/// Sub-mode for the branches screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BranchesMode {
    /// Browsing the branch list.
    Browse,
    /// Confirmation dialog to switch branches.
    ConfirmSwitch,
    /// Showing a result message.
    Result,
}

/// State for the Branches screen.
pub struct BranchesState {
    pub branches: crate::git::branches::BranchListResult,
    pub cursor: usize,
    pub mode: BranchesMode,
    pub result_msg: Option<(String, bool)>,
}

impl Default for BranchesState {
    fn default() -> Self { Self::new() }
}

impl BranchesState {
    pub fn new() -> Self {
        Self {
            branches: crate::git::branches::BranchListResult {
                branches: vec![],
                error: None,
            },
            cursor: 0,
            mode: BranchesMode::Browse,
            result_msg: None,
        }
    }

    pub fn move_up(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    pub fn move_down(&mut self) {
        let max = self.branches.branches.len().saturating_sub(1);
        if self.cursor < max {
            self.cursor += 1;
        }
    }

    /// Return the name of the currently highlighted branch, if any.
    pub fn selected_name(&self) -> Option<&str> {
        self.branches
            .branches
            .get(self.cursor)
            .map(|b| b.name.as_str())
    }

    /// Whether the highlighted branch is the current one.
    pub fn selected_is_current(&self) -> bool {
        self.branches
            .branches
            .get(self.cursor)
            .is_some_and(|b| b.is_current)
    }
}

// ── Settings screen state ───────────────────────────────────────────

/// The terminology modes available for selection, in display order.
pub const SETTINGS_TERM_MODES: &[crate::terminology::TermMode] = &[
    crate::terminology::TermMode::Beginner,
    crate::terminology::TermMode::Hybrid,
    crate::terminology::TermMode::Git,
];

/// State for the Settings screen.
pub struct SettingsState {
    /// Currently highlighted mode index.
    pub cursor: usize,
}

impl Default for SettingsState {
    fn default() -> Self { Self::new() }
}

impl SettingsState {
    pub fn new() -> Self {
        Self { cursor: 0 }
    }

    /// Initialize cursor to match the currently active mode.
    pub fn from_active(mode: crate::terminology::TermMode) -> Self {
        let cursor = SETTINGS_TERM_MODES
            .iter()
            .position(|&m| m == mode)
            .unwrap_or(1); // default to Hybrid (index 1)
        Self { cursor }
    }

    pub fn move_up(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.cursor < SETTINGS_TERM_MODES.len() - 1 {
            self.cursor += 1;
        }
    }

    /// Return the mode at the current cursor position.
    pub fn selected_mode(&self) -> crate::terminology::TermMode {
        SETTINGS_TERM_MODES[self.cursor]
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
    /// State for the Help / Controls screen.
    pub help: HelpState,
    /// State for the retro intro animation.
    pub intro: IntroState,
    /// State for the diff preview screen.
    pub diff: DiffState,
    /// Active terminology (Beginner / Hybrid / Git).
    pub terms: Terms,
    /// State for the Settings screen.
    pub settings_state: SettingsState,
    /// State for the History screen.
    pub history: HistoryState,
    /// State for the Branches screen.
    pub branches_state: BranchesState,
}

impl Default for App {
    fn default() -> Self { Self::new() }
}

impl App {
    pub fn new() -> Self {
        Self {
            screen: Screen::Intro,
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
            help: HelpState::new(),
            intro: IntroState::new(),
            diff: DiffState::new(),
            terms: Terms::new(crate::terminology::TermMode::Hybrid),
            settings_state: SettingsState::new(),
            history: HistoryState::new(),
            branches_state: BranchesState::new(),
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
            // ── Intro ────────────────────────────────────────────
            Screen::Intro => {
                // Any key skips the intro animation.
                self.intro.skip();
                Effect::IntroFinished
            }

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
                        Screen::Help => {
                            self.help = HelpState::new();
                            Effect::None
                        }
                        Screen::Settings => {
                            self.settings_state = SettingsState::from_active(self.terms.mode);
                            Effect::None
                        }
                        Screen::History => Effect::LoadHistory,
                        Screen::Branches => Effect::LoadBranches,
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
                    Action::Preview => {
                        if let Some(entry) = self.stage.entries.get(self.stage.cursor) {
                            let path = entry.path.clone();
                            self.screen = Screen::DiffPreview;
                            return Effect::LoadDiff(path);
                        }
                        Effect::None
                    }
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

            // ── Diff Preview (read-only) ─────────────────────────
            Screen::DiffPreview => match action {
                Action::Quit => Effect::Quit,
                Action::Back => {
                    self.screen = Screen::Stage;
                    Effect::None
                }
                Action::MoveUp => {
                    self.diff.scroll_up();
                    Effect::None
                }
                Action::MoveDown => {
                    self.diff.scroll_down();
                    Effect::None
                }
                _ => Effect::None,
            },

            // ── Help / Controls ───────────────────────────────────
            Screen::Help => match action {
                Action::Quit => Effect::Quit,
                Action::Back => {
                    self.back_to_menu();
                    Effect::None
                }
                Action::MoveDown | Action::Select => {
                    self.help.next_page();
                    Effect::None
                }
                Action::MoveUp => {
                    self.help.prev_page();
                    Effect::None
                }
                _ => Effect::None,
            },

            // ── Branches ──────────────────────────────────────────
            Screen::Branches => match self.branches_state.mode {
                BranchesMode::Browse => match action {
                    Action::Quit => Effect::Quit,
                    Action::Back => {
                        self.back_to_menu();
                        Effect::None
                    }
                    Action::MoveUp => {
                        self.branches_state.move_up();
                        Effect::None
                    }
                    Action::MoveDown => {
                        self.branches_state.move_down();
                        Effect::None
                    }
                    Action::Refresh => Effect::LoadBranches,
                    Action::Select => {
                        if self.branches_state.selected_is_current() {
                            // Already on this branch — friendly no-op message.
                            self.branches_state.result_msg = Some((
                                "You're already on this path!".into(),
                                true,
                            ));
                            self.branches_state.mode = BranchesMode::Result;
                            Effect::None
                        } else if let Some(name) = self.branches_state.selected_name() {
                            let _ = name; // used for the confirmation dialog
                            self.branches_state.mode = BranchesMode::ConfirmSwitch;
                            Effect::None
                        } else {
                            Effect::None
                        }
                    }
                    _ => Effect::None,
                },
                BranchesMode::ConfirmSwitch => match action {
                    Action::Confirm | Action::Select => {
                        if let Some(name) = self.branches_state.selected_name() {
                            let name = name.to_string();
                            Effect::SwitchBranch(name)
                        } else {
                            self.branches_state.mode = BranchesMode::Browse;
                            Effect::None
                        }
                    }
                    Action::Deny | Action::Back => {
                        self.branches_state.mode = BranchesMode::Browse;
                        Effect::None
                    }
                    _ => Effect::None,
                },
                BranchesMode::Result => {
                    // Any key: reload branches and go back to browse.
                    self.branches_state.mode = BranchesMode::Browse;
                    Effect::LoadBranches
                }
            },

            // ── History (read-only) ───────────────────────────────
            Screen::History => match action {
                Action::Quit => Effect::Quit,
                Action::Back => {
                    self.back_to_menu();
                    Effect::None
                }
                Action::MoveUp => {
                    self.history.move_up();
                    Effect::None
                }
                Action::MoveDown => {
                    self.history.move_down();
                    Effect::None
                }
                Action::Refresh => Effect::LoadHistory,
                _ => Effect::None,
            },

            // ── Settings ──────────────────────────────────────────
            Screen::Settings => match action {
                Action::Quit => Effect::Quit,
                Action::Back => {
                    self.back_to_menu();
                    Effect::None
                }
                Action::MoveUp => {
                    self.settings_state.move_up();
                    Effect::None
                }
                Action::MoveDown => {
                    self.settings_state.move_down();
                    Effect::None
                }
                Action::Select => {
                    let mode = self.settings_state.selected_mode();
                    self.terms = Terms::new(mode);
                    Effect::SaveTermMode(mode)
                }
                _ => Effect::None,
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
