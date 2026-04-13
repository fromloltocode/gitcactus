//! Branch listing and safe switching via git2.
//!
//! Read operations are always safe. Switching only happens after explicit
//! user confirmation and is blocked when the working tree is dirty.

use git2::{BranchType, Repository};

/// Summary of a single local branch.
#[derive(Debug, Clone)]
pub struct BranchEntry {
    pub name: String,
    pub is_current: bool,
    /// Abbreviated commit hash the branch points to.
    pub short_hash: String,
    /// First line of the branch-tip commit message.
    pub summary: String,
}

/// Result of loading the branch list.
#[derive(Debug, Clone)]
pub struct BranchListResult {
    pub branches: Vec<BranchEntry>,
    pub error: Option<String>,
}

/// Load local branches from the repository at `path`.
pub fn load_branches(path: &str) -> BranchListResult {
    let repo = match Repository::discover(path) {
        Ok(r) => r,
        Err(_) => {
            return BranchListResult {
                branches: vec![],
                error: Some("Not a git repository.".into()),
            };
        }
    };

    let current_name = repo
        .head()
        .ok()
        .and_then(|h| h.shorthand().map(String::from));

    let branches_iter = match repo.branches(Some(BranchType::Local)) {
        Ok(b) => b,
        Err(e) => {
            return BranchListResult {
                branches: vec![],
                error: Some(format!("Could not list branches: {e}")),
            };
        }
    };

    let mut branches = Vec::new();

    for item in branches_iter {
        let (branch, _) = match item {
            Ok(b) => b,
            Err(_) => continue,
        };

        let name = match branch.name() {
            Ok(Some(n)) => n.to_string(),
            _ => continue,
        };

        let is_current = current_name.as_deref() == Some(name.as_str());

        let (short_hash, summary) = match branch.get().peel_to_commit() {
            Ok(commit) => {
                let hash = commit.id().to_string();
                let short = hash[..7.min(hash.len())].to_string();
                let msg = commit
                    .summary()
                    .unwrap_or("(no message)")
                    .to_string();
                (short, msg)
            }
            Err(_) => ("???????".into(), "(unknown)".into()),
        };

        branches.push(BranchEntry {
            name,
            is_current,
            short_hash,
            summary,
        });
    }

    // Sort: current branch first, then alphabetical.
    branches.sort_by(|a, b| b.is_current.cmp(&a.is_current).then(a.name.cmp(&b.name)));

    BranchListResult {
        branches,
        error: None,
    }
}

/// Result of a branch switch attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SwitchResult {
    Ok(String),
    DirtyWorkingTree,
    Error(String),
}

/// Attempt to switch to `branch_name`. Blocked if the working tree is dirty.
pub fn switch_branch(path: &str, branch_name: &str) -> SwitchResult {
    let repo = match Repository::discover(path) {
        Ok(r) => r,
        Err(e) => return SwitchResult::Error(format!("Not a repo: {e}")),
    };

    // Check for dirty working tree.
    if let Ok(statuses) = repo.statuses(None) {
        let dirty = statuses.iter().any(|e| {
            let s = e.status();
            s.intersects(
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
        });
        if dirty {
            return SwitchResult::DirtyWorkingTree;
        }
    }

    // Resolve branch to a commit.
    let branch = match repo.find_branch(branch_name, BranchType::Local) {
        Ok(b) => b,
        Err(e) => return SwitchResult::Error(format!("Branch not found: {e}")),
    };

    let commit = match branch.get().peel_to_commit() {
        Ok(c) => c,
        Err(e) => return SwitchResult::Error(format!("Could not resolve branch: {e}")),
    };

    let tree = match commit.tree() {
        Ok(t) => t,
        Err(e) => return SwitchResult::Error(format!("Could not read tree: {e}")),
    };

    // Checkout the tree.
    if let Err(e) = repo.checkout_tree(tree.as_object(), None) {
        return SwitchResult::Error(format!("Checkout failed: {e}"));
    }

    // Point HEAD at the branch.
    let refname = format!("refs/heads/{branch_name}");
    if let Err(e) = repo.set_head(&refname) {
        return SwitchResult::Error(format!("Could not set HEAD: {e}"));
    }

    SwitchResult::Ok(branch_name.to_string())
}
