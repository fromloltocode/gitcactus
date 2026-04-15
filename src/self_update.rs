//! Install-method detection + honest self-update plan.
//!
//! This module performs **only two kinds of work**:
//!
//! 1. Look at `std::env::current_exe()` and classify how GitCactus was
//!    installed. No mutation, no network, no shelling out.
//! 2. Describe, as a plain data value ([`UpdatePlan`]), what GitCactus
//!    would do if the user confirmed an update. Execution happens in
//!    `main.rs` where we control TUI teardown/restore; this module
//!    never runs the update itself.
//!
//! The detection is deliberately conservative. If we aren't confident,
//! we return [`InstallKind::Unknown`] so the UI can be honest about
//! not knowing instead of pretending and running the wrong command.

use std::path::{Path, PathBuf};

/// How GitCactus was likely installed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallKind {
    /// Homebrew-managed. `prefix` is the brew prefix we detected
    /// (e.g. `/opt/homebrew` or `/usr/local`).
    Homebrew { prefix: PathBuf },
    /// `cargo install gitcactus` into the user's Cargo bin dir.
    Cargo,
    /// A raw binary the user placed somewhere manually.
    ManualBinary,
    /// Running from a local debug/release build (`target/…`).
    FromSource,
    /// We could not classify the binary path with any confidence.
    Unknown,
}

impl InstallKind {
    /// Whether GitCactus can safely attempt an in-app update for this
    /// install kind. Currently only Homebrew — the task asks us to
    /// start there and add more later.
    #[allow(dead_code)]
    pub fn can_self_update(&self) -> bool {
        matches!(self, InstallKind::Homebrew { .. })
    }

    /// A short human-readable label for the UI ("Homebrew",
    /// "cargo install", etc.).
    pub fn label(&self) -> &'static str {
        match self {
            InstallKind::Homebrew { .. } => "Homebrew",
            InstallKind::Cargo => "cargo install",
            InstallKind::ManualBinary => "manual binary",
            InstallKind::FromSource => "local source build",
            InstallKind::Unknown => "unknown",
        }
    }

    /// Honest fallback instructions for install kinds that can't
    /// self-update. Each entry is a line the UI can render verbatim.
    pub fn manual_instructions(&self) -> Vec<String> {
        match self {
            InstallKind::Homebrew { .. } => vec![
                // Still provided as a fallback if the automated path fails.
                "brew upgrade fromloltocode/tap/gitcactus".into(),
            ],
            InstallKind::Cargo => vec![
                "cargo install --locked gitcactus".into(),
                "  (or: cargo install --path . from a cloned repo)".into(),
            ],
            InstallKind::ManualBinary => vec![
                "Download the new release from:".into(),
                "  https://github.com/fromloltocode/gitcactus/releases".into(),
                "Replace your existing gitcactus binary with the new one.".into(),
            ],
            InstallKind::FromSource => vec![
                "git pull && cargo build --release".into(),
                "  (then re-run ./target/release/gitcactus)".into(),
            ],
            InstallKind::Unknown => vec![
                "GitCactus couldn't tell how it was installed.".into(),
                "See https://github.com/fromloltocode/gitcactus#install".into(),
            ],
        }
    }
}

/// Concrete command GitCactus would run for a self-update, as a
/// `(program, args)` pair. Exposed so the confirmation dialog can
/// show the exact command before anything runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateCommand {
    pub program: String,
    pub args: Vec<String>,
}

impl UpdateCommand {
    /// Render the command the way the user would type it in a shell.
    pub fn render(&self) -> String {
        let mut out = self.program.clone();
        for a in &self.args {
            out.push(' ');
            out.push_str(a);
        }
        out
    }
}

/// What the update screen will render and, if confirmed, what the
/// runner will execute. `command` is `None` when self-update isn't
/// supported for this install kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdatePlan {
    pub kind: InstallKind,
    pub command: Option<UpdateCommand>,
    /// Whether the update requires the user to relaunch `gitcactus`
    /// after a successful upgrade. For Homebrew this is true because
    /// the running binary has been replaced on disk.
    pub requires_relaunch: bool,
}

impl UpdatePlan {
    pub fn for_kind(kind: InstallKind) -> Self {
        match &kind {
            InstallKind::Homebrew { .. } => UpdatePlan {
                command: Some(UpdateCommand {
                    program: "brew".into(),
                    args: vec![
                        "upgrade".into(),
                        "fromloltocode/tap/gitcactus".into(),
                    ],
                }),
                requires_relaunch: true,
                kind,
            },
            _ => UpdatePlan {
                command: None,
                requires_relaunch: false,
                kind,
            },
        }
    }
}

/// Classify the currently-running binary. Safe to call from anywhere —
/// it only reads `std::env::current_exe()`.
pub fn detect_install() -> InstallKind {
    match std::env::current_exe() {
        Ok(path) => classify(&path),
        Err(_) => InstallKind::Unknown,
    }
}

/// Pure classifier exposed for tests.
pub fn classify(exe: &Path) -> InstallKind {
    // Canonicalise when possible so `/usr/local/bin/gitcactus` (a
    // symlink into the Cellar) resolves to its real location. Fall
    // back to the raw path if canonicalisation fails.
    let resolved = std::fs::canonicalize(exe).unwrap_or_else(|_| exe.to_path_buf());
    classify_path(&resolved, exe)
}

