//! Beginner-friendly terminology layer.
//!
//! Maps Git's technical vocabulary to friendlier labels without changing
//! how git operations actually work. Three modes:
//!
//! - **Beginner**: purely friendly language (e.g. "Checkpoint")
//! - **Hybrid** (default): friendly label with git term (e.g. "Checkpoint (Commit)")
//! - **Git**: standard git terminology
//!
//! The translation sits above the real git implementation — it only
//! affects what users *see*, never what git *does*.

/// Which terminology mode is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TermMode {
    Beginner,
    Hybrid,
    Git,
}

impl TermMode {
    /// Parse from a settings string. Returns `None` for unrecognised values.
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "beginner" => Some(Self::Beginner),
            "hybrid" => Some(Self::Hybrid),
            "git" => Some(Self::Git),
            _ => None,
        }
    }

    /// Serialise for the settings file.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Beginner => "beginner",
            Self::Hybrid => "hybrid",
            Self::Git => "git",
        }
    }
}

/// Terminology lookup. All methods return `&'static str` — zero
/// allocations at render time.
#[derive(Debug, Clone, Copy)]
pub struct Terms {
    pub mode: TermMode,
}

impl Terms {
    pub fn new(mode: TermMode) -> Self {
        Self { mode }
    }

    // ── Core concepts ────────────────────────────────────────────

