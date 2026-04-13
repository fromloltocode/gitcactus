//! Safe commit operation via git2.
//!
//! Creates a commit from whatever is currently in the index (staging area).
//! Never stages files implicitly — only commits what the user has already
//! staged.

use git2::Repository;

/// Create a commit with the given `message` from the current index.
///
/// Returns the abbreviated commit hash on success, or an error message.
pub fn create_commit(repo_path: &str, message: &str) -> Result<String, String> {
    let repo = Repository::discover(repo_path).map_err(|e| format!("Not a git repo: {e}"))?;
    let sig = repo.signature().map_err(|e| format!("Could not determine author: {e}"))?;

    let mut index = repo.index().map_err(|e| format!("Could not read index: {e}"))?;
    let tree_oid = index
        .write_tree()
        .map_err(|e| format!("Could not write tree: {e}"))?;
    let tree = repo
        .find_tree(tree_oid)
        .map_err(|e| format!("Could not find tree: {e}"))?;

    // Determine parent commits. For the very first commit there is no parent.
    let parents: Vec<git2::Commit> = match repo.head() {
        Ok(head) => {
            let commit = head
                .peel_to_commit()
                .map_err(|e| format!("Could not find HEAD commit: {e}"))?;
            vec![commit]
        }
        Err(_) => vec![], // initial commit
    };
    let parent_refs: Vec<&git2::Commit> = parents.iter().collect();

    let oid = repo
        .commit(Some("HEAD"), &sig, &sig, message, &tree, &parent_refs)
        .map_err(|e| format!("Commit failed: {e}"))?;

    // Return abbreviated hash (first 7 chars).
    let short = &oid.to_string()[..7];
    Ok(short.to_string())
}
