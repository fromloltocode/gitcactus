//! Scan approved roots for local Git repositories.
//!
//! Discovery is deliberately conservative:
//!
//! - **Depth cap**: bounded DFS at 4 levels below each root. Typical
//!   project layouts (`~/Projects/foo`, `~/Projects/client/foo`) fit
//!   well within that without letting users accidentally walk from a
//!   deep root into the whole disk.
//! - **Skip list**: dev-tool build dirs (`node_modules`, `target`,
//!   `build`, etc.) are never descended into. They're the #1 source
//!   of directory-tree explosions and they never contain Git repos
//!   of their own that a user would want to open.
//! - **`.git` probe**: a directory is considered a Prickling if
//!   `dir/.git` exists as either a directory (normal repo) or a file
//!   (worktree / submodule indirection). Once we recognise a
//!   Prickling we do **not** descend into it — nested repos inside
//!   workspaces stay at the workspace level.
//! - **No symlink following**: prevents loops and avoids surprising
//!   the user with repos outside the approved root.
//! - **No writes**: this is strictly `fs::read_dir` + `Path::exists`.
//!   No git2 calls, no subprocesses.

use std::fs;
use std::path::{Path, PathBuf};

use super::Prickling;

/// Maximum recursion depth from an approved root. Depth 0 is the
/// root itself. A repo at `root/a/b/c/d/.git` is the deepest we'd
/// find; `root/a/b/c/d/e` would be skipped.
pub const MAX_DEPTH: usize = 4;

/// Directory names we never descend into, regardless of depth.
///
/// Ordered roughly by frequency of appearance in real projects so
/// the `contains` probe short-circuits early.
const SKIP_DIRS: &[&str] = &[
    "node_modules",
    "target",
    ".git",
    "build",
    "dist",
    ".venv",
    "venv",
    "__pycache__",
    ".cache",
    ".next",
    ".nuxt",
    ".pnpm-store",
    ".yarn",
    ".tox",
    ".gradle",
    ".idea",
    ".vscode",
    ".DS_Store",
];

/// The outcome of a scan — pricklings found plus any per-directory
/// errors the scanner encountered (e.g. permission denied). Errors
/// are surfaced to the UI rather than silently dropped so users can
/// tell if a permission problem is masking repos they expected.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct ScanResult {
    pub found: Vec<Prickling>,
    pub errors: Vec<(PathBuf, String)>,
}

/// Scan each approved root in order, collecting all discovered
/// pricklings. Results are deduplicated by canonical path.
pub fn scan_roots(roots: &[PathBuf]) -> ScanResult {
    let mut out = ScanResult::default();
    for root in roots {
        let single = scan_root(root, MAX_DEPTH);
        for p in single.found {
            if !out.found.iter().any(|existing| existing.path == p.path) {
                out.found.push(p);
            }
        }
        out.errors.extend(single.errors);
    }
    // Stable alphabetical by display name, so refreshed scans don't
    // reshuffle rows under the user.
    out.found.sort_by(|a, b| a.display_name.cmp(&b.display_name));
    out
}

/// Scan a single root to `max_depth`. Equivalent to calling
/// `scan_roots(&[root])` but lets tests pick a custom depth.
pub fn scan_root(root: &Path, max_depth: usize) -> ScanResult {
    let mut result = ScanResult::default();
    // Resolve the root to its canonical form so reported paths match
    // what save_prickling would record if the user hit "Save".
    let canon_root = match fs::canonicalize(root) {
        Ok(p) => p,
        Err(e) => {
            result.errors.push((root.to_path_buf(), e.to_string()));
            return result;
        }
    };
    walk(&canon_root, 0, max_depth, &mut result);
    result
}

