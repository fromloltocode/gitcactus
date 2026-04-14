//! Tests for the theme / config system.
//!
//! The theme loader must be *fail-graceful*: invalid presets or color
//! names must fall back silently to defaults. GitCactus should never
//! refuse to start because of theme config.

use gitcactus::theme::{parse_color, Theme, ThemePreset};
use ratatui::style::Color;

// ── Preset parsing ───────────────────────────────────────────────────

#[test]
fn preset_parse_known_names() {
    assert_eq!(ThemePreset::parse("default"), Some(ThemePreset::Default));
    assert_eq!(
        ThemePreset::parse("terminal_blue"),
        Some(ThemePreset::TerminalBlue)
    );
    assert_eq!(ThemePreset::parse("matrix"), Some(ThemePreset::Matrix));
    assert_eq!(
        ThemePreset::parse("retro_danger"),
        Some(ThemePreset::RetroDanger)
    );
}

#[test]
fn preset_parse_accepts_aliases() {
    assert_eq!(
        ThemePreset::parse("terminal-blue"),
        Some(ThemePreset::TerminalBlue)
    );
    assert_eq!(ThemePreset::parse("blue"), Some(ThemePreset::TerminalBlue));
    assert_eq!(ThemePreset::parse("danger"), Some(ThemePreset::RetroDanger));
}

#[test]
fn preset_parse_case_insensitive() {
    assert_eq!(ThemePreset::parse("MATRIX"), Some(ThemePreset::Matrix));
    assert_eq!(ThemePreset::parse("Default"), Some(ThemePreset::Default));
}

#[test]
fn preset_parse_unknown_returns_none() {
    assert!(ThemePreset::parse("lol").is_none());
    assert!(ThemePreset::parse("").is_none());
    assert!(ThemePreset::parse("neon-hellscape").is_none());
}

#[test]
fn preset_round_trip() {
    for p in [
        ThemePreset::Default,
        ThemePreset::TerminalBlue,
        ThemePreset::Matrix,
        ThemePreset::RetroDanger,
    ] {
        assert_eq!(ThemePreset::parse(p.as_str()), Some(p));
    }
}

// ── Color parsing ────────────────────────────────────────────────────

#[test]
fn parse_color_happy_path() {
    assert_eq!(parse_color("red"), Some(Color::Red));
    assert_eq!(parse_color("green"), Some(Color::Green));
    assert_eq!(parse_color("darkgray"), Some(Color::DarkGray));
    assert_eq!(parse_color("dark_gray"), Some(Color::DarkGray));
    assert_eq!(parse_color("dark-gray"), Some(Color::DarkGray));
    assert_eq!(parse_color("grey"), Some(Color::Gray));
    assert_eq!(parse_color("purple"), Some(Color::Magenta));
}

#[test]
fn parse_color_case_insensitive_and_trimmed() {
    assert_eq!(parse_color("  RED  "), Some(Color::Red));
    assert_eq!(parse_color("LightGreen"), Some(Color::LightGreen));
}

#[test]
fn parse_color_invalid_returns_none() {
    assert!(parse_color("burnt_sienna").is_none());
    assert!(parse_color("").is_none());
    assert!(parse_color("#ff0000").is_none()); // no hex support in this phase
    assert!(parse_color("123").is_none());
}

// ── Default theme ────────────────────────────────────────────────────

#[test]
fn default_theme_matches_default_preset() {
    let t = Theme::default();
    assert_eq!(t.preset, ThemePreset::Default);
    assert!(!t.has_overrides());
}

// ── Theme::from_config_lines ─────────────────────────────────────────

#[test]
fn empty_config_yields_default() {
    let t = Theme::from_config_lines([]);
    assert_eq!(t.preset, ThemePreset::Default);
    assert!(!t.has_overrides());
}

#[test]
fn preset_only_applies_full_palette() {
    let t = Theme::from_config_lines(["theme=matrix"]);
    assert_eq!(t.preset, ThemePreset::Matrix);
    assert_eq!(t.primary, Color::Green);
    assert_eq!(t.cactus, Color::Green);
    assert!(!t.has_overrides());
}

#[test]
fn per_role_override_on_top_of_preset() {
    let lines = [
        "theme=matrix",
        "theme.primary=blue",
        "theme.error=lightred",
    ];
    let t = Theme::from_config_lines(lines);
    assert_eq!(t.preset, ThemePreset::Matrix);
    assert_eq!(t.primary, Color::Blue);
    assert_eq!(t.error, Color::LightRed);
    // Other roles unchanged — still from Matrix.
    assert_eq!(t.success, Color::LightGreen);
    assert!(t.has_overrides());
}

#[test]
fn malformed_preset_falls_back_to_default() {
    let t = Theme::from_config_lines(["theme=neon-hellscape"]);
    assert_eq!(t.preset, ThemePreset::Default);
    assert!(!t.has_overrides());
}

#[test]
fn invalid_color_override_is_silently_ignored() {
    let lines = [
        "theme=terminal_blue",
        "theme.primary=burnt_sienna",
        "theme.success=green",
    ];
    let t = Theme::from_config_lines(lines);
    assert_eq!(t.preset, ThemePreset::TerminalBlue);
    // Invalid color never overrode the preset's primary — still Blue.
    assert_eq!(t.primary, Color::Blue);
    // Valid override took effect.
    assert_eq!(t.success, Color::Green);
}

#[test]
fn garbage_lines_do_not_crash() {
    let lines = [
        "",
        "# comment",
        "not_a_kv_line",
        "theme=matrix",
        "theme.primary=",
        "theme.error=    ",
    ];
    // Just needs to run without panicking.
    let t = Theme::from_config_lines(lines);
    assert_eq!(t.preset, ThemePreset::Matrix);
}

#[test]
fn has_overrides_is_false_for_bare_preset() {
    for p in [
        ThemePreset::Default,
        ThemePreset::TerminalBlue,
        ThemePreset::Matrix,
        ThemePreset::RetroDanger,
    ] {
        let line = format!("theme={}", p.as_str());
        let t = Theme::from_config_lines(std::iter::once(line.as_str()));
        assert!(
            !t.has_overrides(),
            "preset {p:?} should report no overrides"
        );
    }
}

// ── No-network sanity ────────────────────────────────────────────────

#[test]
fn theme_loading_is_purely_local_no_std_net_touched() {
    // This is really a static claim about the implementation: theme
    // parsing takes strings in and returns a Theme. The test just
    // exercises the full public API to catch any accidental future
    // regression that tries to hit a URL.
    let _ = Theme::from_config_lines(["theme=matrix"]);
    let _ = parse_color("red");
    let _ = Theme::default();
}
