//! Read-only diff generation via git2.
//!
//! This module only inspects the repository — it never mutates anything.
//! All output is bounded to prevent unbounded memory usage.

use git2::{DiffOptions, Repository};

/// Maximum number of lines to keep in a diff result.
const MAX_DIFF_LINES: usize = 500;

/// The kind of diff being displayed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffKind {
    Modified,
    NewFile,
    Deleted,
    Renamed,
    Binary,
    Empty,
    Error(String),
}

/// A single line from a diff, with its origin marker.
#[derive(Debug, Clone)]
pub struct DiffLine {
    /// Origin character: '+', '-', ' ', or 'H' for hunk headers.
    pub origin: char,
    /// The text content of this line.
    pub content: String,
}

/// Result of diffing a single file.
#[derive(Debug, Clone)]
pub struct DiffResult {
    pub file_path: String,
    pub kind: DiffKind,
    pub lines: Vec<DiffLine>,
    pub truncated: bool,
}

/// Get a read-only diff for a single working-directory file.
///
/// For modified files this shows the unstaged changes (workdir vs index).
/// For untracked (new) files it reads the file content directly.
/// Binary files, deleted files, and errors are handled gracefully.
pub fn get_file_diff(repo_path: &str, file_path: &str) -> DiffResult {
    let repo = match Repository::discover(repo_path) {
        Ok(r) => r,
        Err(e) => {
            return DiffResult {
                file_path: file_path.to_string(),
                kind: DiffKind::Error(format!("Not a git repo: {e}")),
                lines: vec![],
                truncated: false,
            };
        }
    };

    // Check if the file is untracked (new) — read it directly.
    if is_untracked(&repo, file_path) {
        return read_new_file(&repo, file_path);
    }

    // For tracked files, generate a diff against the index.
    diff_workdir_file(&repo, file_path)
}

fn is_untracked(repo: &Repository, file_path: &str) -> bool {
    if let Ok(statuses) = repo.statuses(None) {
        for entry in statuses.iter() {
            if entry.path() == Some(file_path) {
                return entry.status().contains(git2::Status::WT_NEW);
            }
        }
    }
    false
}

fn read_new_file(repo: &Repository, file_path: &str) -> DiffResult {
    let workdir = match repo.workdir() {
        Some(w) => w,
        None => {
            return DiffResult {
                file_path: file_path.to_string(),
                kind: DiffKind::Error("Bare repository".to_string()),
                lines: vec![],
                truncated: false,
            };
        }
    };

    let full_path = workdir.join(file_path);
    let content = match std::fs::read(&full_path) {
        Ok(bytes) => bytes,
        Err(e) => {
            return DiffResult {
                file_path: file_path.to_string(),
                kind: DiffKind::Error(format!("Cannot read file: {e}")),
                lines: vec![],
                truncated: false,
            };
        }
    };

    // Check for binary content.
    if content.iter().take(8192).any(|&b| b == 0) {
        return DiffResult {
            file_path: file_path.to_string(),
            kind: DiffKind::Binary,
            lines: vec![],
            truncated: false,
        };
    }

    let text = String::from_utf8_lossy(&content);
    let mut lines: Vec<DiffLine> = Vec::new();
    let mut truncated = false;

    lines.push(DiffLine {
        origin: 'H',
        content: format!("(new file — {} lines)", text.lines().count()),
    });

    for line in text.lines() {
        if lines.len() >= MAX_DIFF_LINES {
            truncated = true;
            break;
        }
        lines.push(DiffLine {
            origin: '+',
            content: line.to_string(),
        });
    }

    DiffResult {
        file_path: file_path.to_string(),
        kind: DiffKind::NewFile,
        lines,
        truncated,
    }
}

