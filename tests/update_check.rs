//! Tests for the update / version-check logic.
//!
//! No network. We inject the tag-fetcher closure so tests are
//! deterministic and offline.

use gitcactus::update::{
    check_for_updates_with, parse_ls_remote_output, SemVer, VERSION,
};

// ── SemVer parsing ───────────────────────────────────────────────────

#[test]
fn parses_plain_triple() {
    assert_eq!(
        SemVer::parse("0.4.0"),
        Some(SemVer { major: 0, minor: 4, patch: 0 })
    );
}

#[test]
fn parses_v_prefix() {
    assert_eq!(
        SemVer::parse("v0.4.0"),
        Some(SemVer { major: 0, minor: 4, patch: 0 })
    );
}

#[test]
fn parses_trims_whitespace() {
    assert_eq!(
        SemVer::parse("  v1.2.3  "),
        Some(SemVer { major: 1, minor: 2, patch: 3 })
    );
}

#[test]
fn prerelease_core_only() {
    // Pre-release / build metadata is dropped to the numeric core.
    assert_eq!(
        SemVer::parse("0.4.0-rc1"),
        Some(SemVer { major: 0, minor: 4, patch: 0 })
    );
    assert_eq!(
        SemVer::parse("1.2.3+build.5"),
        Some(SemVer { major: 1, minor: 2, patch: 3 })
    );
}

#[test]
fn rejects_garbage() {
    assert!(SemVer::parse("").is_none());
    assert!(SemVer::parse("not-a-version").is_none());
    assert!(SemVer::parse("0.4").is_none()); // too short
    assert!(SemVer::parse("0.4.0.0").is_none()); // too long
    assert!(SemVer::parse("0.4.-1").is_none()); // not parseable as u64
    assert!(SemVer::parse("v").is_none());
    assert!(SemVer::parse("vx.y.z").is_none());
}

// ── SemVer ordering ──────────────────────────────────────────────────

#[test]
fn numeric_ordering_not_string_ordering() {
    // The original bug: string-compared "0.10.0" < "0.2.0".
    // Numeric semver must order them the other way.
    let v0_2_0 = SemVer::parse("0.2.0").unwrap();
    let v0_4_0 = SemVer::parse("0.4.0").unwrap();
    let v0_10_0 = SemVer::parse("0.10.0").unwrap();

    assert!(v0_2_0 < v0_4_0);
    assert!(v0_4_0 < v0_10_0);
    assert!(v0_2_0 < v0_10_0);
}

#[test]
fn major_beats_minor_beats_patch() {
    let a = SemVer::parse("1.0.0").unwrap();
    let b = SemVer::parse("0.99.99").unwrap();
    let c = SemVer::parse("0.1.5").unwrap();
    let d = SemVer::parse("0.1.4").unwrap();
    assert!(a > b);
    assert!(c > d);
}

// ── parse_ls_remote_output ───────────────────────────────────────────

#[test]
fn parses_standard_ls_remote_output() {
    let stdout = "\
abc123\trefs/tags/v0.1.0
def456\trefs/tags/v0.2.0
ghi789\trefs/tags/v0.4.0
";
    let tags = parse_ls_remote_output(stdout);
    assert_eq!(tags, vec!["v0.1.0", "v0.2.0", "v0.4.0"]);
}

#[test]
fn strips_peeled_tag_suffix() {
    // `--refs` normally removes these, but just in case.
    let stdout = "abc123\trefs/tags/v0.4.0^{}\n";
    let tags = parse_ls_remote_output(stdout);
    assert_eq!(tags, vec!["v0.4.0"]);
}

#[test]
fn ignores_non_tag_lines() {
    let stdout = "\
abc123\tHEAD
def456\trefs/heads/main
ghi789\trefs/tags/v0.4.0
";
    let tags = parse_ls_remote_output(stdout);
    assert_eq!(tags, vec!["v0.4.0"]);
}

#[test]
fn empty_output_yields_no_tags() {
    assert!(parse_ls_remote_output("").is_empty());
}

// ── check_for_updates_with: full result assembly ─────────────────────

#[test]
fn identifies_newer_latest_correctly() {
    // Current is whatever Cargo.toml says; pretend upstream has a
    // major new release (999.0.0) — we should NOT be up to date.
    let info = check_for_updates_with(|| Ok(vec!["v999.0.0".into()]));
    assert_eq!(info.current, VERSION);
    assert_eq!(info.latest, "999.0.0");
    assert!(!info.is_up_to_date);
    assert!(info.error.is_none());
}

#[test]
fn identifies_up_to_date_when_we_are_newest() {
    // Feed only a tag older than the current build: we're up to date.
    let info = check_for_updates_with(|| Ok(vec!["v0.0.1".into()]));
    assert_eq!(info.current, VERSION);
    assert!(info.is_up_to_date);
    assert!(info.error.is_none());
}

#[test]
fn identifies_up_to_date_when_latest_equals_current() {
    let current = VERSION.to_string();
    let tag = format!("v{current}");
    let info = check_for_updates_with(move || Ok(vec![tag]));
    assert_eq!(info.latest, current);
    assert!(info.is_up_to_date);
}

#[test]
fn picks_highest_tag_regardless_of_input_order() {
    let info = check_for_updates_with(|| {
        Ok(vec![
            "v0.1.0".into(),
            "v0.10.0".into(), // the semver-highest
            "v0.2.0".into(),
            "v0.4.0".into(),
        ])
    });
    assert_eq!(info.latest, "0.10.0");
    // And therefore newer than current (which is 0.4.0 at time of writing).
    assert!(!info.is_up_to_date);
}

#[test]
fn ignores_malformed_tag_names() {
    let info = check_for_updates_with(|| {
        Ok(vec![
            "not-a-version".into(),
            "v0.4.0".into(),
            "release-4".into(),
            "v0.5.0".into(),
        ])
    });
    assert_eq!(info.latest, "0.5.0");
}

#[test]
fn empty_tag_list_falls_back_to_current() {
    let info = check_for_updates_with(|| Ok(Vec::<String>::new()));
    assert_eq!(info.latest, VERSION);
    assert!(info.is_up_to_date);
    assert!(info.error.as_ref().is_some_and(|e| e.to_lowercase().contains("no version tags")));
}

#[test]
fn network_error_is_recorded_and_never_panics() {
    let info = check_for_updates_with(|| Err("offline".into()));
    assert_eq!(info.latest, VERSION);
    // On error we report up-to-date rather than falsely claim behind.
    assert!(info.is_up_to_date);
    assert_eq!(info.error.as_deref(), Some("offline"));
}

#[test]
fn latest_never_has_leading_v() {
    let info = check_for_updates_with(|| Ok(vec!["v0.5.0".into()]));
    assert!(!info.latest.starts_with('v'), "latest must be plain semver");
}

// ── Sanity: VERSION matches Cargo.toml at compile time ───────────────

#[test]
fn version_constant_matches_cargo_pkg_version() {
    assert_eq!(VERSION, env!("CARGO_PKG_VERSION"));
    // Should parse as a valid semver too.
    assert!(
        SemVer::parse(VERSION).is_some(),
        "CARGO_PKG_VERSION '{VERSION}' must be valid semver"
    );
}

#[test]
fn version_is_at_least_0_4_0() {
    // The bug report said 0.4.0 is the current release; pin a lower
    // bound so accidental downgrades in Cargo.toml are caught.
    let current = SemVer::parse(VERSION).expect("version must parse");
    let baseline = SemVer::parse("0.4.0").unwrap();
    assert!(
        current >= baseline,
        "VERSION {VERSION} is below the 0.4.0 baseline"
    );
}
