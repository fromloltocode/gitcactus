//! Rebase execution — actually runs `git rebase`.
//!
//! **This module mutates the repository.** Every caller must treat its
//! results as authoritative and surface them to the user honestly.
//!
//! # Implementation choice
//!
//! We invoke the system `git` binary via `std::process::Command` rather
//! than using `git2`'s lower-level rebase API. The reasons, spelled out
//! so the choice is never hidden:
//!
//! 1. `git rebase` is the most widely tested, semantically-correct
//!    implementation. Using it means the user gets the same behavior
//!    they'd get on the command line.
//! 2. Exit codes and stderr output are well-defined and easy to capture
//!    deterministically.
//! 3. Conflict detection is straightforward: we check both the exit
//!    status *and* the presence of `.git/rebase-merge` /
//!    `.git/rebase-apply` on disk.
//! 4. We can't accidentally corrupt index state with a partial
//!    implementation; `git` handles atomicity.
//!
//! This module performs **only these operations**: preflight status
//! check (read-only), `git rebase <target>` (mutation),
//! `git rebase --abort` (mutation), and `git rebase --continue`
//! (mutation, guarded by an explicit in-progress check). Nothing else.
//! No `--skip`, no force, no autosquash, no interactive rebase.

use std::path::PathBuf;
use std::process::Command;

use git2::Repository;

/// The outcome of running a rebase.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecuteResult {
    /// Rebase completed successfully. All commits replayed cleanly.
    Success,
    /// Rebase stopped because of a conflict. The repo is in a rebase
    /// state on disk and the user must resolve manually or abort.
    Conflict {
        /// Stderr captured from `git rebase`, trimmed.
        stderr: String,
    },
    /// Rebase could not even start or failed for some other reason.
    Failure {
        /// Human-readable reason.
        message: String,
    },
    /// We refused to start because preflight safety checks failed.
    Blocked {
        /// Reason we blocked (dirty tree, same branch, etc.).
        reason: String,
    },
}

/// Preflight checks performed *before* any mutation. If any of these
/// fail we return [`ExecuteResult::Blocked`] without running `git`.
pub fn preflight(repo_path: &str, target: &str) -> Option<ExecuteResult> {
    let repo = match Repository::discover(repo_path) {
        Ok(r) => r,
        Err(e) => {
            return Some(ExecuteResult::Failure {
                message: format!("Not a git repository: {e}"),
            });
        }
    };

    // Block on detached HEAD.
    let head = match repo.head() {
        Ok(h) => h,
        Err(_) => {
            return Some(ExecuteResult::Blocked {
                reason: "Repository has no HEAD yet.".into(),
            });
        }
    };
    if !head.is_branch() {
        return Some(ExecuteResult::Blocked {
            reason: "Detached HEAD — check out a branch first.".into(),
        });
    }

    // Block if source == target.
    if head.shorthand() == Some(target) {
        return Some(ExecuteResult::Blocked {
            reason: "Cannot rebase a branch onto itself.".into(),
        });
    }

    // Block on dirty working tree.
    if is_dirty(&repo) {
        return Some(ExecuteResult::Blocked {
            reason: "Working tree has uncommitted changes. Commit or stash first.".into(),
        });
    }

    // Block if target branch doesn't exist.
    if repo.find_branch(target, git2::BranchType::Local).is_err() {
        return Some(ExecuteResult::Blocked {
            reason: format!("Target branch '{target}' not found."),
        });
    }

    None
}

/// Execute `git rebase <target>` inside `repo_path`. **Mutates the repository.**
///
/// Always call [`preflight`] first and only invoke this when it returns `None`.
pub fn execute_rebase(repo_path: &str, target: &str) -> ExecuteResult {
    // Invoke `git` directly so we get well-tested behaviour and clear exit codes.
    let output = match Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .arg("rebase")
        .arg(target)
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            return ExecuteResult::Failure {
                message: format!("Failed to invoke git: {e}"),
            };
        }
    };

    if output.status.success() {
        return ExecuteResult::Success;
    }

    // Non-zero exit: distinguish conflict vs plain failure.
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if is_rebase_in_progress(repo_path) {
        ExecuteResult::Conflict { stderr }
    } else {
        ExecuteResult::Failure {
            message: if stderr.is_empty() {
                "git rebase failed with no stderr".into()
            } else {
                stderr
            },
        }
    }
}

