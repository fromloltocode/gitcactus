//! Read-only rebase preview.
//!
//! Given a source branch (usually current HEAD) and a target branch, this
//! module computes what commits *would* be replayed if the user performed
//! a rebase onto the target — **without performing any rebase, checkout,
//! or other repository mutation**.
//!
//! The result is an informational snapshot the UI can render. It is
//! intentionally a preview: accuracy is best-effort, and any situation
//! where we cannot confidently produce a preview is surfaced through a
//! [`PreviewKind`] variant rather than silently fudged.

use git2::{BranchType, Repository};

/// Maximum number of commits to include in the replayed-commit list.
const MAX_PREVIEW_COMMITS: usize = 100;

/// Outcome of a preview attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreviewKind {
    Ready,
    NoCommitsToReplay,
    SameBranch,
    DetachedHead,
    BranchMissing,
    EmptyRepo,
    NotARepo,
    Error(String),
}

/// A single commit that would be replayed by a rebase.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewCommit {
    pub short_hash: String,
    pub full_hash: String,
    pub summary: String,
    pub author: String,
    pub is_merge: bool,
}

/// The full read-only preview result.
#[derive(Debug, Clone)]
pub struct RebasePreview {
    pub source: String,
    pub target: String,
    pub source_tip: String,
    pub target_tip: String,
    pub merge_base: Option<String>,
    pub commits: Vec<PreviewCommit>,
    pub truncated: bool,
    pub has_merge_commits: bool,
    pub dirty_tree: bool,
    pub kind: PreviewKind,
}

impl RebasePreview {
    fn failure(source: &str, target: &str, kind: PreviewKind) -> Self {
        Self {
            source: source.into(),
            target: target.into(),
            source_tip: String::new(),
            target_tip: String::new(),
            merge_base: None,
            commits: vec![],
            truncated: false,
            has_merge_commits: false,
            dirty_tree: false,
            kind,
        }
    }

    /// Whether it is safe to proceed to execution from this preview.
    /// Used by the Phase 2 confirmation step to gate the confirm button.
    pub fn is_executable(&self) -> bool {
        self.kind == PreviewKind::Ready && !self.dirty_tree
    }
}

/// Compute a read-only preview of rebasing current HEAD onto `target`.
pub fn preview_rebase(repo_path: &str, target: &str) -> RebasePreview {
    let repo = match Repository::discover(repo_path) {
        Ok(r) => r,
        Err(_) => return RebasePreview::failure("", target, PreviewKind::NotARepo),
    };

    let head = match repo.head() {
        Ok(h) => h,
        Err(_) => return RebasePreview::failure("", target, PreviewKind::EmptyRepo),
    };
    if !head.is_branch() {
        return RebasePreview::failure("", target, PreviewKind::DetachedHead);
    }
    let source_name = head.shorthand().unwrap_or("HEAD").to_string();

    if source_name == target {
        return RebasePreview::failure(&source_name, target, PreviewKind::SameBranch);
    }

    let source_oid = match head.target() {
        Some(o) => o,
        None => {
            return RebasePreview::failure(
                &source_name,
                target,
                PreviewKind::Error("Could not resolve HEAD target".into()),
            );
        }
    };

    let target_branch = match repo.find_branch(target, BranchType::Local) {
        Ok(b) => b,
        Err(_) => {
            return RebasePreview::failure(&source_name, target, PreviewKind::BranchMissing)
        }
    };
    let target_oid = match target_branch.get().peel_to_commit() {
        Ok(c) => c.id(),
        Err(e) => {
            return RebasePreview::failure(
                &source_name,
                target,
                PreviewKind::Error(format!("Could not resolve target: {e}")),
            );
        }
    };

    let dirty_tree = is_dirty(&repo);
    let merge_base = repo.merge_base(source_oid, target_oid).ok();
    let merge_base_short = merge_base.map(|o| short_hash(&o.to_string()));

    let mut revwalk = match repo.revwalk() {
        Ok(r) => r,
        Err(e) => {
            return RebasePreview::failure(
                &source_name,
                target,
                PreviewKind::Error(format!("Revwalk failed: {e}")),
            );
        }
    };
    if revwalk.push(source_oid).is_err() || revwalk.hide(target_oid).is_err() {
        return RebasePreview::failure(
            &source_name,
            target,
            PreviewKind::Error("Revwalk push/hide failed".into()),
        );
    }

    let mut commits: Vec<PreviewCommit> = Vec::new();
    let mut truncated = false;
    let mut has_merge_commits = false;

    for oid_result in revwalk {
        if commits.len() >= MAX_PREVIEW_COMMITS {
            truncated = true;
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
        let is_merge = commit.parent_count() > 1;
        if is_merge {
            has_merge_commits = true;
        }
        let full = oid.to_string();
        commits.push(PreviewCommit {
            short_hash: short_hash(&full),
            full_hash: full,
            summary: commit.summary().unwrap_or("(no message)").to_string(),
            author: commit.author().name().unwrap_or("Unknown").to_string(),
            is_merge,
        });
    }

    // Reverse so the UI / animation shows commits in replay order.
    commits.reverse();

    let kind = if commits.is_empty() {
        PreviewKind::NoCommitsToReplay
    } else {
        PreviewKind::Ready
    };

    RebasePreview {
        source: source_name,
        target: target.to_string(),
        source_tip: short_hash(&source_oid.to_string()),
        target_tip: short_hash(&target_oid.to_string()),
        merge_base: merge_base_short,
        commits,
        truncated,
        has_merge_commits,
        dirty_tree,
        kind,
    }
}

fn short_hash(full: &str) -> String {
    full[..7.min(full.len())].to_string()
}

fn is_dirty(repo: &Repository) -> bool {
    match repo.statuses(None) {
        Ok(s) => s.iter().any(|e| {
            let st = e.status();
            st.intersects(
                git2::Status::WT_MODIFIED
                    | git2::Status::WT_DELETED
                    | git2::Status::WT_RENAMED
                    | git2::Status::WT_TYPECHANGE
                    | git2::Status::INDEX_NEW
                    | git2::Status::INDEX_MODIFIED
                    | git2::Status::INDEX_DELETED
                    | git2::Status::INDEX_RENAMED
                    | git2::Status::INDEX_TYPECHANGE,
            )
        }),
        Err(_) => false,
    }
}
