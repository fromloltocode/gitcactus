//! Integration tests for the CLI flags (--help, --version).
//!
//! These invoke the compiled binary to verify user-facing output,
//! which is hard to test any other way. Cargo ensures the binary is
//! built before running integration tests.

use std::process::Command;

fn binary_path() -> std::path::PathBuf {
    // Cargo exposes the built binary path via CARGO_BIN_EXE_<name>.
    std::path::PathBuf::from(env!("CARGO_BIN_EXE_gitcactus"))
}

#[test]
fn version_flag_prints_name_and_semver() {
    let output = Command::new(binary_path())
        .arg("--version")
        .output()
        .expect("run gitcactus --version");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.starts_with("gitcactus "), "stdout was: {stdout:?}");
    // Should contain a semver-shaped version somewhere.
    assert!(
        stdout
            .split_whitespace()
            .next_back()
            .map(|v| v.split('.').count() >= 3)
            .unwrap_or(false),
        "expected semver in: {stdout:?}"
    );
}

#[test]
fn version_short_flag_works() {
    let output = Command::new(binary_path())
        .arg("-V")
        .output()
        .expect("run gitcactus -V");
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).starts_with("gitcactus "));
}

#[test]
fn help_flag_prints_usage_and_exits_successfully() {
    let output = Command::new(binary_path())
        .arg("--help")
        .output()
        .expect("run gitcactus --help");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    for needle in &[
        "GitCactus",
        "USAGE",
        "--help",
        "--version",
        "--intro",
        "--skip-intro",
        "EDITOR",
    ] {
        assert!(
            stdout.contains(needle),
            "help output missing {needle:?}; was:\n{stdout}"
        );
    }
}

#[test]
fn help_short_flag_works() {
    let output = Command::new(binary_path())
        .arg("-h")
        .output()
        .expect("run gitcactus -h");
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("USAGE"));
}
