/// Safe, explicit staging operations.
///
/// Stages only the files explicitly passed in. Never stages everything.
/// Returns the count of files successfully staged, or an error message.
use std::path::Path;

use git2::Repository;

pub fn stage_files(repo_path: &str, files: &[String]) -> Result<usize, String> {
    if files.is_empty() {
        return Ok(0);
    }

    let repo = Repository::discover(repo_path).map_err(|e| format!("Not a git repo: {e}"))?;
    let mut index = repo.index().map_err(|e| format!("Could not read index: {e}"))?;

    for file in files {
        index
            .add_path(Path::new(file))
            .map_err(|e| format!("Failed to stage {file}: {e}"))?;
    }

    index.write().map_err(|e| format!("Failed to write index: {e}"))?;

    Ok(files.len())
}
