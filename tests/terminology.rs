//! Tests for the terminology system.

use gitcactus::terminology::{TermMode, Terms};

// ── Mode parsing ─────────────────────────────────────────────────────

#[test]
fn parse_beginner() {
    assert_eq!(TermMode::parse("beginner"), Some(TermMode::Beginner));
}

#[test]
fn parse_hybrid() {
    assert_eq!(TermMode::parse("hybrid"), Some(TermMode::Hybrid));
}

#[test]
fn parse_git() {
    assert_eq!(TermMode::parse("git"), Some(TermMode::Git));
}

#[test]
fn parse_case_insensitive() {
    assert_eq!(TermMode::parse("BEGINNER"), Some(TermMode::Beginner));
    assert_eq!(TermMode::parse("Hybrid"), Some(TermMode::Hybrid));
    assert_eq!(TermMode::parse("GIT"), Some(TermMode::Git));
}

#[test]
fn parse_trims_whitespace() {
    assert_eq!(TermMode::parse("  hybrid  "), Some(TermMode::Hybrid));
}

#[test]
fn parse_unknown_returns_none() {
    assert_eq!(TermMode::parse(""), None);
    assert_eq!(TermMode::parse("advanced"), None);
    assert_eq!(TermMode::parse("pro"), None);
}

#[test]
fn as_str_round_trips() {
    for mode in [TermMode::Beginner, TermMode::Hybrid, TermMode::Git] {
        let s = mode.as_str();
        assert_eq!(TermMode::parse(s), Some(mode));
    }
}

// ── Beginner mode labels ─────────────────────────────────────────────

#[test]
fn beginner_uses_friendly_labels() {
    let t = Terms::new(TermMode::Beginner);
    assert_eq!(t.commit(), "Checkpoint");
    assert_eq!(t.branch(), "Saved Path");
    assert_eq!(t.staged(), "Ready to Save");
    assert_eq!(t.modified(), "Changed");
    assert_eq!(t.untracked(), "New Files");
    assert_eq!(t.diff(), "Compare Changes");
    assert_eq!(t.head(), "Last Saved");
    assert_eq!(t.current_branch(), "Current Path");
    assert_eq!(t.merge(), "Combine Paths");
}

// ── Hybrid mode labels ───────────────────────────────────────────────

#[test]
fn hybrid_includes_both_terms() {
    let t = Terms::new(TermMode::Hybrid);
    assert!(t.commit().contains("Checkpoint"));
    assert!(t.commit().contains("Commit"));
    assert!(t.staged().contains("Ready to Save"));
    assert!(t.staged().contains("Staged"));
    assert!(t.diff().contains("Compare Changes"));
    assert!(t.diff().contains("Diff"));
}

// ── Git mode labels ──────────────────────────────────────────────────

#[test]
fn git_mode_uses_standard_terms() {
    let t = Terms::new(TermMode::Git);
    assert_eq!(t.commit(), "Commit");
    assert_eq!(t.branch(), "Branch");
    assert_eq!(t.staged(), "Staged");
    assert_eq!(t.modified(), "Modified");
    assert_eq!(t.untracked(), "Untracked");
    assert_eq!(t.diff(), "Diff");
    assert_eq!(t.head(), "HEAD");
    assert_eq!(t.current_branch(), "Current Branch");
    assert_eq!(t.merge(), "Merge");
}

// ── Concepts that are the same across modes ──────────────────────────

#[test]
fn working_changes_same_in_all_modes() {
    for mode in [TermMode::Beginner, TermMode::Hybrid, TermMode::Git] {
        let t = Terms::new(mode);
        assert_eq!(t.working_changes(), "Working Changes");
    }
}

#[test]
fn sync_same_in_all_modes() {
    for mode in [TermMode::Beginner, TermMode::Hybrid, TermMode::Git] {
        let t = Terms::new(mode);
        assert_eq!(t.sync(), "Sync");
    }
}

// ── Menu labels ──────────────────────────────────────────────────────

#[test]
fn menu_label_covers_all_items() {
    let t = Terms::new(TermMode::Hybrid);
    for i in 0..=8 {
        let label = t.menu_label(i);
        assert!(!label.is_empty(), "Menu label {i} should not be empty");
        assert_ne!(label, "???", "Menu label {i} should not be unknown");
    }
}

#[test]
fn beginner_menu_labels_differ_from_git() {
    let b = Terms::new(TermMode::Beginner);
    let g = Terms::new(TermMode::Git);
    // Stage and Commit should have different labels
    assert_ne!(b.menu_stage(), g.menu_stage());
    assert_ne!(b.menu_commit(), g.menu_commit());
    assert_ne!(b.menu_branches(), g.menu_branches());
}

// ── Mode label ───────────────────────────────────────────────────────

#[test]
fn mode_label_is_human_readable() {
    assert_eq!(Terms::new(TermMode::Beginner).mode_label(), "Beginner");
    assert_eq!(Terms::new(TermMode::Hybrid).mode_label(), "Hybrid");
    assert_eq!(Terms::new(TermMode::Git).mode_label(), "Git");
}

// ── Accuracy: no misleading mappings ─────────────────────────────────

#[test]
fn branch_is_not_checkpoint() {
    let t = Terms::new(TermMode::Beginner);
    // Branch should say "Path", not "Checkpoint" — checkpoints are commits
    assert!(!t.branch().contains("Checkpoint"));
    assert!(!t.current_branch().contains("Checkpoint"));
}

#[test]
fn commit_is_not_branch() {
    let t = Terms::new(TermMode::Beginner);
    assert!(!t.commit().contains("Path"));
    assert!(!t.commit().contains("Branch"));
}

// ── Screen titles ────────────────────────────────────────────────────

#[test]
fn screen_titles_non_empty_in_all_modes() {
    for mode in [TermMode::Beginner, TermMode::Hybrid, TermMode::Git] {
        let t = Terms::new(mode);
        assert!(!t.title_status().is_empty());
        assert!(!t.title_stage().is_empty());
        assert!(!t.title_commit().is_empty());
        assert!(!t.title_diff().is_empty());
    }
}
