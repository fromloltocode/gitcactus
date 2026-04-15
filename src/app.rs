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
    /// Persist the selected theme preset to the settings file and
    /// reload the full theme so any existing `theme.*=` overrides
    /// still apply on top.
    SaveThemePreset(crate::theme::ThemePreset),
    /// Load commit history (read-only).
    LoadHistory,
    /// Load the branch list (read-only).
    LoadBranches,
    /// Switch to the given branch name (after confirmation).
    SwitchBranch(String),
    /// Create a new branch from HEAD with the given name.
    CreateBranch(String),
    /// Load full details for the given commit hash (read-only).
    LoadCommitDetails(String),
    /// Load a diff for the given commit vs its parent (read-only).
    LoadCommitDiff(String),
    /// Open the given file path in the user's $EDITOR.
    /// The event loop is responsible for suspending the TUI before this.
    OpenEditor(String),
    /// Compute a read-only rebase preview for the given target branch.
    LoadRebasePreview(String),
    /// Execute `git rebase <target>`. Mutates the repository.
    /// The event loop performs preflight checks and runs the rebase synchronously.
    ExecuteRebase(String),
    /// Run `git rebase --abort` from the conflict state.
    AbortRebase,
    /// Run `git rebase --continue` from the conflict state.
    /// Guarded at the git layer: refuses to run unless a rebase is
    /// actually in progress on disk.
    ContinueRebase,
    /// Load a fresh snapshot of remote state for the Remote Sync screen.
    /// Read-only: no network activity.
    LoadRemoteSync,
    /// Fetch from the named remote. Writes to `refs/remotes/<remote>/*`
    /// only — never touches the working tree, index, HEAD, or local
    /// branches.
    FetchFromRemote(String),
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
    CommitDetails,
    /// Local progression / skill tree screen.
    SkillTree,
    /// Preview / confirm step for a rebase.
    RebasePortal,
    /// Actually running rebase + animation + result.
    RebaseExecute,
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
    ("Skill Tree", Screen::SkillTree),
    ("Quit", Screen::Title), // sentinel — handled specially
];

/// Index of the "Quit" entry in MENU_ITEMS.
pub const QUIT_INDEX: usize = 10;

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
    /// Which screen to return to on Esc.
    pub return_to: Screen,
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
            return_to: Screen::Stage,
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
    /// Currently highlighted entry index *in the visible (filtered) list*.
    pub cursor: usize,
    /// Current filter text. Empty means no filter.
    pub filter: String,
    /// Whether the user is actively typing in the filter input.
    pub search_mode: bool,
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
            filter: String::new(),
            search_mode: false,
        }
    }

    /// Return the indices (in `result.entries`) of entries that match
    /// the current filter. Empty filter returns every index.
    ///
    /// Matching is case-insensitive against summary, author, and short hash.
    pub fn visible_indices(&self) -> Vec<usize> {
        if self.filter.is_empty() {
            return (0..self.result.entries.len()).collect();
        }
        let needle = self.filter.to_lowercase();
        self.result
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                e.summary.to_lowercase().contains(&needle)
                    || e.author.to_lowercase().contains(&needle)
                    || e.short_hash.to_lowercase().contains(&needle)
            })
            .map(|(i, _)| i)
            .collect()
    }

    /// Entry currently under the cursor, honoring the filter.
    pub fn selected_entry(&self) -> Option<&crate::git::history::HistoryEntry> {
        let visible = self.visible_indices();
        visible
            .get(self.cursor)
            .and_then(|&i| self.result.entries.get(i))
    }

    /// Clamp the cursor into the currently visible range.
    pub fn clamp_cursor(&mut self) {
        let visible_count = self.visible_indices().len();
        let max = visible_count.saturating_sub(1);
        if self.cursor > max {
            self.cursor = max;
        }
    }

    pub fn move_up(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    pub fn move_down(&mut self) {
        let max = self.visible_indices().len().saturating_sub(1);
        if self.cursor < max {
            self.cursor += 1;
        }
    }

    pub fn enter_search(&mut self) {
        self.search_mode = true;
    }

    pub fn exit_search(&mut self) {
        self.search_mode = false;
    }

    pub fn push_filter_char(&mut self, c: char) {
        self.filter.push(c);
        self.cursor = 0;
    }

    pub fn pop_filter_char(&mut self) {
        self.filter.pop();
        self.cursor = 0;
    }

    pub fn clear_filter(&mut self) {
        self.filter.clear();
        self.search_mode = false;
        self.cursor = 0;
    }
}

