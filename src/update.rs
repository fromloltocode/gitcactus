//! Update-awareness module.
//!
//! `VERSION` always reflects this build's `Cargo.toml` (via
//! `env!("CARGO_PKG_VERSION")`). `check_for_updates()` queries the
//! GitCactus repository for version tags and returns whichever is
//! highest in semver order.
//!
//! # Implementation notes
//!
//! - Lookup uses `git ls-remote --tags <URL>` and parses tag names
//!   matching `vX.Y.Z`. We already depend on the system `git` for
//!   rebase, so no extra crate is needed for HTTP or JSON.
//! - Ordering is proper numeric semver (so `v0.10.0 > v0.2.0`).
//! - Network or git failures never panic — they fall back to the
//!   current version with an explanatory `error` message the screen
//!   can render.

use std::cmp::Ordering;
use std::process::{Command, Stdio};
use std::time::Duration;

/// Current version of GitCactus, kept in sync with `Cargo.toml`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Upstream repository whose tags we consult for the latest version.
const RELEASES_URL: &str = "https://github.com/fromloltocode/gitcactus.git";

/// Maximum time to wait for `git ls-remote` before giving up.
const LS_REMOTE_TIMEOUT: Duration = Duration::from_secs(6);

/// Information returned by an update check.
pub struct UpdateInfo {
    pub current: &'static str,
    /// Latest version reported by the remote, formatted as `X.Y.Z`.
    /// If the check failed, this falls back to `current` so the UI
    /// never shows "you're behind" when we can't actually tell.
    pub latest: String,
    pub is_up_to_date: bool,
    pub changelog: Vec<ChangelogEntry>,
    /// Non-fatal error from the last check, if any. When set, the
    /// screen should tell the user the check failed rather than
    /// presenting `latest` as authoritative.
    pub error: Option<String>,
}

/// A single entry in the changelog shown on the update screen.
pub struct ChangelogEntry {
    pub version: String,
    pub summary: String,
}

/// Hard-coded release notes for past versions, newest-first.
///
/// Kept minimal and local — the update screen doesn't need to load
/// these from the network. New releases should add an entry at the top.
fn canned_changelog() -> Vec<ChangelogEntry> {
    vec![
        ChangelogEntry {
            version: "0.4.0".into(),
            summary: "Rebase Portal execution, continue/abort, mouse input, remote sync scaffolding".into(),
        },
        ChangelogEntry {
            version: "0.2.0".into(),
            summary: "Theme system, local progression / skill tree, XP feedback banner".into(),
        },
        ChangelogEntry {
            version: "0.1.1".into(),
            summary: "Pixel-art cactus mascot with shimmer animation".into(),
        },
        ChangelogEntry {
            version: "0.1.0".into(),
            summary: "Initial release \u{2014} TUI skeleton, status screen, cactus mascot".into(),
        },
    ]
}

/// Check for updates by querying the upstream repo for tags.
///
/// Returns whichever is highest of (`current`, all parsable tags). On
/// any error — git not on PATH, offline, malformed output, timeout —
/// the result falls back to treating `current` as the latest known
/// version and records the reason in `error`.
pub fn check_for_updates() -> UpdateInfo {
    check_for_updates_with(|| fetch_remote_tags(RELEASES_URL))
}

/// Injection point for tests: computes a result from a tag-fetcher
/// closure. Production code always uses [`check_for_updates`].
pub fn check_for_updates_with<F>(fetch: F) -> UpdateInfo
where
    F: FnOnce() -> Result<Vec<String>, String>,
{
    let current = VERSION;
    let current_ver = SemVer::parse(current);

    let (latest_str, error) = match fetch() {
        Ok(tags) => {
            let max_tag = tags
                .iter()
                .filter_map(|t| SemVer::parse(t).map(|v| (v, t.clone())))
                .max_by(|a, b| a.0.cmp(&b.0));
            match max_tag {
                Some((_, tag)) => (tag, None),
                None => (
                    current.to_string(),
                    Some("No version tags found upstream.".into()),
                ),
            }
        }
        Err(e) => (current.to_string(), Some(e)),
    };

    // Compare as semver if both are parseable; otherwise, safe fallback
    // is "we are up to date" so we never falsely alarm the user.
    let is_up_to_date = match (current_ver.as_ref(), SemVer::parse(&latest_str).as_ref()) {
        (Some(c), Some(l)) => c.cmp(l) != Ordering::Less,
        _ => true,
    };

    UpdateInfo {
        current,
        latest: strip_v_prefix(&latest_str).to_string(),
        is_up_to_date,
        changelog: canned_changelog(),
        error,
    }
}