fn walk(dir: &Path, depth: usize, max_depth: usize, out: &mut ScanResult) {
    // A directory that *is* a repo terminates the descent here — we
    // don't want nested .git dirs (like bare clones inside a parent
    // repo) to each show up as their own Prickling under a shared
    // umbrella.
    if is_git_repo(dir) {
        let display_name = dir
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| dir.to_string_lossy().into_owned());
        out.found.push(Prickling {
            path: dir.to_path_buf(),
            display_name,
        });
        return;
    }

    if depth >= max_depth {
        return;
    }

    let entries = match fs::read_dir(dir) {
        Ok(r) => r,
        Err(e) => {
            // Permission denied, vanished directory, etc. — record
            // and keep scanning siblings.
            out.errors.push((dir.to_path_buf(), e.to_string()));
            return;
        }
    };

    for entry in entries.flatten() {
        let Ok(meta) = entry.file_type() else {
            continue;
        };

        // Skip symlinks entirely — they're the easiest way to escape
        // the approved root and the trickiest source of loops.
        if meta.is_symlink() || !meta.is_dir() {
            continue;
        }

        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        // Skip hidden dirs except ones that could be valid project
        // roots. `.git` itself is in the skip list and caught below.
        if name_str.starts_with('.') && !is_allowed_dotdir(&name_str) {
            continue;
        }

        if SKIP_DIRS.iter().any(|s| *s == name_str) {
            continue;
        }

        let child = entry.path();
        walk(&child, depth + 1, max_depth, out);
    }
}

/// Return true if `dir` looks like a Git repository root.
///
/// Accepts both `.git/` directories (normal repos) and `.git` regular
/// files (worktree / submodule indirection). We don't follow the
/// indirection in Phase 1 — the outer directory is still the thing
/// the user would want to "open".
fn is_git_repo(dir: &Path) -> bool {
    let dot_git = dir.join(".git");
    // `exists()` is cheap and works for both the dir and file cases.
    dot_git.exists()
}

