//! XP rules, level math, and unlock evaluation.
//!
//! Kept entirely local in this phase — no network, no accounts.
//! The only thing the rest of the app interacts with is
//! [`apply_event`], which takes a [`ProgressionEvent`] and the current
//! [`Profile`] and produces an updated profile.
//!
//! # Design rules
//!
//! - XP should feel earned, not spammed. Meaningful user actions get
//!   reasonable amounts; passive events get little or nothing.
//! - Formulas stay simple and readable — no curves, no balancing
//!   config files, no per-action multipliers.
//! - Levels grow modestly: the goal is recognition, not grinding.
//! - Core Git functionality is **never** gated on level or unlocks.
//!   If someone wipes their profile, GitCactus still does everything
//!   it did before.

use crate::profile::Profile;

/// Meaningful actions a user can take. Emitted from the main event
/// loop after an effect has actually succeeded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressionEvent {
    /// User successfully staged N files.
    FilesStaged(u32),
    /// User successfully created a commit.
    CommitCreated,
    /// User created a new branch (New Game).
    BranchCreated,
    /// User switched branches safely (not force, not dirty).
    BranchSwitched,
    /// User completed a status → stage → commit chain.
    ComboChainCompleted,
    /// User completed a rebase without conflicts.
    RebaseCompleted,
}

impl ProgressionEvent {
    /// Flat XP value for this event. Kept tiny and boring on purpose.
    pub fn xp_value(&self) -> u32 {
        match self {
            // Staging scales with how much was staged, but capped so
            // hammering Space on a huge file list doesn't farm XP.
            Self::FilesStaged(n) => (*n).min(10),
            Self::CommitCreated => 15,
            Self::BranchCreated => 10,
            Self::BranchSwitched => 3,
            Self::ComboChainCompleted => 25,
            Self::RebaseCompleted => 20,
        }
    }
}

/// Apply an event to a profile, updating stats, XP, level, and unlocks
/// in-place. Saves nothing to disk — the caller decides when to
/// persist.
pub fn apply_event(profile: &mut Profile, event: ProgressionEvent) {
    // Update stat counters.
    match event {
        ProgressionEvent::FilesStaged(n) => {
            profile.stats.files_staged = profile.stats.files_staged.saturating_add(n);
        }
        ProgressionEvent::CommitCreated => {
            profile.stats.commits_created = profile.stats.commits_created.saturating_add(1);
        }
        ProgressionEvent::BranchCreated => {
            profile.stats.branches_created = profile.stats.branches_created.saturating_add(1);
        }
        ProgressionEvent::BranchSwitched => {
            profile.stats.branches_switched = profile.stats.branches_switched.saturating_add(1);
        }
        ProgressionEvent::ComboChainCompleted => {
            profile.stats.combos_completed = profile.stats.combos_completed.saturating_add(1);
        }
        ProgressionEvent::RebaseCompleted => {
            profile.stats.rebases_completed = profile.stats.rebases_completed.saturating_add(1);
        }
    }

    // Grant XP + recompute level + evaluate unlocks.
    profile.xp = profile.xp.saturating_add(event.xp_value());
    profile.level = level_for_xp(profile.xp);
    refresh_unlocks(profile);
}

/// XP threshold for reaching `level`. Level 1 = 0 XP; threshold grows
/// with `level * level * 50`. Modest, easy to reason about.
pub fn xp_for_level(level: u32) -> u32 {
    if level <= 1 {
        return 0;
    }
    let l = level.saturating_sub(1);
    l.saturating_mul(l).saturating_mul(50)
}

/// Given a total XP, return the level the user should be at.
pub fn level_for_xp(xp: u32) -> u32 {
    // Linear-search levels; profiles won't realistically reach huge
    // numbers and this keeps the math readable.
    let mut level = 1u32;
    loop {
        let next = level.saturating_add(1);
        if xp < xp_for_level(next) {
            return level;
        }
        level = next;
        if level >= 100 {
            return level; // hard cap, just in case
        }
    }
}

/// Returns `(current_level_xp, next_level_xp)` for progress bars.
/// For max level, `next_level_xp` equals `current_level_xp`.
pub fn xp_window(xp: u32) -> (u32, u32) {
    let level = level_for_xp(xp);
    let current = xp_for_level(level);
    let next = xp_for_level(level.saturating_add(1));
    if next <= current {
        (current, current)
    } else {
        (current, next)
    }
}

