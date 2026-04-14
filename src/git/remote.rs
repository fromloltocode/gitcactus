//! Remote-aware git operations for the Remote Sync screen.
//!
//! This module is **Phase 1** of Remote Sync — focused on *visibility* and
//! *safe read-mostly behaviour*:
//!
//! - [`load_remote_info`]: pure read. Enumerates remotes, finds the
//!   current branch's upstream (if any), and computes ahead/behind
//!   counts against it without touching the network.
//! - [`fetch`]: downloads refs from a named remote. This writes to the
//!   local refs/remotes/ namespace but does **not** touch the working
//!   tree, the index, HEAD, or any local branch.
//!
//! Everything that could mutate local history (push, pull, rebase,
//! merge) lives elsewhere or is intentionally not implemented yet.
//!
//! Authentication is wired up conservatively:
//! - SSH URLs try `ssh-agent` via `Cred::ssh_key_from_agent`.
//! - HTTPS URLs try the user's configured git credential helper.
//!
//! We never prompt, never store credentials, and never log them.

use git2::{
    Cred, CredentialType, Direction, ErrorClass, ErrorCode, FetchOptions, RemoteCallbacks,
    Repository,
};

// ── Data types ───────────────────────────────────────────────────────

/// A single configured remote ("origin", "upstream", etc.).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteEntry {
    pub name: String,
    /// The fetch URL. Empty string if unreadable.
    pub url: String,
}

/// How the current local branch relates to its configured upstream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackingInfo {
    /// Name of the remote the branch tracks (e.g. "origin").
    pub remote_name: String,
    /// Shorthand of the upstream branch (e.g. "origin/main").
    pub upstream: String,
    /// Commits on the local branch that are not on the upstream.
    pub ahead: usize,
    /// Commits on the upstream that are not on the local branch.
    pub behind: usize,
    /// True when we could not compute ahead/behind (e.g. the remote
    /// ref does not exist locally — a fetch is needed).
    pub ahead_behind_unknown: bool,
}

/// Snapshot of remote-related state for the Remote Sync screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteInfo {
    /// All configured remotes.
    pub remotes: Vec<RemoteEntry>,
    /// Current local branch name, or `None` in a detached-HEAD state.
    pub current_branch: Option<String>,
    /// Tracking details for the current branch, if it has an upstream.
    pub tracking: Option<TrackingInfo>,
    /// True if the current directory resolved to a real git repo.
    pub is_real: bool,
    /// Fatal error message (repo discovery failed, etc.). Non-fatal
    /// per-remote errors are silently skipped.
    pub error: Option<String>,
}

/// Outcome of a single fetch attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FetchResult {
    /// Fetch completed. The remote ref namespace may have advanced.
    Ok { remote: String },
    /// The repository has no remotes configured at all.
    NoRemote,
    /// The named remote does not exist in this repository.
    NoSuchRemote(String),
    /// Authentication failed. Message is user-facing.
    AuthFailed(String),
    /// Network-level failure (DNS, TLS, refused connection, etc.).
    NetworkError(String),
    /// Any other failure, with the git2 message preserved.
    Error(String),
}

// ── Read-only: remote info ───────────────────────────────────────────

/// Load a snapshot of the repository's remote state.
///
/// This function is **read-only**: no network, no mutation.
pub fn load_remote_info(path: &str) -> RemoteInfo {
    let repo = match Repository::discover(path) {
        Ok(r) => r,
        Err(_) => {
            return RemoteInfo {
                remotes: vec![],
                current_branch: None,
                tracking: None,
                is_real: false,
                error: Some("Not a git repository.".into()),
            };
        }
    };

    // Enumerate remotes.
    let mut remotes: Vec<RemoteEntry> = Vec::new();
    if let Ok(names) = repo.remotes() {
        for name in names.iter().flatten() {
            let url = repo
                .find_remote(name)
                .ok()
                .and_then(|r| r.url().map(String::from))
                .unwrap_or_default();
            remotes.push(RemoteEntry {
                name: name.to_string(),
                url,
            });
        }
    }
    remotes.sort_by(|a, b| a.name.cmp(&b.name));

    // Current branch name (None = detached HEAD).
    let current_branch = repo
        .head()
        .ok()
        .filter(|h| h.is_branch())
        .and_then(|h| h.shorthand().map(String::from));

    // Tracking info, if the current branch has an upstream.
    let tracking = current_branch
        .as_deref()
        .and_then(|b| resolve_tracking(&repo, b));

    RemoteInfo {
        remotes,
        current_branch,
        tracking,
        is_real: true,
        error: None,
    }
}