// ── Commit details screen state ─────────────────────────────────────

/// State for the commit details screen.
pub struct CommitDetailsState {
    pub details: crate::git::commit_details::CommitDetails,
    /// Scroll position (for long messages + file lists).
    pub scroll: usize,
    /// Hash we came from — used when returning from diff screen.
    pub source_hash: String,
}

impl Default for CommitDetailsState {
    fn default() -> Self { Self::new() }
}

impl CommitDetailsState {
    pub fn new() -> Self {
        Self {
            details: crate::git::commit_details::CommitDetails::empty(),
            scroll: 0,
            source_hash: String::new(),
        }
    }

    pub fn scroll_up(&mut self) {
        self.scroll = self.scroll.saturating_sub(2);
    }

    pub fn scroll_down(&mut self) {
        // Clamp against (message lines + file count) — simple upper bound.
        let msg_lines = self.details.message.lines().count();
        let max = msg_lines + self.details.files.len() + 8;
        self.scroll = (self.scroll + 2).min(max.saturating_sub(1));
    }
}

// ── Rebase Portal state (preview + confirm) ─────────────────────────

/// Sub-mode for the rebase portal screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RebasePortalMode {
    /// Showing the preview and (if executable) a prompt to continue.
    Preview,
    /// Final confirmation before execution.
    Confirm,
}

/// State for the read-only rebase portal (preview + confirm).
pub struct RebasePortalState {
    pub preview: Option<crate::git::rebase_preview::RebasePreview>,
    pub mode: RebasePortalMode,
    pub scroll: usize,
    /// Screen to return to on Esc when in preview mode.
    pub return_to: Screen,
}

impl Default for RebasePortalState {
    fn default() -> Self { Self::new() }
}

impl RebasePortalState {
    pub fn new() -> Self {
        Self {
            preview: None,
            mode: RebasePortalMode::Preview,
            scroll: 0,
            return_to: Screen::Branches,
        }
    }

    pub fn scroll_up(&mut self) {
        self.scroll = self.scroll.saturating_sub(2);
    }

    pub fn scroll_down(&mut self) {
        let max = self
            .preview
            .as_ref()
            .map(|p| p.commits.len())
            .unwrap_or(0)
            .saturating_sub(1);
        self.scroll = (self.scroll + 2).min(max);
    }

    pub fn can_execute(&self) -> bool {
        self.preview
            .as_ref()
            .map(|p| p.is_executable())
            .unwrap_or(false)
    }
}

// ── Rebase Execute state (running + animation + result) ─────────────

/// Sub-mode for the rebase execute screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RebaseExecuteMode {
    /// Cactus is hopping through portals — a playback of what just happened.
    Animating,
    /// Animation complete and rebase succeeded.
    Success,
    /// Rebase stopped at a conflict. Abort is offered.
    Conflict { stderr: String },
    /// Some other non-conflict failure.
    Failure { message: String },
    /// Abort in progress / done — (message, was_success).
    Aborted { message: String, ok: bool },
}

/// State for the rebase execution + animation screen.
pub struct RebaseExecuteState {
    pub source: String,
    pub target: String,
    /// Ordered list of commit summaries that were (or would have been) replayed.
    pub commits: Vec<String>,
    /// Current animation frame (0-based step).
    pub step: usize,
    /// Whether the cactus should be shown as fallen (conflict animation).
    pub fell: bool,
    pub mode: RebaseExecuteMode,
}

impl Default for RebaseExecuteState {
    fn default() -> Self { Self::new() }
}

impl RebaseExecuteState {
    pub fn new() -> Self {
        Self {
            source: String::new(),
            target: String::new(),
            commits: Vec::new(),
            step: 0,
            fell: false,
            mode: RebaseExecuteMode::Success,
        }
    }

