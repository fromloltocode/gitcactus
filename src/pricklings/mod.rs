//! Pricklings — GitCactus's term for a local Git project the user can
//! open, save, and reopen.
//!
//! This module owns three small concerns:
//!
//! - **Types** ([`Prickling`], [`PricklingsStore`]) — lightweight data
//!   structures for one local repo and for the entire persisted state.
//! - **Persistence** — reading and writing the `pricklings` file next
//!   to `settings` in the platform config directory, using the same
//!   plain key=value convention.
//! - **Submodules** — discovery ([`discovery`]) walks approved roots,
//!   and default roots ([`roots`]) offers sensible platform-specific
//!   starting points without ever scanning them automatically.
//!
//! # Safety / privacy
//!
//! The whole point of this module is that the user is in control:
//!
//! - No automatic scanning. Every scan is user-initiated.
//! - No whole-filesystem walks. Only directories the user has
//!   explicitly approved.
//! - No network. Pricklings are local paths, full stop.
//! - Discovery is strictly read-only (`fs::read_dir`, `path.exists()`).
//!   No `git2` writes, no shell-outs, no subprocesses.

pub mod discovery;
pub mod roots;

use std::fs;
use std::path::{Path, PathBuf};

use crate::platform;

/// A single local Git repository that's been discovered or saved.
///
/// `path` is always canonical (symlinks resolved, `.` / `..` folded)
/// so two pointers at the same on-disk directory compare equal and
/// only appear once in any list.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Prickling {
    pub path: PathBuf,
    pub display_name: String,
}

impl Prickling {
    /// Build a Prickling from an arbitrary path. Canonicalises on
    /// a best-effort basis; falls back to the raw path if the OS
    /// refuses (e.g. a broken symlink).
    #[allow(dead_code)]
    pub fn from_path<P: AsRef<Path>>(p: P) -> Self {
        let raw = p.as_ref().to_path_buf();
        let canonical = fs::canonicalize(&raw).unwrap_or(raw);
        let display_name = canonical
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| canonical.to_string_lossy().into_owned());
        Self {
            path: canonical,
            display_name,
        }
    }

    /// User-visible path string. Uses `~` shorthand on Unix when the
    /// path is inside `$HOME`, so `/Users/alice/Projects/foo` renders
    /// as `~/Projects/foo` in the UI.
    pub fn display_path(&self) -> String {
        shorten_home(&self.path)
    }
}

/// The full on-disk state for the Pricklings feature.
///
/// Two collections, both deduplicated:
/// - `scan_roots`: directories the user has approved for scanning.
/// - `saved`:      pricklings the user explicitly asked to remember.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PricklingsStore {
    pub scan_roots: Vec<PathBuf>,
    pub saved: Vec<Prickling>,
}

impl PricklingsStore {
    /// Load from disk, returning an empty store if the file doesn't
    /// exist or can't be read. Never fails — missing Pricklings data
    /// is a feature (no "first run" error), not an error.
    pub fn load() -> Self {
        let path = match platform::config_file("pricklings") {
            Some(p) => p,
            None => return Self::default(),
        };
        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => return Self::default(),
        };
        Self::parse(&content)
    }

    /// Serialize to disk. Creates parent directories on demand.
    /// Silently succeeds if the platform can't resolve a config dir
    /// (e.g. no HOME set) — we never crash the app over persistence.
    pub fn save(&self) {
        if let Some(path) = platform::config_file("pricklings") {
            if let Some(dir) = path.parent() {
                let _ = fs::create_dir_all(dir);
            }
            let _ = fs::write(&path, self.to_text());
        }
    }

    /// Pure parsing. Exposed for tests so we don't have to touch disk.
    pub fn parse(content: &str) -> Self {
        let mut scan_roots: Vec<PathBuf> = Vec::new();
        let mut saved: Vec<Prickling> = Vec::new();
        for raw in content.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some(rest) = line.strip_prefix("scan_root=") {
                let p = PathBuf::from(rest);
                if !scan_roots.iter().any(|r| r == &p) {
                    scan_roots.push(p);
                }
                continue;
            }
            if let Some(rest) = line.strip_prefix("saved=") {
                // Format: <path>|<display_name>
                let (path_str, name) = match rest.split_once('|') {
                    Some((p, n)) => (p, n.to_string()),
                    None => (rest, String::new()),
                };
                let path = PathBuf::from(path_str);
                let display_name = if name.is_empty() {
                    path.file_name()
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_else(|| path.to_string_lossy().into_owned())
                } else {
                    name
                };
                let prick = Prickling { path, display_name };
                if !saved.iter().any(|s| s.path == prick.path) {
                    saved.push(prick);
                }
            }
        }
        Self { scan_roots, saved }
    }

    /// Pure serialisation. Exposed for tests + symmetry with `parse`.
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        out.push_str("# GitCactus Pricklings — local project registry.\n");
        out.push_str("# Edited by the app; hand-edits are preserved on a best-effort basis.\n\n");
        for r in &self.scan_roots {
            out.push_str(&format!("scan_root={}\n", r.display()));
        }
        if !self.saved.is_empty() {
            out.push('\n');
        }
        for p in &self.saved {
            out.push_str(&format!(
                "saved={}|{}\n",
                p.path.display(),
                p.display_name
            ));
        }
        out
    }

    // ── Mutation helpers (dedup + canonicalize inline) ───────────

    /// Add an approved scan root (canonicalised, deduped).
    /// Returns true if it was actually added (false if already present
    /// or the path doesn't exist).
    pub fn add_scan_root<P: AsRef<Path>>(&mut self, p: P) -> bool {
        let canon = match fs::canonicalize(p.as_ref()) {
            Ok(c) => c,
            Err(_) => return false,
        };
        if self.scan_roots.iter().any(|r| r == &canon) {
            return false;
        }
        self.scan_roots.push(canon);
        true
    }

    /// Remove a scan root by index. No-op for out-of-range indices.
    pub fn remove_scan_root(&mut self, idx: usize) {
        if idx < self.scan_roots.len() {
            self.scan_roots.remove(idx);
        }
    }

    /// Save a Prickling. Returns true if newly added.
    pub fn save_prickling(&mut self, p: Prickling) -> bool {
        if self.saved.iter().any(|s| s.path == p.path) {
            return false;
        }
        self.saved.push(p);
        true
    }

    /// Remove a saved Prickling by index. No-op for out-of-range.
    pub fn remove_saved(&mut self, idx: usize) {
        if idx < self.saved.len() {
            self.saved.remove(idx);
        }
    }
}