fn classify_path(resolved: &Path, raw: &Path) -> InstallKind {
    let resolved_str = resolved.to_string_lossy();
    let raw_str = raw.to_string_lossy();

    // Homebrew lives at one of a few well-known prefixes.
    for prefix in [
        "/opt/homebrew",              // macOS arm64 default
        "/usr/local",                  // macOS x86_64 default + Linux
        "/home/linuxbrew/.linuxbrew",  // Linuxbrew default
    ] {
        if resolved_str.starts_with(&format!("{prefix}/"))
            || raw_str.starts_with(&format!("{prefix}/"))
        {
            return InstallKind::Homebrew {
                prefix: PathBuf::from(prefix),
            };
        }
    }

    // Cargo installs land in $CARGO_HOME/bin (default ~/.cargo/bin).
    if let Ok(cargo_home) = std::env::var("CARGO_HOME") {
        let cargo_bin = format!("{cargo_home}/bin/");
        if resolved_str.starts_with(&cargo_bin) || raw_str.starts_with(&cargo_bin) {
            return InstallKind::Cargo;
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        let cargo_bin = format!("{home}/.cargo/bin/");
        if resolved_str.starts_with(&cargo_bin) || raw_str.starts_with(&cargo_bin) {
            return InstallKind::Cargo;
        }
    }

    // Local source builds produce `…/target/debug/` or `…/target/release/`.
    if resolved_str.contains("/target/debug/") || resolved_str.contains("/target/release/")
        || raw_str.contains("/target/debug/") || raw_str.contains("/target/release/")
    {
        return InstallKind::FromSource;
    }

    // Anything else: presume the user placed a binary somewhere themselves.
    InstallKind::ManualBinary
}

#[cfg(test)]
mod tests {
    use super::*;

    fn classify_raw(p: &str) -> InstallKind {
        classify_path(Path::new(p), Path::new(p))
    }

    #[test]
    fn classifies_homebrew_arm64_prefix() {
        match classify_raw("/opt/homebrew/bin/gitcactus") {
            InstallKind::Homebrew { prefix } => {
                assert_eq!(prefix, PathBuf::from("/opt/homebrew"));
            }
            other => panic!("expected Homebrew, got {other:?}"),
        }
    }

    #[test]
    fn classifies_homebrew_x86_prefix() {
        match classify_raw("/usr/local/bin/gitcactus") {
            InstallKind::Homebrew { prefix } => {
                assert_eq!(prefix, PathBuf::from("/usr/local"));
            }
            other => panic!("expected Homebrew, got {other:?}"),
        }
    }

    #[test]
    fn classifies_homebrew_linuxbrew_prefix() {
        assert!(matches!(
            classify_raw("/home/linuxbrew/.linuxbrew/bin/gitcactus"),
            InstallKind::Homebrew { .. }
        ));
    }

    #[test]
    fn classifies_from_source_debug_build() {
        assert_eq!(
            classify_raw("/home/u/code/gitcactus/target/debug/gitcactus"),
            InstallKind::FromSource
        );
    }

    #[test]
    fn classifies_from_source_release_build() {
        assert_eq!(
            classify_raw("/home/u/code/gitcactus/target/release/gitcactus"),
            InstallKind::FromSource
        );
    }

    #[test]
    fn classifies_manual_binary_fallback() {
        assert_eq!(
            classify_raw("/opt/tools/gitcactus"),
            InstallKind::ManualBinary
        );
    }

    #[test]
    fn homebrew_plan_has_upgrade_command_and_requires_relaunch() {
        let plan = UpdatePlan::for_kind(InstallKind::Homebrew {
            prefix: PathBuf::from("/opt/homebrew"),
        });
        let cmd = plan.command.as_ref().expect("homebrew has a command");
        assert_eq!(cmd.program, "brew");
        assert_eq!(cmd.args, vec!["upgrade", "fromloltocode/tap/gitcactus"]);
        assert!(plan.requires_relaunch);
    }

    #[test]
    fn non_homebrew_plans_have_no_command() {
        for kind in [
            InstallKind::Cargo,
            InstallKind::ManualBinary,
            InstallKind::FromSource,
            InstallKind::Unknown,
        ] {
            let plan = UpdatePlan::for_kind(kind.clone());
            assert!(plan.command.is_none(), "unexpected command for {kind:?}");
            assert!(!plan.requires_relaunch);
        }
    }

    #[test]
    fn manual_instructions_non_empty_for_every_kind() {
        for kind in [
            InstallKind::Homebrew {
                prefix: PathBuf::from("/opt/homebrew"),
            },
            InstallKind::Cargo,
            InstallKind::ManualBinary,
            InstallKind::FromSource,
            InstallKind::Unknown,
        ] {
            assert!(
                !kind.manual_instructions().is_empty(),
                "missing instructions for {kind:?}"
            );
        }
    }

    #[test]
    fn render_reconstructs_shell_form() {
        let cmd = UpdateCommand {
            program: "brew".into(),
            args: vec!["upgrade".into(), "fromloltocode/tap/gitcactus".into()],
        };
        assert_eq!(cmd.render(), "brew upgrade fromloltocode/tap/gitcactus");
    }
}