    /// Seed state from a preview + execute outcome.
    pub fn from_outcome(
        preview: &crate::git::rebase_preview::RebasePreview,
        result: &crate::git::rebase_execute::ExecuteResult,
    ) -> Self {
        let commits: Vec<String> = preview
            .commits
            .iter()
            .map(|c| format!("{} {}", c.short_hash, c.summary))
            .collect();
        let n = commits.len();

        use crate::git::rebase_execute::ExecuteResult as R;
        let (mode, fell) = match result {
            R::Success => (RebaseExecuteMode::Success, false),
            R::Conflict { stderr } => (
                RebaseExecuteMode::Conflict { stderr: stderr.clone() },
                true,
            ),
            R::Failure { message } => (
                RebaseExecuteMode::Failure { message: message.clone() },
                true,
            ),
            R::Blocked { reason } => (
                RebaseExecuteMode::Failure { message: reason.clone() },
                false,
            ),
        };

        Self {
            source: preview.source.clone(),
            target: preview.target.clone(),
            commits,
            step: 0,
            fell,
            // Play the success animation first; other modes show immediately.
            mode: if matches!(mode, RebaseExecuteMode::Success) && n > 0 {
                RebaseExecuteMode::Animating
            } else {
                mode
            },
        }
    }

    /// Advance one animation step. Returns true when animation is done.
    pub fn tick(&mut self) -> bool {
        if !matches!(self.mode, RebaseExecuteMode::Animating) {
            return true;
        }
        if self.step + 1 >= self.commits.len() {
            self.mode = RebaseExecuteMode::Success;
            return true;
        }
        self.step += 1;
        false
    }

    /// Skip the animation by jumping to the final frame.
    pub fn skip_animation(&mut self) {
        if matches!(self.mode, RebaseExecuteMode::Animating) {
            self.step = self.commits.len().saturating_sub(1);
            self.mode = RebaseExecuteMode::Success;
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
    /// Typing the name for a new branch ("New Game").
    Creating,
    /// Showing a result message.
    Result,
}

/// Maximum branch-name length.
pub const MAX_BRANCH_NAME_LEN: usize = 100;

/// State for the Branches screen.
pub struct BranchesState {
    pub branches: crate::git::branches::BranchListResult,
    /// Currently highlighted entry index *in the visible (filtered) list*.
    pub cursor: usize,
    pub mode: BranchesMode,
    pub result_msg: Option<(String, bool)>,
    /// Buffer for the new-branch name being typed.
    pub new_name: String,
    /// Current filter text. Empty means no filter.
    pub filter: String,
    /// Whether the user is actively typing in the filter input.
    pub search_mode: bool,
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
            new_name: String::new(),
            filter: String::new(),
            search_mode: false,
        }
    }

    pub fn push_name_char(&mut self, c: char) {
        if self.new_name.len() < MAX_BRANCH_NAME_LEN {
            self.new_name.push(c);
        }
    }

    pub fn pop_name_char(&mut self) {
        self.new_name.pop();
    }

    /// Return the indices (in `branches.branches`) of entries that match
    /// the current filter. Empty filter returns every index.
    ///
    /// Matching is case-insensitive against branch name and commit summary.
    pub fn visible_indices(&self) -> Vec<usize> {
        if self.filter.is_empty() {
            return (0..self.branches.branches.len()).collect();
        }
        let needle = self.filter.to_lowercase();
        self.branches
            .branches
            .iter()
            .enumerate()
            .filter(|(_, b)| {
                b.name.to_lowercase().contains(&needle)
                    || b.summary.to_lowercase().contains(&needle)
            })
            .map(|(i, _)| i)
            .collect()
    }