/// Allow-list of dot-prefixed directory names that may be a valid
/// project root. Most projects live under non-dotted directory names
/// but a small handful (like `.config/something/repo`) don't, so we
/// don't want to blanket-skip every hidden directory.
fn is_allowed_dotdir(name: &str) -> bool {
    matches!(name, ".config" | ".local" | ".dotfiles")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Produce a scratch directory under `$TMPDIR/gitcactus-tests/`
    /// keyed by the test name so parallel tests don't collide, and
    /// clean it on drop. No extra dev-deps.
    struct TempDir {
        path: PathBuf,
    }
    impl TempDir {
        fn new(key: &str) -> Self {
            let root = std::env::temp_dir()
                .join("gitcactus-tests")
                .join(format!("{key}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(&root).unwrap();
            Self { path: root }
        }
    }
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    /// Create a fake git repo at `path` by making `path/.git/` exist.
    fn make_repo(path: &Path) {
        fs::create_dir_all(path.join(".git")).unwrap();
    }

    /// Create a fake "worktree" by writing a `.git` *file*.
    fn make_worktree(path: &Path, gitdir: &Path) {
        fs::create_dir_all(path).unwrap();
        fs::write(
            path.join(".git"),
            format!("gitdir: {}\n", gitdir.display()),
        )
        .unwrap();
    }

    #[test]
    fn finds_repo_at_shallow_depth() {
        let td = TempDir::new("shallow");
        make_repo(&td.path.join("foo"));
        make_repo(&td.path.join("bar"));
        fs::create_dir_all(td.path.join("not-a-repo")).unwrap();

        let result = scan_root(&td.path, MAX_DEPTH);
        assert_eq!(result.found.len(), 2);
        let names: Vec<_> = result
            .found
            .iter()
            .map(|p| p.display_name.clone())
            .collect();
        assert!(names.contains(&"foo".into()));
        assert!(names.contains(&"bar".into()));
    }

    #[test]
    fn ignores_non_git_directories() {
        let td = TempDir::new("no-repos");
        fs::create_dir_all(td.path.join("stuff")).unwrap();
        fs::create_dir_all(td.path.join("docs/drafts")).unwrap();
        let result = scan_root(&td.path, MAX_DEPTH);
        assert!(result.found.is_empty());
    }

    #[test]
    fn dedupes_repos_found_via_overlapping_roots() {
        let td = TempDir::new("dedup");
        make_repo(&td.path.join("proj"));
        let result = scan_roots(&[td.path.clone(), td.path.clone()]);
        assert_eq!(result.found.len(), 1);
    }

    #[test]
    fn respects_depth_cap() {
        let td = TempDir::new("deep");
        // Create a repo at depth 5 — just over the cap.
        let deep = td.path.join("a/b/c/d/e");
        make_repo(&deep);
        let result = scan_root(&td.path, MAX_DEPTH);
        assert!(result.found.is_empty(), "depth-5 repo must not be found");

        // Same dir with max_depth=5 should find it.
        let result5 = scan_root(&td.path, 5);
        assert_eq!(result5.found.len(), 1);
    }

    #[test]
    fn does_not_descend_into_repos() {
        let td = TempDir::new("nested");
        make_repo(&td.path.join("outer"));
        // A second .git dir underneath outer shouldn't become a
        // separate Prickling — the outer repo swallows everything.
        make_repo(&td.path.join("outer/inner"));
        let result = scan_root(&td.path, MAX_DEPTH);
        assert_eq!(result.found.len(), 1);
        assert_eq!(result.found[0].display_name, "outer");
    }

    #[test]
    fn skips_noise_directories() {
        let td = TempDir::new("noise");
        // A repo inside node_modules should NOT be surfaced.
        make_repo(&td.path.join("node_modules/junk"));
        make_repo(&td.path.join("target/debug/build/something"));
        // A legitimate sibling repo should.
        make_repo(&td.path.join("real-project"));
        let result = scan_root(&td.path, MAX_DEPTH);
        let names: Vec<_> = result
            .found
            .iter()
            .map(|p| p.display_name.clone())
            .collect();
        assert_eq!(names, vec!["real-project".to_string()]);
    }

    #[test]
    fn detects_worktree_git_file() {
        let td = TempDir::new("worktree");
        let gitdir = td.path.join(".git-actual");
        fs::create_dir_all(&gitdir).unwrap();
        make_worktree(&td.path.join("wt"), &gitdir);
        let result = scan_root(&td.path, MAX_DEPTH);
        assert_eq!(result.found.len(), 1);
        assert_eq!(result.found[0].display_name, "wt");
    }

    #[test]
    fn records_error_for_nonexistent_root() {
        let result = scan_root(
            Path::new("/nonexistent/gitcactus/does-not-exist"),
            MAX_DEPTH,
        );
        assert!(result.found.is_empty());
        assert_eq!(result.errors.len(), 1);
    }

    #[test]
    fn results_are_sorted_by_display_name() {
        let td = TempDir::new("sorted");
        make_repo(&td.path.join("zzz"));
        make_repo(&td.path.join("aaa"));
        make_repo(&td.path.join("mmm"));
        let result = scan_roots(std::slice::from_ref(&td.path));
        let names: Vec<_> = result
            .found
            .iter()
            .map(|p| p.display_name.clone())
            .collect();
        assert_eq!(names, vec!["aaa", "mmm", "zzz"]);
    }

    #[test]
    fn discovery_does_not_mutate_filesystem() {
        // Snapshot directory contents before + after a scan and
        // verify nothing moved. Guards against a regression where a
        // well-meaning "normalize" step writes something back.
        let td = TempDir::new("immutable");
        make_repo(&td.path.join("p1"));
        make_repo(&td.path.join("p2"));
        let before: Vec<_> = walk_all(&td.path);
        let _ = scan_root(&td.path, MAX_DEPTH);
        let after: Vec<_> = walk_all(&td.path);
        assert_eq!(before, after);
    }

    fn walk_all(root: &Path) -> Vec<PathBuf> {
        let mut out = Vec::new();
        fn go(d: &Path, out: &mut Vec<PathBuf>) {
            if let Ok(entries) = fs::read_dir(d) {
                for e in entries.flatten() {
                    out.push(e.path());
                    if e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                        go(&e.path(), out);
                    }
                }
            }
        }
        go(root, &mut out);
        out.sort();
        out
    }
}