/// Strict `vX.Y.Z` / `X.Y.Z` semver triple with numeric ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SemVer {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

impl SemVer {
    /// Parse a tag or version string. Accepts an optional leading `v`.
    /// Ignores pre-release suffixes (everything after `-` or `+`) by
    /// taking just the numeric core — adequate for our tag flow.
    pub fn parse(s: &str) -> Option<Self> {
        let trimmed = s.trim();
        let core = trimmed.strip_prefix('v').unwrap_or(trimmed);
        // Drop pre-release / build metadata so "0.4.0-rc1" still parses
        // to (0, 4, 0) and orders below "0.4.0" via the normal numeric
        // compare. Simple and predictable.
        let core = core
            .split(['-', '+'])
            .next()
            .unwrap_or(core);
        let mut parts = core.split('.');
        let major: u64 = parts.next()?.parse().ok()?;
        let minor: u64 = parts.next()?.parse().ok()?;
        let patch: u64 = parts.next()?.parse().ok()?;
        if parts.next().is_some() {
            return None; // too many segments
        }
        Some(SemVer { major, minor, patch })
    }
}

impl PartialOrd for SemVer {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SemVer {
    fn cmp(&self, other: &Self) -> Ordering {
        self.major
            .cmp(&other.major)
            .then(self.minor.cmp(&other.minor))
            .then(self.patch.cmp(&other.patch))
    }
}

/// Call `git ls-remote --tags <url>` and return bare tag names.
///
/// Output lines look like:
///   `<sha>\trefs/tags/v0.4.0`
///   `<sha>\trefs/tags/v0.4.0^{}`
/// We keep only tags that look like `vX.Y.Z`, ignoring `^{}`
/// peeled-tag suffixes git emits.
fn fetch_remote_tags(url: &str) -> Result<Vec<String>, String> {
    // Use a child process so we can enforce a timeout. `Command::output`
    // blocks without one, which is a bad UX if the remote is slow.
    let mut child = Command::new("git")
        .arg("ls-remote")
        .arg("--tags")
        .arg("--refs")
        .arg(url)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Could not run git: {e}"))?;

    // Poll until the child exits or the timeout elapses.
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {
                if start.elapsed() >= LS_REMOTE_TIMEOUT {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err("Update check timed out.".into());
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(e) => {
                let _ = child.kill();
                return Err(format!("git ls-remote failed: {e}"));
            }
        }
    }

    let output = child
        .wait_with_output()
        .map_err(|e| format!("git ls-remote failed: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            "git ls-remote exited with an error.".into()
        } else {
            format!("git ls-remote: {stderr}")
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_ls_remote_output(&stdout))
}

/// Extract plain tag names from `git ls-remote --tags` output.
pub fn parse_ls_remote_output(stdout: &str) -> Vec<String> {
    stdout
        .lines()
        .filter_map(|line| {
            // <sha>\trefs/tags/<name>
            let tail = line.split('\t').nth(1)?;
            let name = tail.strip_prefix("refs/tags/")?;
            // Peeled-tag suffix git sometimes emits — we pass --refs
            // which strips these, but belt-and-braces here.
            let name = name.strip_suffix("^{}").unwrap_or(name);
            Some(name.to_string())
        })
        .collect()
}

fn strip_v_prefix(s: &str) -> &str {
    s.strip_prefix('v').unwrap_or(s)
}