/// Render a path with `$HOME` collapsed to `~`, for display in the UI.
///
/// Windows users keep their literal path (no tilde-shortening
/// convention on Windows shells), so the function is conditional.
fn shorten_home(p: &Path) -> String {
    #[cfg(not(windows))]
    {
        if let Ok(home) = std::env::var("HOME") {
            if !home.is_empty() {
                let home_path = Path::new(&home);
                if let Ok(rest) = p.strip_prefix(home_path) {
                    if rest.as_os_str().is_empty() {
                        return "~".to_string();
                    }
                    return format!("~/{}", rest.display());
                }
            }
        }
    }
    p.display().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_round_trip_preserves_order_and_dedups() {
        let source = "\
# comment
scan_root=/tmp/a
scan_root=/tmp/b
scan_root=/tmp/a

saved=/tmp/a/foo|Foo
saved=/tmp/b/bar|Bar
saved=/tmp/a/foo|Duplicate
";
        let store = PricklingsStore::parse(source);
        assert_eq!(
            store.scan_roots,
            vec![PathBuf::from("/tmp/a"), PathBuf::from("/tmp/b")]
        );
        assert_eq!(store.saved.len(), 2);
        assert_eq!(store.saved[0].display_name, "Foo");
        assert_eq!(store.saved[1].display_name, "Bar");
    }

    #[test]
    fn to_text_parses_back_into_same_store() {
        let mut s = PricklingsStore::default();
        s.scan_roots.push(PathBuf::from("/tmp/projects"));
        s.saved.push(Prickling {
            path: PathBuf::from("/tmp/projects/hello"),
            display_name: "hello".into(),
        });
        let text = s.to_text();
        let reparsed = PricklingsStore::parse(&text);
        assert_eq!(reparsed, s);
    }

    #[test]
    fn saved_without_display_name_falls_back_to_basename() {
        let s = PricklingsStore::parse("saved=/tmp/projects/hello\n");
        assert_eq!(s.saved.len(), 1);
        assert_eq!(s.saved[0].display_name, "hello");
    }

    #[test]
    fn save_prickling_dedupes() {
        let mut s = PricklingsStore::default();
        let p = Prickling {
            path: PathBuf::from("/tmp/x"),
            display_name: "X".into(),
        };
        assert!(s.save_prickling(p.clone()));
        assert!(!s.save_prickling(p));
        assert_eq!(s.saved.len(), 1);
    }

    #[test]
    fn remove_saved_is_bounds_safe() {
        let mut s = PricklingsStore::default();
        s.remove_saved(0); // empty list is a no-op
        s.saved.push(Prickling {
            path: PathBuf::from("/tmp/x"),
            display_name: "X".into(),
        });
        s.remove_saved(99); // out-of-range is a no-op
        assert_eq!(s.saved.len(), 1);
        s.remove_saved(0);
        assert!(s.saved.is_empty());
    }

    #[cfg(not(windows))]
    #[test]
    fn shorten_home_collapses_home_prefix() {
        // SAFETY: no other test mutates HOME.
        std::env::set_var("HOME", "/tmp/gctest");
        let long = Path::new("/tmp/gctest/projects/foo");
        assert_eq!(shorten_home(long), "~/projects/foo");
        let outside = Path::new("/var/log/foo");
        assert_eq!(shorten_home(outside), "/var/log/foo");
    }
}