/// Run `git rebase --abort`. **Mutates the repository.**
///
/// Intended for use from the conflict state of the execute screen.
/// Guarded: refuses to run if no rebase is actually in progress, so
/// an errant 'a' press on a stale screen can't do anything weird.
pub fn abort_rebase(repo_path: &str) -> Result<(), String> {
    if !is_rebase_in_progress(repo_path) {
        return Err("No rebase is in progress.".into());
    }
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .arg("rebase")
        .arg("--abort")
        .output()
        .map_err(|e| format!("Failed to invoke git: {e}"))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

/// Outcome of `git rebase --continue`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContinueResult {
    /// Rebase fully finished — no rebase-in-progress on disk after.
    Finished,
    /// Git accepted the continue and applied a commit, but hit *another*
    /// conflict on a later commit. The repo is still in rebase state.
    AnotherConflict { stderr: String },
    /// Git refused to continue (unmerged paths, nothing to commit, etc.).
    /// The repo is still in rebase state and the user must act.
    Blocked { stderr: String },
    /// Some other non-zero exit that isn't covered by the cases above.
    Error { message: String },
    /// We refused to run because no rebase is actually in progress.
    NotInRebase,
}

/// Run `git rebase --continue`. **Mutates the repository.**
///
/// Safety rules implemented here:
///
/// * We **refuse to run** unless [`is_rebase_in_progress`] reports true.
///   This is the gate the task asks for — we never call continue out of
///   context.
/// * We never add `--force` or any similar flag.
/// * We never `git add` on behalf of the user. They must stage their
///   conflict resolutions themselves (or use the Stage screen).
/// * We do not pass `-c core.editor` or anything to skip the commit
///   editor — `git rebase --continue` may open the user's editor for a
///   message; the TUI suspension path is the caller's responsibility.
///   For this pass we pass `GIT_EDITOR=true` so continue never blocks
///   on an editor prompt. Messages from prior commits are kept as-is.
/// * Stderr is captured verbatim and handed back for display.
pub fn continue_rebase(repo_path: &str) -> ContinueResult {
    if !is_rebase_in_progress(repo_path) {
        return ContinueResult::NotInRebase;
    }

    let output = match Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .arg("rebase")
        .arg("--continue")
        // Never pop an editor from under the TUI. If a commit needs a
        // message, keep the existing one (git -c behavior w/ GIT_EDITOR=true).
        .env("GIT_EDITOR", "true")
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            return ContinueResult::Error {
                message: format!("Failed to invoke git: {e}"),
            };
        }
    };

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let still_in_rebase = is_rebase_in_progress(repo_path);

    match (output.status.success(), still_in_rebase) {
        // Exit 0 and no more rebase dirs → rebase is fully done.
        (true, false) => ContinueResult::Finished,
        // Exit 0 but rebase is still in progress shouldn't normally happen,
        // but treat it as a soft re-conflict rather than silently declaring
        // success.
        (true, true) => ContinueResult::AnotherConflict { stderr },
        // Non-zero + still in rebase → either another conflict or a hard
        // block (e.g. "Unmerged paths"). Use stderr to decide.
        (false, true) => {
            if looks_like_unmerged_block(&stderr) {
                ContinueResult::Blocked { stderr }
            } else {
                ContinueResult::AnotherConflict { stderr }
            }
        }
        // Non-zero and no rebase state on disk → something went wrong
        // and the rebase was unwound.
        (false, false) => ContinueResult::Error {
            message: if stderr.is_empty() {
                "git rebase --continue failed and the rebase is no longer in progress".into()
            } else {
                stderr
            },
        },
    }
}

fn looks_like_unmerged_block(stderr: &str) -> bool {
    let lower = stderr.to_lowercase();
    lower.contains("unmerged path")
        || lower.contains("needs merge")
        || lower.contains("resolve all merge conflicts")
        || lower.contains("no changes - did you forget to use")
        || lower.contains("nothing to commit")
}

/// Check whether a rebase is currently in progress on disk.
///
/// Public so screens can gate UI hints without duplicating the logic.
pub fn is_rebase_in_progress(repo_path: &str) -> bool {
    // Look relative to the discovered git directory, not `repo_path`,
    // so this works in worktrees and subdirectories.
    let git_dir = match Repository::discover(repo_path) {
        Ok(r) => r.path().to_path_buf(),
        Err(_) => PathBuf::from(repo_path).join(".git"),
    };
    git_dir.join("rebase-merge").exists() || git_dir.join("rebase-apply").exists()
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