/// Get a read-only diff for a commit against its first parent.
///
/// Shows all file changes introduced by the commit. Used from the
/// commit details screen. Truncates at `MAX_DIFF_LINES` like file diffs.
pub fn get_commit_diff(repo_path: &str, hash: &str) -> DiffResult {
    let repo = match Repository::discover(repo_path) {
        Ok(r) => r,
        Err(e) => {
            return DiffResult {
                file_path: hash.to_string(),
                kind: DiffKind::Error(format!("Not a git repo: {e}")),
                lines: vec![],
                truncated: false,
            };
        }
    };

    let oid = match git2::Oid::from_str(hash) {
        Ok(o) => o,
        Err(e) => {
            return DiffResult {
                file_path: hash.to_string(),
                kind: DiffKind::Error(format!("Invalid hash: {e}")),
                lines: vec![],
                truncated: false,
            };
        }
    };

    let commit = match repo.find_commit(oid) {
        Ok(c) => c,
        Err(e) => {
            return DiffResult {
                file_path: hash.to_string(),
                kind: DiffKind::Error(format!("Commit not found: {e}")),
                lines: vec![],
                truncated: false,
            };
        }
    };

    let commit_tree = match commit.tree() {
        Ok(t) => t,
        Err(e) => {
            return DiffResult {
                file_path: hash.to_string(),
                kind: DiffKind::Error(format!("Could not read tree: {e}")),
                lines: vec![],
                truncated: false,
            };
        }
    };

    let parent_tree = commit.parent(0).ok().and_then(|p| p.tree().ok());

    let diff = match repo.diff_tree_to_tree(
        parent_tree.as_ref(),
        Some(&commit_tree),
        None,
    ) {
        Ok(d) => d,
        Err(e) => {
            return DiffResult {
                file_path: hash.to_string(),
                kind: DiffKind::Error(format!("Diff failed: {e}")),
                lines: vec![],
                truncated: false,
            };
        }
    };

    let mut lines: Vec<DiffLine> = Vec::new();
    let mut truncated = false;

    let _ = diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
        if lines.len() >= MAX_DIFF_LINES {
            truncated = true;
            return true;
        }
        let origin = match line.origin() {
            '+' | '>' => '+',
            '-' | '<' => '-',
            'H' | 'F' => 'H',
            _ => ' ',
        };
        let content = String::from_utf8_lossy(line.content())
            .trim_end()
            .to_string();
        lines.push(DiffLine { origin, content });
        true
    });

    let short = &hash[..7.min(hash.len())];
    DiffResult {
        file_path: format!("commit {short}"),
        kind: DiffKind::Modified,
        lines,
        truncated,
    }
}

fn diff_workdir_file(repo: &Repository, file_path: &str) -> DiffResult {
    let mut opts = DiffOptions::new();
    opts.pathspec(file_path);

    let diff = match repo.diff_index_to_workdir(None, Some(&mut opts)) {
        Ok(d) => d,
        Err(e) => {
            return DiffResult {
                file_path: file_path.to_string(),
                kind: DiffKind::Error(format!("Diff failed: {e}")),
                lines: vec![],
                truncated: false,
            };
        }
    };

    // Detect the change kind from the first delta.
    let mut kind = DiffKind::Empty;
    let mut is_binary = false;
    for delta_idx in 0..diff.deltas().len() {
        let delta = diff.get_delta(delta_idx).unwrap();
        use git2::Delta;
        kind = match delta.status() {
            Delta::Modified => DiffKind::Modified,
            Delta::Added => DiffKind::NewFile,
            Delta::Deleted => DiffKind::Deleted,
            Delta::Renamed => DiffKind::Renamed,
            _ => DiffKind::Modified,
        };
        if delta.flags().is_binary() {
            is_binary = true;
        }
    }

    if is_binary {
        return DiffResult {
            file_path: file_path.to_string(),
            kind: DiffKind::Binary,
            lines: vec![],
            truncated: false,
        };
    }

    // Use Diff::print which takes a single closure.
    let mut lines: Vec<DiffLine> = Vec::new();
    let mut truncated = false;

    let _ = diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
        if lines.len() >= MAX_DIFF_LINES {
            truncated = true;
            return true;
        }
        let origin = match line.origin() {
            '+' | '>' => '+',
            '-' | '<' => '-',
            'H' | 'F' => 'H',
            _ => ' ',
        };
        let content = String::from_utf8_lossy(line.content())
            .trim_end()
            .to_string();
        lines.push(DiffLine { origin, content });
        true
    });

    DiffResult {
        file_path: file_path.to_string(),
        kind,
        lines,
        truncated,
    }
}
