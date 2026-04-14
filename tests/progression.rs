//! Tests for the local progression system.
//!
//! All progression state and math is local. These tests exist to pin
//! down the XP / level / unlock semantics so future refactors don't
//! silently change how much XP someone earns.

use gitcactus::profile::{Profile, Stats};
use gitcactus::progression::{
    apply_event, level_for_xp, refresh_unlocks, xp_for_level, xp_window,
    ProgressionEvent, SkillUnlock, SKILL_NODES, UNLOCKS,
};

// ── Profile parse / round-trip ───────────────────────────────────────

#[test]
fn default_profile_starts_level_one_no_xp() {
    let p = Profile::default();
    assert_eq!(p.level, 1);
    assert_eq!(p.xp, 0);
    assert_eq!(p.stats, Stats::default());
    assert!(p.unlocks.is_empty());
}

#[test]
fn profile_from_empty_text_equals_default() {
    let p = Profile::from_text("");
    assert_eq!(p, Profile::default());
}

#[test]
fn profile_round_trip_text() {
    let p = Profile {
        xp: 175,
        level: 2,
        stats: Stats {
            commits_created: 4,
            files_staged: 9,
            ..Stats::default()
        },
        unlocks: vec!["rank_apprentice".into()],
        ..Profile::default()
    };

    let text = p.to_text();
    let parsed = Profile::from_text(&text);
    assert_eq!(parsed, p);
}

#[test]
fn profile_ignores_unknown_keys() {
    let text = "xp=50\nlevel=2\nbogus_key=whatever\ncommits_created=3\n";
    let p = Profile::from_text(text);
    assert_eq!(p.xp, 50);
    assert_eq!(p.level, 2);
    assert_eq!(p.stats.commits_created, 3);
}

#[test]
fn profile_malformed_numbers_fall_back_silently() {
    let text = "xp=notanumber\nlevel=also_bad\ncommits_created=\n";
    let p = Profile::from_text(text);
    assert_eq!(p, Profile::default());
}

#[test]
fn profile_level_zero_is_clamped_to_one() {
    // We never want a stored "level=0" to render as "LEVEL 0".
    let p = Profile::from_text("level=0\n");
    assert_eq!(p.level, 1);
}

#[test]
fn profile_unlocks_parsed_sorted_and_deduped() {
    let p = Profile::from_text("unlocks=cactus_hat,rank_apprentice,cactus_hat\n");
    assert_eq!(p.unlocks, vec!["cactus_hat", "rank_apprentice"]);
}

// ── XP / level math ──────────────────────────────────────────────────

#[test]
fn xp_for_level_is_zero_at_level_one() {
    assert_eq!(xp_for_level(1), 0);
}

#[test]
fn xp_for_level_grows_quadratically() {
    // Exact values pinned so refactors are deliberate.
    assert_eq!(xp_for_level(2), 50);
    assert_eq!(xp_for_level(3), 200);
    assert_eq!(xp_for_level(4), 450);
    assert_eq!(xp_for_level(5), 800);
}

#[test]
fn level_for_xp_roundtrips_with_xp_for_level() {
    for lvl in 1..=10u32 {
        let xp = xp_for_level(lvl);
        assert_eq!(level_for_xp(xp), lvl, "xp={xp} should be level {lvl}");
        if lvl > 1 {
            // Just under the threshold should be the previous level.
            assert_eq!(level_for_xp(xp - 1), lvl - 1);
        }
    }
}

#[test]
fn xp_window_reports_correct_bounds() {
    let (cur, next) = xp_window(0);
    assert_eq!(cur, 0);
    assert_eq!(next, 50);

    let (cur, next) = xp_window(75);
    assert_eq!(cur, 50);
    assert_eq!(next, 200);
}

// ── Event rules ──────────────────────────────────────────────────────

#[test]
fn commit_event_awards_15_xp_and_counts() {
    let mut p = Profile::default();
    apply_event(&mut p, ProgressionEvent::CommitCreated);
    assert_eq!(p.xp, 15);
    assert_eq!(p.stats.commits_created, 1);
}

#[test]
fn files_staged_xp_caps_at_ten() {
    let mut p = Profile::default();
    apply_event(&mut p, ProgressionEvent::FilesStaged(3));
    assert_eq!(p.xp, 3);
    assert_eq!(p.stats.files_staged, 3);

    let mut p2 = Profile::default();
    apply_event(&mut p2, ProgressionEvent::FilesStaged(100));
    assert_eq!(p2.xp, 10, "XP from staging caps at 10 per event");
    assert_eq!(p2.stats.files_staged, 100, "stat still counts real files");
}

#[test]
fn branch_created_grants_10_xp() {
    let mut p = Profile::default();
    apply_event(&mut p, ProgressionEvent::BranchCreated);
    assert_eq!(p.xp, 10);
    assert_eq!(p.stats.branches_created, 1);
}

#[test]
fn branch_switched_grants_only_3_xp() {
    // Switching is cheap — reward low to avoid encouraging XP farming.
    let mut p = Profile::default();
    apply_event(&mut p, ProgressionEvent::BranchSwitched);
    assert_eq!(p.xp, 3);
    assert_eq!(p.stats.branches_switched, 1);
}

#[test]
fn combo_chain_grants_25_xp() {
    let mut p = Profile::default();
    apply_event(&mut p, ProgressionEvent::ComboChainCompleted);
    assert_eq!(p.xp, 25);
    assert_eq!(p.stats.combos_completed, 1);
}

#[test]
fn rebase_completed_grants_20_xp() {
    let mut p = Profile::default();
    apply_event(&mut p, ProgressionEvent::RebaseCompleted);
    assert_eq!(p.xp, 20);
    assert_eq!(p.stats.rebases_completed, 1);
}

