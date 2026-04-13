//! Read-only commit details loader.
//!
//! Loads the full metadata and file-change list for a single commit.
//! Never mutates the repository.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use git2::{Delta, DiffOptions, Repository};

/// Maximum number of files to list per commit.
const MAX_FILES: usize = 50;

/// The kind of change for a single file in a commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChangeKind {
    Added,
    Modified,
    Deleted,
    Renamed,
    Other,
}

/// A single file-level change in a commit.
#[derive(Debug, Clone)]
pub struct FileChange {
    pub path: String,
    pub kind: ChangeKind,
}

/// Full details for a single commit.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CommitDetails {
    pub short_hash: String,
    pub full_hash: String,
    pub author: String,
    pub email: String,
    pub timestamp: i64,
    pub relative_time: String,
    /// Full commit message (may span multiple lines).
    pub message: String,
    pub files: Vec<FileChange>,
    pub files_truncated: bool,
    pub error: Option<String>,
}

impl CommitDetails {
    /// Create an empty details struct (used for initial state).
    pub fn empty() -> Self {
        Self {
            short_hash: String::new(),
            full_hash: String::new(),
            author: String::new(),
            email: String::new(),
            timestamp: 0,
            relative_time: String::new(),
            message: String::new(),
            files: vec![],
            files_truncated: false,
            error: None,
        }
    }

    fn error(hash: &str, msg: String) -> Self {
        let mut d = Self::empty();
        d.full_hash = hash.to_string();
        d.error = Some(msg);
        d
    }
}

/// Load details for the commit identified by `hash`.
pub fn load_commit_details(path: &str, hash: &str) -> CommitDetails {
    let repo = match Repository::discover(path) {
        Ok(r) => r,
        Err(e) => return CommitDetails::error(hash, format!("Not a git repo: {e}")),
    };

    let oid = match git2::Oid::from_str(hash) {
        Ok(o) => o,
        Err(e) => return CommitDetails::error(hash, format!("Invalid hash: {e}")),
    };

    let commit = match repo.find_commit(oid) {
        Ok(c) => c,
        Err(e) => return CommitDetails::error(hash, format!("Commit not found: {e}")),
    };

    let full_hash = commit.id().to_string();
    let short_hash = full_hash[..7.min(full_hash.len())].to_string();
    let author_sig = commit.author();
    let author = author_sig.name().unwrap_or("Unknown").to_string();
    let email = author_sig.email().unwrap_or("").to_string();
    let timestamp = commit.time().seconds();
    let relative_time = format_relative_time(timestamp);
    let message = commit.message().unwrap_or("").trim().to_string();

    // Load file changes by diffing against the first parent (or empty for root).
    let (files, files_truncated) = load_file_changes(&repo, &commit);

    CommitDetails {
        short_hash,
        full_hash,
        author,
        email,
        timestamp,
        relative_time,
        message,
        files,
        files_truncated,
        error: None,
    }
}

fn load_file_changes(
    repo: &Repository,
    commit: &git2::Commit,
) -> (Vec<FileChange>, bool) {
    let commit_tree = match commit.tree() {
        Ok(t) => t,
        Err(_) => return (vec![], false),
    };

    // Diff against first parent (or no parent for root commit).
    let parent_tree = commit.parent(0).ok().and_then(|p| p.tree().ok());

    let mut opts = DiffOptions::new();
    let diff = match repo.diff_tree_to_tree(
        parent_tree.as_ref(),
        Some(&commit_tree),
        Some(&mut opts),
    ) {
        Ok(d) => d,
        Err(_) => return (vec![], false),
    };

    let mut files = Vec::new();
    let mut truncated = false;
    let delta_count = diff.deltas().len();

    for i in 0..delta_count {
        if files.len() >= MAX_FILES {
            truncated = true;
            break;
        }
        let delta = match diff.get_delta(i) {
            Some(d) => d,
            None => continue,
        };
        let kind = match delta.status() {
            Delta::Added => ChangeKind::Added,
            Delta::Modified => ChangeKind::Modified,
            Delta::Deleted => ChangeKind::Deleted,
            Delta::Renamed | Delta::Copied => ChangeKind::Renamed,
            _ => ChangeKind::Other,
        };
        let path = delta
            .new_file()
            .path()
            .or_else(|| delta.old_file().path())
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|| "(unknown)".into());
        files.push(FileChange { path, kind });
    }

    (files, truncated)
}

fn format_relative_time(then: i64) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs() as i64;
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