    pub fn move_up(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    pub fn move_down(&mut self) {
        let max = self.visible_indices().len().saturating_sub(1);
        if self.cursor < max {
            self.cursor += 1;
        }
    }

    /// Return the name of the currently highlighted branch, honoring filter.
    pub fn selected_name(&self) -> Option<String> {
        let visible = self.visible_indices();
        visible
            .get(self.cursor)
            .and_then(|&i| self.branches.branches.get(i))
            .map(|b| b.name.clone())
    }

    /// Whether the highlighted branch is the current one.
    pub fn selected_is_current(&self) -> bool {
        let visible = self.visible_indices();
        visible
            .get(self.cursor)
            .and_then(|&i| self.branches.branches.get(i))
            .is_some_and(|b| b.is_current)
    }

    pub fn clamp_cursor(&mut self) {
        let max = self.visible_indices().len().saturating_sub(1);
        if self.cursor > max {
            self.cursor = max;
        }
    }

    pub fn enter_search(&mut self) {
        self.search_mode = true;
    }

    pub fn exit_search(&mut self) {
        self.search_mode = false;
    }

    pub fn push_filter_char(&mut self, c: char) {
        self.filter.push(c);
        self.cursor = 0;
    }

    pub fn pop_filter_char(&mut self) {
        self.filter.pop();
        self.cursor = 0;
    }

    pub fn clear_filter(&mut self) {
        self.filter.clear();
        self.search_mode = false;
        self.cursor = 0;
    }
}

// ── Settings screen state ───────────────────────────────────────────

/// The terminology modes available for selection, in display order.
pub const SETTINGS_TERM_MODES: &[crate::terminology::TermMode] = &[
    crate::terminology::TermMode::Beginner,
    crate::terminology::TermMode::Hybrid,
    crate::terminology::TermMode::Git,
];

/// The theme presets available for selection, in display order.
pub const SETTINGS_THEME_PRESETS: &[crate::theme::ThemePreset] = &[
    crate::theme::ThemePreset::Default,
    crate::theme::ThemePreset::TerminalBlue,
    crate::theme::ThemePreset::Matrix,
    crate::theme::ThemePreset::RetroDanger,
];

/// What kind of setting the cursor is currently pointing at.
///
/// A flat cursor walks the concatenation of terminology modes then
/// theme presets; [`SettingsState::selection`] translates the cursor
/// position into this enum so the handler doesn't need to know the
/// underlying index math.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsSelection {
    TermMode(crate::terminology::TermMode),
    Theme(crate::theme::ThemePreset),
}

/// State for the Settings screen.
pub struct SettingsState {
    /// Unified cursor across all setting rows (terminology then theme).
    pub cursor: usize,
}

impl Default for SettingsState {
    fn default() -> Self {
        Self::new()
    }
}

impl SettingsState {
    /// Total number of rows the cursor can occupy.
    pub const fn total_rows() -> usize {
        SETTINGS_TERM_MODES.len() + SETTINGS_THEME_PRESETS.len()
    }

    pub fn new() -> Self {
        Self { cursor: 0 }
    }

    /// Initialize cursor to match the currently active terminology mode.
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
        if self.cursor + 1 < Self::total_rows() {
            self.cursor += 1;
        }
    }

    /// Return what the cursor is currently pointing at.
    pub fn selection(&self) -> SettingsSelection {
        let term_count = SETTINGS_TERM_MODES.len();
        if self.cursor < term_count {
            SettingsSelection::TermMode(SETTINGS_TERM_MODES[self.cursor])
        } else {
            let theme_idx = (self.cursor - term_count).min(SETTINGS_THEME_PRESETS.len() - 1);
            SettingsSelection::Theme(SETTINGS_THEME_PRESETS[theme_idx])
        }
    }
}

// ── Remote Sync screen state ────────────────────────────────────────

/// Sub-mode for the Remote Sync screen.
///
/// The screen is visibility-first: Browse is read-only. Confirming a
/// fetch is the *only* way to trigger network activity, and the Result
/// mode reports the outcome. Push/Pull are deliberately not modelled
/// yet — they will get their own modes when implemented.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteSyncMode {
    /// Read-only view of remotes + tracking info.
    Browse,
    /// Confirmation dialog before running a fetch.
    ConfirmFetch,
    /// Showing the result of the last fetch attempt.
    Result,
}

/// State for the Remote Sync screen.
pub struct RemoteSyncState {
    /// Last-loaded snapshot of remote state.
    pub info: crate::git::remote::RemoteInfo,
    /// Currently highlighted remote index (into `info.remotes`).
    pub cursor: usize,
    pub mode: RemoteSyncMode,
    /// Result message from the last fetch (message, is_success).
    pub result_msg: Option<(String, bool)>,
}

impl Default for RemoteSyncState {
    fn default() -> Self {
        Self::new()
    }
}

impl RemoteSyncState {
    pub fn new() -> Self {
        Self {
            info: crate::git::remote::RemoteInfo {
                remotes: vec![],
                current_branch: None,
                tracking: None,
                is_real: false,
                error: None,
            },
            cursor: 0,
            mode: RemoteSyncMode::Browse,
            result_msg: None,
        }
    }

    pub fn move_up(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if !self.info.remotes.is_empty() && self.cursor + 1 < self.info.remotes.len() {
            self.cursor += 1;
        }
    }