#[test]
fn level_advances_on_threshold_crossing() {
    let mut p = Profile::default();
    // 4 commits = 60 XP → past 50 threshold, so level 2.
    for _ in 0..4 {
        apply_event(&mut p, ProgressionEvent::CommitCreated);
    }
    assert_eq!(p.xp, 60);
    assert_eq!(p.level, 2);
}

// ── Unlocks ──────────────────────────────────────────────────────────

#[test]
fn rank_apprentice_unlocks_at_level_two() {
    let mut p = Profile::default();
    // Push level to 2: 50 XP = threshold.
    p.xp = 50;
    p.level = level_for_xp(p.xp);
    refresh_unlocks(&mut p);
    assert!(p.unlocks.iter().any(|u| u == "rank_apprentice"));
    // Cactus hat (level 3) should NOT be unlocked yet.
    assert!(!p.unlocks.iter().any(|u| u == "cactus_hat"));
}

#[test]
fn cactus_hat_unlocks_at_level_three() {
    let mut p = Profile::default();
    p.xp = 200;
    p.level = level_for_xp(p.xp);
    refresh_unlocks(&mut p);
    assert!(p.unlocks.iter().any(|u| u == "cactus_hat"));
}

#[test]
fn combo_chainer_unlocks_after_five_combos() {
    let mut p = Profile::default();
    for _ in 0..5 {
        apply_event(&mut p, ProgressionEvent::ComboChainCompleted);
    }
    assert!(p.unlocks.iter().any(|u| u == "combo_chainer"));
}

#[test]
fn path_surveyor_unlocks_after_three_branches_created() {
    let mut p = Profile::default();
    for _ in 0..3 {
        apply_event(&mut p, ProgressionEvent::BranchCreated);
    }
    assert!(p.unlocks.iter().any(|u| u == "saved_path_surveyor"));
}

#[test]
fn unlocks_never_duplicate_or_regress() {
    let mut p = Profile::default();
    p.xp = 1000;
    p.level = level_for_xp(p.xp);
    refresh_unlocks(&mut p);
    let first = p.unlocks.clone();
    refresh_unlocks(&mut p);
    assert_eq!(p.unlocks, first, "idempotent refresh");

    // Lowering level should not revoke earned unlocks.
    p.level = 1;
    refresh_unlocks(&mut p);
    assert!(p.unlocks.contains(&"cactus_hat".to_string()));
}

#[test]
fn unlocks_catalog_is_non_empty_and_labels_non_empty() {
    assert!(!UNLOCKS.is_empty());
    for (key, label, _cond) in UNLOCKS {
        assert!(!key.is_empty());
        assert!(!label.is_empty());
    }
}

// ── Skill tree derivation ────────────────────────────────────────────

#[test]
fn basic_moves_node_is_always_unlocked() {
    let p = Profile::default();
    let basic = SKILL_NODES
        .iter()
        .find(|n| n.id == "basic_moves")
        .expect("basic_moves node must exist");
    assert!(basic.unlock.is_unlocked(&p));
    assert!(matches!(basic.unlock, SkillUnlock::Always));
}

#[test]
fn commit_flow_node_unlocks_after_first_commit() {
    let mut p = Profile::default();
    let node = SKILL_NODES
        .iter()
        .find(|n| n.id == "commit_flow")
        .expect("commit_flow node must exist");
    assert!(!node.unlock.is_unlocked(&p));
    apply_event(&mut p, ProgressionEvent::CommitCreated);
    assert!(node.unlock.is_unlocked(&p));
}

#[test]
fn rebase_portal_node_requires_completed_rebase() {
    let p = Profile::default();
    let node = SKILL_NODES
        .iter()
        .find(|n| n.id == "rebase_portal")
        .expect("rebase_portal node must exist");
    assert!(!node.unlock.is_unlocked(&p));

    let mut p2 = p;
    apply_event(&mut p2, ProgressionEvent::RebaseCompleted);
    assert!(node.unlock.is_unlocked(&p2));
}

#[test]
fn reserved_nodes_never_auto_unlock() {
    let mut p = Profile {
        xp: u32::MAX / 2,
        level: 100,
        ..Profile::default()
    };
    refresh_unlocks(&mut p);
    for node in SKILL_NODES {
        if matches!(node.unlock, SkillUnlock::Reserved) {
            assert!(
                !node.unlock.is_unlocked(&p),
                "reserved node {} should stay locked",
                node.id
            );
        }
    }
}

#[test]
fn requirement_label_non_empty_for_all_unlock_kinds() {
    let kinds = [
        SkillUnlock::Always,
        SkillUnlock::FilesStaged(5),
        SkillUnlock::CommitsCreated(1),
        SkillUnlock::BranchesSwitched(2),
        SkillUnlock::CombosCompleted(1),
        SkillUnlock::RebasesCompleted(1),
        SkillUnlock::Reserved,
    ];
    for k in kinds {
        assert!(!k.requirement_label().is_empty());
    }
}

// ── Local-first sanity ───────────────────────────────────────────────

#[test]
fn apply_event_does_not_touch_network() {
    // Static claim: progression is pure math on a Profile struct.
    // Calling apply_event here just exercises the public entry point
    // so future code that adds network calls would trip the test on
    // sandbox environments.
    let mut p = Profile::default();
    for _ in 0..10 {
        apply_event(&mut p, ProgressionEvent::CommitCreated);
    }
    assert_eq!(p.xp, 150);
    assert_eq!(p.stats.commits_created, 10);
}
