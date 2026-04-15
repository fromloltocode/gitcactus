//! Sensible default scan-root candidates per platform.
//!
//! This module only ever *proposes* directories. It never scans, and
//! nothing here gets added to the user's approved list automatically —
//! the Scan Locations screen uses these as a starting point for the
//! user to explicitly approve (one Enter-press each).
//!
//! A candidate is filtered out unless it already exists on disk, so
//! we never suggest a folder the user hasn't created yet.

use std::path::PathBuf;

/// Return the subset of platform-default directories that currently
/// exist on disk. Always a fresh call — the result can change across
/// sessions if the user creates or removes `~/Projects` etc.
pub fn default_scan_root_candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();
    for name in candidate_names() {
        if let Some(full) = resolve(name) {
            if full.is_dir() {
                out.push(full);
            }
        }
    }
    // Dedup by path in case two names resolve to the same directory
    // (unlikely but cheap insurance).
    let mut seen = Vec::new();
    out.retain(|p| {
        if seen.iter().any(|s: &PathBuf| s == p) {
            false
        } else {
            seen.push(p.clone());
            true
        }
    });
    out
}

/// Platform-specific lists of directory names to try under the user's
/// home. Ordered roughly by "most common" first so the initial UI is
/// familiar on each platform.
fn candidate_names() -> &'static [&'static str] {
    #[cfg(windows)]
    {
        &[
            "Projects",
            r"source\repos", // Visual Studio default
            "Code",
            r"Documents\GitHub",
            "Developer",
            "src",
        ]
    }
    #[cfg(not(windows))]
    {
        &[
            "Projects",
            "Code",
            "Developer",
            "Documents/GitHub",
            "src",
            "dev",
            "work",
        ]
    }
}

/// Join `name` onto the platform's home directory, respecting
/// `$HOME` on Unix and `%USERPROFILE%` on Windows.
fn resolve(name: &str) -> Option<PathBuf> {
    let home = home_dir()?;
    Some(home.join(name))
}

fn home_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var("USERPROFILE")
            .ok()
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
    }
    #[cfg(not(windows))]
    {
        std::env::var("HOME")
            .ok()
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_only_existing_directories() {
        // We can't guarantee any specific candidate exists on the
        // machine running this test, so the most we can assert is
        // that every returned path does exist and is a directory.
        for p in default_scan_root_candidates() {
            assert!(p.is_dir(), "{p:?} should exist and be a directory");
        }
    }

    #[test]
    fn candidates_list_is_non_empty_per_platform() {
        assert!(!candidate_names().is_empty());
    }
}
