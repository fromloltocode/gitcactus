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
            9 => "Quit",
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

    pub fn title_history(&self) -> &'static str {
        match self.mode {
            TermMode::Beginner => " Checkpoint History ",
            TermMode::Hybrid => " History ",
            TermMode::Git => " Commit History ",
        }
    }

    pub fn title_diff(&self) -> &'static str {
        match self.mode {
            TermMode::Beginner => " Compare Changes ",
            TermMode::Hybrid => " Diff Preview ",
            TermMode::Git => " Diff Preview ",
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