/// Resolve the upstream of `branch_name` and compute ahead/behind.
///
/// Returns `None` if the branch has no configured upstream at all.
fn resolve_tracking(repo: &Repository, branch_name: &str) -> Option<TrackingInfo> {
    let local = repo
        .find_branch(branch_name, git2::BranchType::Local)
        .ok()?;

    let upstream_branch = local.upstream().ok()?;
    let upstream_shorthand = upstream_branch
        .name()
        .ok()
        .flatten()
        .map(String::from)
        .unwrap_or_default();

    // The remote name sits at the start of the upstream refname, e.g.
    // "origin/main" → "origin". Fall back to the branch config lookup
    // if the shorthand parse fails.
    let remote_name = upstream_shorthand
        .split_once('/')
        .map(|(r, _)| r.to_string())
        .or_else(|| {
            let cfg = repo.config().ok()?;
            cfg.get_string(&format!("branch.{branch_name}.remote")).ok()
        })
        .unwrap_or_default();

    // Try to compute ahead/behind. If either ref cannot be resolved,
    // flag it rather than silently dropping the upstream.
    let local_oid = local.get().target();
    let upstream_oid = upstream_branch.get().target();

    let (ahead, behind, unknown) = match (local_oid, upstream_oid) {
        (Some(l), Some(u)) => match repo.graph_ahead_behind(l, u) {
            Ok((a, b)) => (a, b, false),
            Err(_) => (0, 0, true),
        },
        _ => (0, 0, true),
    };

    Some(TrackingInfo {
        remote_name,
        upstream: upstream_shorthand,
        ahead,
        behind,
        ahead_behind_unknown: unknown,
    })
}

// ── Fetch ────────────────────────────────────────────────────────────

/// Fetch from the named remote.
///
/// Updates `refs/remotes/<remote>/*` but never touches the working
/// tree, the index, HEAD, or any local branch.
pub fn fetch(path: &str, remote_name: &str) -> FetchResult {
    let repo = match Repository::discover(path) {
        Ok(r) => r,
        Err(e) => return FetchResult::Error(format!("Not a repo: {e}")),
    };

    // Guard: confirm at least one remote is configured.
    match repo.remotes() {
        Ok(names) if names.is_empty() => return FetchResult::NoRemote,
        Err(e) => return FetchResult::Error(format!("Could not list remotes: {e}")),
        _ => {}
    }

    let mut remote = match repo.find_remote(remote_name) {
        Ok(r) => r,
        Err(_) => return FetchResult::NoSuchRemote(remote_name.to_string()),
    };

    // Auth callbacks. We distinguish "auth failed" vs "network failed"
    // via the error classifier below rather than tracking state here.
    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(credentials_callback);

    let mut fetch_opts = FetchOptions::new();
    fetch_opts.remote_callbacks(callbacks);

    // Use the remote's configured refspecs — same refs `git fetch` would.
    // Passing an empty slice tells git2 to use the default.
    let refspecs: [&str; 0] = [];

    // Connect first so we can produce a clean auth-vs-network distinction.
    if let Err(e) = remote.connect(Direction::Fetch) {
        return classify_error(e, remote_name);
    }

    let result = remote.fetch(&refspecs, Some(&mut fetch_opts), None);
    // Always disconnect, even if fetch failed.
    let _ = remote.disconnect();

    match result {
        Ok(()) => FetchResult::Ok {
            remote: remote_name.to_string(),
        },
        Err(e) => classify_error(e, remote_name),
    }
}

/// Map a `git2::Error` to our user-facing [`FetchResult`] variant.
fn classify_error(e: git2::Error, remote_name: &str) -> FetchResult {
    match (e.class(), e.code()) {
        (ErrorClass::Http | ErrorClass::Ssh | ErrorClass::Callback, _) => {
            if looks_like_auth(&e) {
                FetchResult::AuthFailed(format!(
                    "Authentication failed for '{remote_name}'. \
                     Check your SSH key / credential helper, then try again."
                ))
            } else {
                FetchResult::NetworkError(format!(
                    "Network error fetching '{remote_name}': {}",
                    e.message()
                ))
            }
        }
        (ErrorClass::Net, _) | (_, ErrorCode::GenericError) if looks_like_network(&e) => {
            FetchResult::NetworkError(format!(
                "Network error fetching '{remote_name}': {}",
                e.message()
            ))
        }
        _ => FetchResult::Error(format!("Fetch failed: {}", e.message())),
    }
}

fn looks_like_auth(e: &git2::Error) -> bool {
    let msg = e.message().to_lowercase();
    msg.contains("authentication")
        || msg.contains("credentials")
        || msg.contains("unauthorized")
        || msg.contains("permission denied")
        || msg.contains("authenticity")
}

fn looks_like_network(e: &git2::Error) -> bool {
    let msg = e.message().to_lowercase();
    msg.contains("resolve")
        || msg.contains("connection")
        || msg.contains("network")
        || msg.contains("timed out")
        || msg.contains("no route")
}

/// Credentials callback used during fetch.
///
/// Order of attempts:
/// 1. SSH agent (for SSH URLs where git2 asks for `SSH_KEY`).
/// 2. Git credential helper (for HTTPS where git2 asks for `USER_PASS_PLAINTEXT`).
/// 3. Default — may still succeed for public HTTP endpoints.
fn credentials_callback(
    url: &str,
    username_from_url: Option<&str>,
    allowed: CredentialType,
) -> Result<Cred, git2::Error> {
    if allowed.contains(CredentialType::SSH_KEY) {
        let user = username_from_url.unwrap_or("git");
        return Cred::ssh_key_from_agent(user);
    }
    if allowed.contains(CredentialType::USER_PASS_PLAINTEXT) {
        if let Ok(config) = git2::Config::open_default() {
            if let Ok(cred) = Cred::credential_helper(&config, url, username_from_url) {
                return Ok(cred);
            }
        }
    }
    if allowed.contains(CredentialType::DEFAULT) {
        return Cred::default();
    }
    Err(git2::Error::from_str("no supported authentication method"))
}