/// All possible unlocks, keyed by stable string IDs.
///
/// Adding a new unlock is a matter of appending here and rendering it
/// on the profile screen.
pub const UNLOCKS: &[(&str, &str, UnlockCondition)] = &[
    (
        "cactus_hat",
        "Cactus Hat",
        UnlockCondition::Level(3),
    ),
    (
        "rank_apprentice",
        "Rank: Apprentice",
        UnlockCondition::Level(2),
    ),
    (
        "rank_journeyman",
        "Rank: Journeyman",
        UnlockCondition::Level(5),
    ),
    (
        "combo_chainer",
        "Combo Chainer",
        UnlockCondition::CombosAtLeast(5),
    ),
    (
        "saved_path_surveyor",
        "Path Surveyor",
        UnlockCondition::BranchesCreatedAtLeast(3),
    ),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnlockCondition {
    Level(u32),
    CombosAtLeast(u32),
    BranchesCreatedAtLeast(u32),
}

impl UnlockCondition {
    pub fn is_met(&self, profile: &Profile) -> bool {
        match *self {
            Self::Level(n) => profile.level >= n,
            Self::CombosAtLeast(n) => profile.stats.combos_completed >= n,
            Self::BranchesCreatedAtLeast(n) => profile.stats.branches_created >= n,
        }
    }
}

/// Recompute the `unlocks` vec from the current profile state.
/// Any unlock already earned stays earned — we never revoke.
pub fn refresh_unlocks(profile: &mut Profile) {
    for (key, _label, cond) in UNLOCKS {
        if cond.is_met(profile) && !profile.unlocks.iter().any(|u| u == key) {
            profile.unlocks.push((*key).to_string());
        }
    }
    profile.unlocks.sort();
    profile.unlocks.dedup();
}

/// Stable skill-tree node descriptors. Used by the Skill Grid screen.
pub const SKILL_NODES: &[SkillNode] = &[
    SkillNode {
        id: "basic_moves",
        title: "BASIC MOVES",
        detail: "Navigate menus and screens with confidence.",
        unlock: SkillUnlock::Always,
    },
    SkillNode {
        id: "staging_mastery",
        title: "STAGING MASTERY",
        detail: "Stage changes deliberately.",
        unlock: SkillUnlock::FilesStaged(5),
    },
    SkillNode {
        id: "commit_flow",
        title: "COMMIT FLOW",
        detail: "Write commits that tell a story.",
        unlock: SkillUnlock::CommitsCreated(1),
    },
    SkillNode {
        id: "checkpoint_history",
        title: "CHECKPOINT HISTORY",
        detail: "Walk through past work.",
        unlock: SkillUnlock::CommitsCreated(3),
    },
    SkillNode {
        id: "saved_paths",
        title: "SAVED PATHS",
        detail: "Switch branches without fear.",
        unlock: SkillUnlock::BranchesSwitched(2),
    },
    SkillNode {
        id: "combo_chain",
        title: "COMBO CHAIN",
        detail: "Status → Stage → Commit, one smooth motion.",
        unlock: SkillUnlock::CombosCompleted(1),
    },
    SkillNode {
        id: "rebase_portal",
        title: "REBASE PORTAL",
        detail: "Replay commits onto a new base.",
        unlock: SkillUnlock::RebasesCompleted(1),
    },
    SkillNode {
        id: "conflict_survivor",
        title: "CONFLICT SURVIVOR",
        detail: "Resolve a conflict without losing work.",
        // No reliable local signal yet — reserved for a future phase.
        unlock: SkillUnlock::Reserved,
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkillNode {
    pub id: &'static str,
    pub title: &'static str,
    pub detail: &'static str,
    pub unlock: SkillUnlock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillUnlock {
    /// Always unlocked — visible from the first launch.
    Always,
    FilesStaged(u32),
    CommitsCreated(u32),
    BranchesSwitched(u32),
    CombosCompleted(u32),
    RebasesCompleted(u32),
    /// Cannot be unlocked from local signals in this phase.
    Reserved,
}

impl SkillUnlock {
    pub fn is_unlocked(&self, profile: &Profile) -> bool {
        match *self {
            Self::Always => true,
            Self::FilesStaged(n) => profile.stats.files_staged >= n,
            Self::CommitsCreated(n) => profile.stats.commits_created >= n,
            Self::BranchesSwitched(n) => profile.stats.branches_switched >= n,
            Self::CombosCompleted(n) => profile.stats.combos_completed >= n,
            Self::RebasesCompleted(n) => profile.stats.rebases_completed >= n,
            Self::Reserved => false,
        }
    }

    /// Short human-readable requirement, for the locked node tooltip.
    pub fn requirement_label(&self) -> String {
        match *self {
            Self::Always => "Available".into(),
            Self::FilesStaged(n) => format!("stage {n} files"),
            Self::CommitsCreated(n) => format!("create {n} commit{}", plural(n)),
            Self::BranchesSwitched(n) => format!("switch {n} branch{}", plural(n)),
            Self::CombosCompleted(n) => {
                format!("complete {n} status→stage→commit combo{}", plural(n))
            }
            Self::RebasesCompleted(n) => format!("complete {n} rebase{}", plural(n)),
            Self::Reserved => "(coming soon)".into(),
        }
    }
}

fn plural(n: u32) -> &'static str {
    if n == 1 {
        ""
    } else {
        "es"
    }
}
