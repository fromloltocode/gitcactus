/// Placeholder Git status integration layer.
///
/// This module wraps `git2` to provide a safe, read-only view of the current
/// repository state. For now it returns basic info from the real repo.
///
/// # Safety philosophy
/// GitCactus never silently mutates a repository. All write operations
/// (stage, commit, push, etc.) must be gated behind explicit user confirmation
/// in the UI layer. This module only exposes *read* operations until write
/// support is deliberately added with full confirmation flows.
use git2::Repository;

/// Summary of the current repo state shown on the status screen.
pub struct RepoStatus {
    pub branch: String,
    pub modified: Option<usize>,
    pub staged: Option<usize>,
    pub untracked: Option<usize>,
    /// Individual filenames per group (up to MAX_FILES_PER_GROUP each).
    pub staged_files: Vec<String>,
    pub modified_files: Vec<String>,
    pub untracked_files: Vec<String>,
    /// Whether we successfully read from a real repo.
    pub is_real: bool,
}

/// Attempt to read basic status from the repo at `path`.
///
/// Returns a `RepoStatus`. If the path is not a git repo or reading fails,
/// the struct will have `is_real = false` and placeholder values.
pub fn read_status(path: &str) -> RepoStatus {
    match Repository::discover(path) {
        Ok(repo) => read_from_repo(&repo),
        Err(_) => RepoStatus {
            branch: "(not a git repo)".into(),
            modified: None,
            staged: None,
            untracked: None,
            staged_files: Vec::new(),
            modified_files: Vec::new(),
            untracked_files: Vec::new(),
            is_real: false,
        },
    }
}

fn read_from_repo(repo: &Repository) -> RepoStatus {
    let branch = match repo.head() {
        Ok(head) => head.shorthand().unwrap_or("(detached HEAD)").to_string(),
        Err(_) => "(unknown)".into(),
    };

    let (mut modified, mut staged, mut untracked) = (0usize, 0usize, 0usize);
    let mut staged_files = Vec::new();
    let mut modified_files = Vec::new();
    let mut untracked_files = Vec::new();

    if let Ok(statuses) = repo.statuses(None) {
        for entry in statuses.iter() {
            let s = entry.status();
            let name = entry.path().unwrap_or("(unknown)").to_string();

            if s.intersects(
                git2::Status::WT_MODIFIED
                    | git2::Status::WT_DELETED
                    | git2::Status::WT_RENAMED
                    | git2::Status::WT_TYPECHANGE,
            ) {
                modified_files.push(name.clone());
                modified += 1;
            }
            if s.intersects(
                git2::Status::INDEX_NEW
                    | git2::Status::INDEX_MODIFIED
                    | git2::Status::INDEX_DELETED
                    | git2::Status::INDEX_RENAMED
                    | git2::Status::INDEX_TYPECHANGE,
            ) {
                staged_files.push(name.clone());
                staged += 1;
            }
            if s.contains(git2::Status::WT_NEW) {
                untracked_files.push(name);
                untracked += 1;
            }
        }
    }

    RepoStatus {
        branch,
        modified: Some(modified),
        staged: Some(staged),
        untracked: Some(untracked),
        staged_files,
        modified_files,
        untracked_files,
        is_real: true,
    }
}
