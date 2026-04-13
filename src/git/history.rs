//! Read-only commit history retrieval via git2.
//!
//! Walks the commit graph from HEAD and collects summary information.
//! Never mutates the repository.

use git2::Repository;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Maximum number of commits to load.
const MAX_COMMITS: usize = 50;

/// Summary of a single commit.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct HistoryEntry {
    /// Abbreviated commit hash (7 characters).
    pub short_hash: String,
    /// Full commit hash.
    pub full_hash: String,
    /// First line of the commit message.
    pub summary: String,
    /// Author name.
    pub author: String,
    /// Seconds since epoch.
    pub timestamp: i64,
    /// Human-readable relative time (e.g. "3 hours ago").
    pub relative_time: String,
}

/// Result of loading commit history.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct HistoryResult {
    pub entries: Vec<HistoryEntry>,
    pub is_real: bool,
    pub error: Option<String>,
}

/// Load commit history from the repository at `path`.
pub fn load_history(path: &str) -> HistoryResult {
    let repo = match Repository::discover(path) {
        Ok(r) => r,
        Err(_) => {
            return HistoryResult {
                entries: vec![],
                is_real: false,
                error: Some("Not a git repository.".into()),
            };
        }
    };

    let head = match repo.head() {
        Ok(h) => h,
        Err(_) => {
            return HistoryResult {
                entries: vec![],
                is_real: true,
                error: Some("No commits yet.".into()),
            };
        }
    };

    let oid = match head.target() {
        Some(o) => o,
        None => {
            return HistoryResult {
                entries: vec![],
                is_real: true,
                error: Some("Could not resolve HEAD.".into()),
            };
        }
    };

    let mut revwalk = match repo.revwalk() {
        Ok(r) => r,
        Err(e) => {
            return HistoryResult {
                entries: vec![],
                is_real: true,
                error: Some(format!("Revwalk failed: {e}")),
            };
        }
    };

    if revwalk.push(oid).is_err() {
        return HistoryResult {
            entries: vec![],
            is_real: true,
            error: Some("Could not start walking from HEAD.".into()),
        };
    }

    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs() as i64;

    let mut entries = Vec::new();

    for oid_result in revwalk {
        if entries.len() >= MAX_COMMITS {
            break;
        }
        let oid = match oid_result {
            Ok(o) => o,
            Err(_) => continue,
        };
        let commit = match repo.find_commit(oid) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let hash_str = oid.to_string();
        let short_hash = hash_str[..7.min(hash_str.len())].to_string();
        let summary = commit
            .summary()
            .unwrap_or("(no message)")
            .to_string();
        let author = commit.author().name().unwrap_or("Unknown").to_string();
        let timestamp = commit.time().seconds();
        let relative_time = format_relative(now_secs, timestamp);

        entries.push(HistoryEntry {
            short_hash,
            full_hash: hash_str,
            summary,
            author,
            timestamp,
            relative_time,
        });
    }

    HistoryResult {
        entries,
        is_real: true,
        error: None,
    }
}

fn format_relative(now: i64, then: i64) -> String {
    let diff = (now - then).max(0) as u64;
    if diff < 60 {
        "just now".into()
    } else if diff < 3600 {
        let m = diff / 60;
        format!("{m} min ago")
    } else if diff < 86400 {
        let h = diff / 3600;
        format!("{h} hour{} ago", if h == 1 { "" } else { "s" })
    } else if diff < 604800 {
        let d = diff / 86400;
        format!("{d} day{} ago", if d == 1 { "" } else { "s" })
    } else if diff < 2592000 {
        let w = diff / 604800;
        format!("{w} week{} ago", if w == 1 { "" } else { "s" })
    } else if diff < 31536000 {
        let m = diff / 2592000;
        format!("{m} month{} ago", if m == 1 { "" } else { "s" })
    } else {
        let y = diff / 31536000;
        format!("{y} year{} ago", if y == 1 { "" } else { "s" })
    }
}