    pub fn commit(&self) -> &'static str {
        match self.mode {
            TermMode::Beginner => "Checkpoint",
            TermMode::Hybrid => "Checkpoint (Commit)",
            TermMode::Git => "Commit",
        }
    }

    pub fn commit_verb(&self) -> &'static str {
        match self.mode {
            TermMode::Beginner => "Save Checkpoint",
            TermMode::Hybrid => "Create Checkpoint (Commit)",
            TermMode::Git => "Commit Changes",
        }
    }

    pub fn commit_history(&self) -> &'static str {
        match self.mode {
            TermMode::Beginner => "Checkpoint History",
            TermMode::Hybrid => "Checkpoint History (Commit Log)",
            TermMode::Git => "Commit History",
        }
    }

    pub fn branch(&self) -> &'static str {
        match self.mode {
            TermMode::Beginner => "Saved Path",
            TermMode::Hybrid => "Saved Path (Branch)",
            TermMode::Git => "Branch",
        }
    }

    pub fn current_branch(&self) -> &'static str {
        match self.mode {
            TermMode::Beginner => "Current Path",
            TermMode::Hybrid => "Current Path (Branch)",
            TermMode::Git => "Current Branch",
        }
    }

    pub fn head(&self) -> &'static str {
        match self.mode {
            TermMode::Beginner => "Last Saved",
            TermMode::Hybrid => "Last Saved (HEAD)",
            TermMode::Git => "HEAD",
        }
    }

    pub fn staged(&self) -> &'static str {
        match self.mode {
            TermMode::Beginner => "Ready to Save",
            TermMode::Hybrid => "Ready to Save (Staged)",
            TermMode::Git => "Staged",
        }
    }

    pub fn stage_verb(&self) -> &'static str {
        match self.mode {
            TermMode::Beginner => "Prepare to Save",
            TermMode::Hybrid => "Prepare to Save (Stage)",
            TermMode::Git => "Stage Changes",
        }
    }

    pub fn modified(&self) -> &'static str {
        match self.mode {
            TermMode::Beginner => "Changed",
            TermMode::Hybrid => "Changed (Modified)",
            TermMode::Git => "Modified",
        }
    }

    pub fn untracked(&self) -> &'static str {
        match self.mode {
            TermMode::Beginner => "New Files",
            TermMode::Hybrid => "New Files (Untracked)",
            TermMode::Git => "Untracked",
        }
    }

    pub fn working_changes(&self) -> &'static str {
        "Working Changes" // same in all modes
    }

    pub fn diff(&self) -> &'static str {
        match self.mode {
            TermMode::Beginner => "Compare Changes",
            TermMode::Hybrid => "Compare Changes (Diff)",
            TermMode::Git => "Diff",
        }
    }

    /// Label for the branch-creation action.
    pub fn new_branch_action(&self) -> &'static str {
        match self.mode {
            TermMode::Beginner => "New Game",
            TermMode::Hybrid => "New Game (New Path)",
            TermMode::Git => "Create Branch",
        }
    }

    /// Title for the branch-creation dialog.
    pub fn new_branch_title(&self) -> &'static str {
        match self.mode {
            TermMode::Beginner => " Start a New Game ",
            TermMode::Hybrid => " Start a New Game (New Path) ",
            TermMode::Git => " Create Branch ",
        }
    }

    /// Short explanation shown in the creation dialog.
    pub fn new_branch_description(&self) -> &'static str {
        match self.mode {
            TermMode::Beginner => {
                "A new game creates a new path from your current progress. \
                 Experiment freely — your main path stays safe."
            }
            TermMode::Hybrid => {
                "A new game is a new path (branch) starting from your current \
                 commit. Experiment freely — your main path stays safe."
            }
            TermMode::Git => {
                "Create a new branch pointing at the current HEAD. \
                 The new branch will diverge from here."
            }
        }
    }

    pub fn merge(&self) -> &'static str {
        match self.mode {
            TermMode::Beginner => "Combine Paths",
            TermMode::Hybrid => "Combine Paths (Merge)",
            TermMode::Git => "Merge",
        }
    }

    pub fn sync(&self) -> &'static str {
        "Sync" // same in all modes
    }

    /// Entry-point label for the rebase portal (used on the Branches screen).
    pub fn rebase_action(&self) -> &'static str {
        match self.mode {
            TermMode::Beginner => "Rebase Portal",
            TermMode::Hybrid => "Rebase Portal (Rebase)",
            TermMode::Git => "Rebase",
        }
    }

    /// Title of the portal preview screen.
    pub fn title_rebase_portal(&self) -> &'static str {
        match self.mode {
            TermMode::Beginner => " Rebase Portal ",
            TermMode::Hybrid => " Rebase Portal (Rebase) ",
            TermMode::Git => " Rebase Preview ",
        }
    }

    /// Title of the rebase execution / animation screen.
    pub fn title_rebase_execute(&self) -> &'static str {
        match self.mode {
            TermMode::Beginner => " Rebase Portal \u{2014} Running ",
            TermMode::Hybrid => " Rebase Portal (Rebase) \u{2014} Running ",
            TermMode::Git => " Rebase \u{2014} Running ",
        }
    }

    /// Honest description of what rebase does, used in preview + confirm.
    pub fn rebase_description(&self) -> &'static str {
        match self.mode {
            TermMode::Beginner => {
                "A portal replays your checkpoints on top of a new base. \
                 Your history gets rewritten \u{2014} avoid on shared paths."
            }
            TermMode::Hybrid => {
                "Rebase replays your commits on top of a new base. \
                 It rewrites history \u{2014} avoid on shared branches."
            }
            TermMode::Git => {
                "Rebase replays commits in <current>..<target> on top of <target>. \
                 History is rewritten \u{2014} do not use on shared refs."
            }
        }
    }

    // ── Menu labels (per screen) ─────────────────────────────────

    pub fn menu_status(&self) -> &'static str {
        "Status" // universal
    }

    pub fn menu_stage(&self) -> &'static str {
        match self.mode {
            TermMode::Beginner => "Prepare to Save",
            TermMode::Hybrid => "Stage Changes",
            TermMode::Git => "Stage Changes",
        }
    }

    pub fn menu_commit(&self) -> &'static str {
        match self.mode {
            TermMode::Beginner => "Save Checkpoint",
            TermMode::Hybrid => "Commit Changes",
            TermMode::Git => "Commit Changes",
        }
    }

    pub fn menu_branches(&self) -> &'static str {
        match self.mode {
            TermMode::Beginner => "Saved Paths",
            TermMode::Hybrid => "Branches",
            TermMode::Git => "Branches",
        }
    }

    pub fn menu_history(&self) -> &'static str {
        match self.mode {
            TermMode::Beginner => "Checkpoint History",
            TermMode::Hybrid => "History",
            TermMode::Git => "History",
        }
    }

    pub fn menu_sync(&self) -> &'static str {
        match self.mode {
            TermMode::Beginner => "Sync with Team",
            TermMode::Hybrid => "Remote Sync",
            TermMode::Git => "Remote Sync",
        }
    }

    /// Return the display label for a menu item by its index in MENU_ITEMS.
    pub fn menu_label(&self, index: usize) -> &str {
        match index {
            0 => self.menu_status(),
            1 => self.menu_stage(),
            2 => self.menu_commit(),
            3 => self.menu_branches(),
            4 => self.menu_history(),
            5 => self.menu_sync(),
            6 => "Controls",
            7 => "Check for Updates",
            8 => "Settings",
            9 => "Skill Tree",
            10 => "Quit",
            _ => "???",
        }
    }

    // ── Screen titles ────────────────────────────────────────────

    pub fn title_status(&self) -> &'static str {
        " Status "
    }

    pub fn title_stage(&self) -> &'static str {
        match self.mode {
            TermMode::Beginner => " Prepare to Save ",
            TermMode::Hybrid => " Stage Changes ",
            TermMode::Git => " Stage Changes ",
        }
    }

    pub fn title_commit(&self) -> &'static str {
        match self.mode {
            TermMode::Beginner => " Save Checkpoint ",
            TermMode::Hybrid => " Commit Changes ",
            TermMode::Git => " Commit Changes ",
        }
    }

    pub fn title_branches(&self) -> &'static str {
        match self.mode {
            TermMode::Beginner => " Saved Paths ",
            TermMode::Hybrid => " Branches ",
            TermMode::Git => " Branches ",
        }
    }

    pub fn title_history(&self) -> &'static str {
        match self.mode {
            TermMode::Beginner => " Checkpoint History ",
            TermMode::Hybrid => " History ",
            TermMode::Git => " Commit History ",
        }
    }

    pub fn title_commit_details(&self) -> &'static str {
        match self.mode {
            TermMode::Beginner => " Checkpoint Details ",
            TermMode::Hybrid => " Checkpoint Details (Commit) ",
            TermMode::Git => " Commit Details ",
        }
    }

    pub fn title_diff(&self) -> &'static str {
        match self.mode {
            TermMode::Beginner => " Compare Changes ",
            TermMode::Hybrid => " Diff Preview ",
            TermMode::Git => " Diff Preview ",
        }
    }

    pub fn title_remote_sync(&self) -> &'static str {
        match self.mode {
            TermMode::Beginner => " Sync with Team ",
            TermMode::Hybrid => " Remote Sync ",
            TermMode::Git => " Remote Sync ",
        }
    }

    /// One-sentence, plain-English explanation of what a "remote" is.
    pub fn remote_description(&self) -> &'static str {
        match self.mode {
            TermMode::Beginner => {
                "A remote is a copy of your project on another machine \
                 (like GitHub). Fetch pulls down its latest state without \
                 changing your work."
            }
            TermMode::Hybrid => {
                "A remote is a named link to another copy of the repo \
                 (usually on a server). Fetch updates your local view of \
                 it without touching your branches."
            }
            TermMode::Git => {
                "Fetch downloads refs and objects from the named remote \
                 into refs/remotes/<remote>/*. It does not modify HEAD, \
                 the index, the working tree, or any local branch."
            }
        }
    }

    /// A short label showing the current terminology mode.
    pub fn mode_label(&self) -> &'static str {
        match self.mode {
            TermMode::Beginner => "Beginner",
            TermMode::Hybrid => "Hybrid",
            TermMode::Git => "Git",
        }
    }
}