    /// Keep the cursor in-bounds when the remote list shrinks.
    pub fn clamp_cursor(&mut self) {
        let n = self.info.remotes.len();
        if n == 0 {
            self.cursor = 0;
        } else if self.cursor >= n {
            self.cursor = n - 1;
        }
    }

    /// The remote the cursor currently points at, if any.
    pub fn selected_remote(&self) -> Option<&crate::git::remote::RemoteEntry> {
        self.info.remotes.get(self.cursor)
    }

    /// Preferred remote name for a fetch action.
    ///
    /// - If the current branch has an upstream, use that remote.
    /// - Otherwise use the highlighted remote.
    /// - Otherwise `None` (no remotes configured).
    pub fn fetch_target(&self) -> Option<String> {
        if let Some(t) = &self.info.tracking {
            return Some(t.remote_name.clone());
        }
        self.selected_remote().map(|r| r.name.clone())
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
    /// State for the Commit Details screen.
    pub commit_details: CommitDetailsState,
    /// Transient message from the last editor-open attempt.
    /// Rendered as a status banner until the next key press clears it.
    pub editor_msg: Option<(String, bool)>,
    /// Active color theme (preset + optional overrides).
    pub theme: crate::theme::Theme,
    /// Local progression profile (XP, level, stats, unlocks).
    pub profile: crate::profile::Profile,
    /// State for the Rebase Portal preview + confirm screen.
    pub rebase_portal: RebasePortalState,
    /// State for the Rebase execute / animation screen.
    pub rebase_execute: RebaseExecuteState,
    /// State for the Remote Sync screen.
    pub remote_sync: RemoteSyncState,
    /// Active overlay animation, if any. `None` = normal operation.
    ///
    /// Set via `mascot::animations::play_*` after a successful Clone,
    /// Pull, or Push. Advanced by the main loop on idle ticks; cleared
    /// when the user dismisses the teaching panel.
    pub animation: Option<crate::mascot::animations::AnimationState>,
    /// Whether overlay animations are enabled (from Settings).
    pub animations_enabled: bool,
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
            commit_details: CommitDetailsState::new(),
            editor_msg: None,
            theme: crate::theme::Theme::default(),
            profile: crate::profile::Profile::default(),
            rebase_portal: RebasePortalState::new(),
            rebase_execute: RebaseExecuteState::new(),
            remote_sync: RemoteSyncState::new(),
            animation: None,
            animations_enabled: true,
        }
    }

    /// Whether the app is in a text-input mode that needs raw characters.
    ///
    /// When true, the event loop should use [`input::map_key_text`] instead
    /// of [`input::map_key`] so that typed characters reach the app as
    /// [`Action::Char`] rather than being mapped to navigation actions.
    pub fn needs_text_input(&self) -> bool {
        (self.screen == Screen::Commit && self.commit.mode == CommitMode::Editing)
            || (self.screen == Screen::Branches
                && self.branches_state.mode == BranchesMode::Creating)
            || (self.screen == Screen::History && self.history.search_mode)
            || (self.screen == Screen::Branches && self.branches_state.search_mode)
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
    ///
    /// Translate a mouse [`ClickAction`](crate::mouse::ClickAction)
    /// into the equivalent keyboard flow.
    ///
    /// For list-row clicks we set the corresponding screen's cursor
    /// first (clamped to the current list length) and then dispatch
    /// the same [`Action`] a keyboard user would press. For plain
    /// footer / dialog button clicks we dispatch the action directly.
    /// Either way, the resulting [`Effect`] flows through the same
    /// handler as a keyboard event — clicks never bypass confirm
    /// dialogs, never invent new semantics, and never touch Git
    /// state outside of what `handle_action` already allows.
    pub fn handle_click_action(&mut self, ca: crate::mouse::ClickAction) -> Effect {
        use crate::mouse::ClickAction as CA;
        match ca {
            CA::Fire(a) => self.handle_action(a),
            CA::SelectMenu(i) => {
                let max = MENU_ITEMS.len().saturating_sub(1);
                self.menu_index = i.min(max);
                self.handle_action(Action::Select)
            }
            CA::SelectSettings(i) => {
                let max = SettingsState::total_rows().saturating_sub(1);
                self.settings_state.cursor = i.min(max);
                self.handle_action(Action::Select)
            }
            CA::ToggleStage(i) => {
                if i >= self.stage.entries.len() {
                    return Effect::None;
                }
                self.stage.cursor = i;
                self.handle_action(Action::Toggle)
            }
        }
    }

    pub fn handle_action(&mut self, action: Action) -> Effect {
        // When an overlay animation is active, any key press is the
        // skip / dismiss gesture. We intercept here so animations are
        // universally skippable regardless of which screen triggered
        // them, and no per-screen handler needs to know about it.
        if let Some(anim) = self.animation.as_mut() {
            use crate::mascot::animations::AnimationPhase;
            let _ = action; // any key
            match anim.phase {
                AnimationPhase::Playing => anim.skip_to_teaching(),
                AnimationPhase::Teaching => self.animation = None,
            }
            return Effect::None;
        }

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
                        Screen::RemoteSync => Effect::LoadRemoteSync,
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
                            self.diff.return_to = Screen::Stage;
                            self.screen = Screen::DiffPreview;
                            return Effect::LoadDiff(path);
                        }
                        Effect::None
                    }
                    Action::Open => {
                        if let Some(entry) = self.stage.entries.get(self.stage.cursor) {
                            return Effect::OpenEditor(entry.path.clone());
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
                    self.screen = self.diff.return_to;
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
                BranchesMode::Browse => {
                    // Search mode: typing goes into the filter buffer.
                    if self.branches_state.search_mode {
                        return match action {
                            Action::Back => {
                                // Esc cancels search and clears the filter.
                                self.branches_state.clear_filter();
                                Effect::None
                            }
                            Action::Select => {
                                // Enter applies the filter and exits search mode.
                                self.branches_state.exit_search();
                                Effect::None
                            }
                            Action::Char(c) => {
                                self.branches_state.push_filter_char(c);
                                Effect::None
                            }
                            Action::Backspace => {
                                self.branches_state.pop_filter_char();
                                Effect::None
                            }
                            _ => Effect::None,
                        };
                    }

                    match action {
                        Action::Quit => Effect::Quit,
                        Action::Back => {
                            // If a filter is active, clear it first.
                            if !self.branches_state.filter.is_empty() {
                                self.branches_state.clear_filter();
                                Effect::None
                            } else {
                                self.back_to_menu();
                                Effect::None
                            }
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
                        Action::Search => {
                            self.branches_state.enter_search();
                            Effect::None
                        }
                        Action::Deny => {
                            // 'n' starts a "New Game" (create branch).
                            self.branches_state.new_name.clear();
                            self.branches_state.mode = BranchesMode::Creating;
                            Effect::None
                        }
                        Action::Portal => {
                            // 'p' opens the rebase portal preview for the
                            // highlighted (non-current) branch.
                            if self.branches_state.selected_is_current() {
                                Effect::None
                            } else if let Some(target) =
                                self.branches_state.selected_name()
                            {
                                self.rebase_portal = RebasePortalState::new();
                                self.rebase_portal.return_to = Screen::Branches;
                                self.screen = Screen::RebasePortal;
                                Effect::LoadRebasePreview(target)
                            } else {
                                Effect::None
                            }
                        }
                        Action::Select => {
                            if self.branches_state.selected_is_current() {
                                self.branches_state.result_msg = Some((
                                    "You're already on this path!".into(),
                                    true,
                                ));
                                self.branches_state.mode = BranchesMode::Result;
                                Effect::None
                            } else if self.branches_state.selected_name().is_some() {
                                self.branches_state.mode = BranchesMode::ConfirmSwitch;
                                Effect::None
                            } else {
                                Effect::None
                            }
                        }
                        _ => Effect::None,
                    }
                }
                BranchesMode::ConfirmSwitch => match action {
                    Action::Confirm | Action::Select => {
                        if let Some(name) = self.branches_state.selected_name() {
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
                BranchesMode::Creating => match action {
                    Action::Back => {
                        self.branches_state.new_name.clear();
                        self.branches_state.mode = BranchesMode::Browse;
                        Effect::None
                    }
                    Action::Char(c) => {
                        self.branches_state.push_name_char(c);
                        Effect::None
                    }
                    Action::Backspace => {
                        self.branches_state.pop_name_char();
                        Effect::None
                    }
                    Action::Select => {
                        let name = self.branches_state.new_name.trim().to_string();
                        if !name.is_empty() {
                            Effect::CreateBranch(name)
                        } else {
                            Effect::None
                        }
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
            Screen::History => {
                // Search mode: typing goes into the filter buffer.
                if self.history.search_mode {
                    return match action {
                        Action::Back => {
                            // Esc cancels search and clears the filter.
                            self.history.clear_filter();
                            Effect::None
                        }
                        Action::Select => {
                            // Enter applies the filter and exits search mode.
                            self.history.exit_search();
                            Effect::None
                        }
                        Action::Char(c) => {
                            self.history.push_filter_char(c);
                            Effect::None
                        }
                        Action::Backspace => {
                            self.history.pop_filter_char();
                            Effect::None
                        }
                        _ => Effect::None,
                    };
                }

                match action {
                    Action::Quit => Effect::Quit,
                    Action::Back => {
                        // If a filter is active in browse mode, clear it first.
                        if !self.history.filter.is_empty() {
                            self.history.clear_filter();
                            Effect::None
                        } else {
                            self.back_to_menu();
                            Effect::None
                        }
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
                    Action::Search => {
                        self.history.enter_search();
                        Effect::None
                    }
                    Action::Select => {
                        if let Some(entry) = self.history.selected_entry() {
                            let hash = entry.full_hash.clone();
                            self.screen = Screen::CommitDetails;
                            Effect::LoadCommitDetails(hash)
                        } else {
                            Effect::None
                        }
                    }
                    Action::Preview => {
                        if let Some(entry) = self.history.selected_entry() {
                            let hash = entry.full_hash.clone();
                            self.diff.return_to = Screen::History;
                            self.screen = Screen::DiffPreview;
                            Effect::LoadCommitDiff(hash)
                        } else {
                            Effect::None
                        }
                    }
                    _ => Effect::None,
                }
            }

            // ── Commit Details (read-only) ────────────────────────
            Screen::CommitDetails => match action {
                Action::Quit => Effect::Quit,
                Action::Back => {
                    self.screen = Screen::History;
                    Effect::None
                }
                Action::MoveUp => {
                    self.commit_details.scroll_up();
                    Effect::None
                }
                Action::MoveDown => {
                    self.commit_details.scroll_down();
                    Effect::None
                }
                Action::Preview | Action::Select => {
                    let hash = self.commit_details.source_hash.clone();
                    if !hash.is_empty() {
                        self.diff.return_to = Screen::CommitDetails;
                        self.screen = Screen::DiffPreview;
                        Effect::LoadCommitDiff(hash)
                    } else {
                        Effect::None
                    }
                }
                Action::Open => {
                    // Open the first file in the change list in $EDITOR.
                    if let Some(f) = self.commit_details.details.files.first() {
                        Effect::OpenEditor(f.path.clone())
                    } else {
                        Effect::None
                    }
                }
                _ => Effect::None,
            },

            // ── Rebase Portal (preview + confirm) ─────────────────
            Screen::RebasePortal => match self.rebase_portal.mode {
                RebasePortalMode::Preview => match action {
                    Action::Quit => Effect::Quit,
                    Action::Back => {
                        self.screen = self.rebase_portal.return_to;
                        Effect::None
                    }
                    Action::MoveUp => {
                        self.rebase_portal.scroll_up();
                        Effect::None
                    }
                    Action::MoveDown => {
                        self.rebase_portal.scroll_down();
                        Effect::None
                    }
                    Action::Select => {
                        // Enter advances to confirmation — but only if the
                        // preview reports the situation is actually executable.
                        if self.rebase_portal.can_execute() {
                            self.rebase_portal.mode = RebasePortalMode::Confirm;
                        }
                        Effect::None
                    }
                    _ => Effect::None,
                },
                RebasePortalMode::Confirm => match action {
                    Action::Quit => Effect::Quit,
                    Action::Back | Action::Deny => {
                        self.rebase_portal.mode = RebasePortalMode::Preview;
                        Effect::None
                    }
                    Action::Confirm | Action::Select => {
                        // Kick off the actual rebase. The event loop will
                        // run preflight and execute synchronously.
                        if let Some(p) = self.rebase_portal.preview.as_ref() {
                            let target = p.target.clone();
                            self.screen = Screen::RebaseExecute;
                            Effect::ExecuteRebase(target)
                        } else {
                            self.rebase_portal.mode = RebasePortalMode::Preview;
                            Effect::None
                        }
                    }
                    _ => Effect::None,
                },
            },

            // ── Rebase Execute (animation + result) ───────────────
            Screen::RebaseExecute => match self.rebase_execute.mode.clone() {
                RebaseExecuteMode::Animating => {
                    // Any key skips the animation. Using a wildcard here is
                    // deliberate — we never mutate the repo from this mode,
                    // so "any input means skip" is the only behaviour.
                    let _ = action;
                    self.rebase_execute.skip_animation();
                    Effect::None
                }
                RebaseExecuteMode::Success => match action {
                    Action::Quit => Effect::Quit,
                    Action::Back | Action::Select => {
                        self.screen = Screen::Branches;
                        Effect::LoadBranches
                    }
                    _ => Effect::None,
                },
                RebaseExecuteMode::Conflict { .. } => match action {
                    // Esc in conflict returns to branches *without* aborting —
                    // the user may want to resolve manually. Aborting and
                    // continuing are both deliberate explicit key presses.
                    Action::Back => {
                        self.screen = Screen::Branches;
                        Effect::LoadBranches
                    }
                    Action::Quit => Effect::Quit,
                    // 'a' (ToggleAll) → abort rebase.
                    Action::ToggleAll => Effect::AbortRebase,
                    // 'c' → continue a paused rebase. Safety-guarded at the
                    // git layer (refuses if no rebase is in progress on disk).
                    Action::Continue => Effect::ContinueRebase,
                    _ => Effect::None,
                },
                RebaseExecuteMode::Failure { .. } => match action {
                    Action::Quit => Effect::Quit,
                    Action::Back | Action::Select => {
                        self.screen = Screen::Branches;
                        Effect::LoadBranches
                    }
                    _ => Effect::None,
                },
                RebaseExecuteMode::Aborted { .. } => match action {
                    Action::Quit => Effect::Quit,
                    _ => {
                        self.screen = Screen::Branches;
                        Effect::LoadBranches
                    }
                },
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
                Action::Select => match self.settings_state.selection() {
                    SettingsSelection::TermMode(mode) => {
                        self.terms = Terms::new(mode);
                        Effect::SaveTermMode(mode)
                    }
                    SettingsSelection::Theme(preset) => {
                        // Apply the bare preset optimistically so the
                        // UI refreshes immediately. The effect handler
                        // will re-read the settings file afterwards so
                        // any hand-written `theme.*=` overrides fold
                        // back in.
                        self.theme = preset.palette();
                        Effect::SaveThemePreset(preset)
                    }
                },
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

            // ── Remote Sync ───────────────────────────────────────
            //
            // Browse is strictly read-only. A fetch requires passing
            // through ConfirmFetch → explicit Confirm/Select. Push and
            // Pull are intentionally not handled yet; keys that would
            // trigger them fall through to Effect::None.
            Screen::RemoteSync => match self.remote_sync.mode {
                RemoteSyncMode::Browse => match action {
                    Action::Quit => Effect::Quit,
                    Action::Back => {
                        self.back_to_menu();
                        Effect::None
                    }
                    Action::MoveUp => {
                        self.remote_sync.move_up();
                        Effect::None
                    }
                    Action::MoveDown => {
                        self.remote_sync.move_down();
                        Effect::None
                    }
                    Action::Refresh => Effect::LoadRemoteSync,
                    Action::Select => {
                        // Only arm a fetch if we actually have a remote
                        // to fetch from. Otherwise the screen stays a
                        // pure viewer — no accidental network traffic.
                        if self.remote_sync.fetch_target().is_some() {
                            self.remote_sync.mode = RemoteSyncMode::ConfirmFetch;
                        }
                        Effect::None
                    }
                    _ => Effect::None,
                },
                RemoteSyncMode::ConfirmFetch => match action {
                    Action::Confirm | Action::Select => {
                        if let Some(remote) = self.remote_sync.fetch_target() {
                            Effect::FetchFromRemote(remote)
                        } else {
                            self.remote_sync.mode = RemoteSyncMode::Browse;
                            Effect::None
                        }
                    }
                    Action::Deny | Action::Back => {
                        self.remote_sync.mode = RemoteSyncMode::Browse;
                        Effect::None
                    }
                    _ => Effect::None,
                },
                RemoteSyncMode::Result => {
                    // Any key dismisses the result and reloads the
                    // (read-only) remote info so ahead/behind refresh.
                    self.remote_sync.mode = RemoteSyncMode::Browse;
                    Effect::LoadRemoteSync
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
