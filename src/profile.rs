//! Local-first progression profile.
//!
//! Persisted as plain key=value text at `~/.config/gitcactus/profile`.
//! Kept separate from `settings` because it is gameplay data, not user
//! preferences, and so the two files can be backed up / wiped / reset
//! independently.
//!
//! # Local-first philosophy
//!
//! - The local profile is the **source of truth** for this phase.
//! - All data lives in the user's home directory. No network calls.
//! - There is **no account, no auth, no telemetry** in this build.
//! - A future opt-in sync might allow exporting the profile to connect
//!   it to a community service, but that is explicitly out of scope
//!   here and must remain a user choice.
//! - The file format is intentionally human-readable so users can
//!   inspect, edit, back up, or delete their progression freely.

use std::fs;
use std::path::PathBuf;

/// Profile file format version. Increment if the stored shape changes
/// in a way old loaders cannot tolerate.
const PROFILE_VERSION: u32 = 1;

/// Running counts of meaningful actions the user has taken.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Stats {
    pub commits_created: u32,
    pub files_staged: u32,
    pub branches_created: u32,
    pub branches_switched: u32,
    pub combos_completed: u32,
    pub rebases_completed: u32,
}

/// The local progression profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    pub version: u32,
    pub xp: u32,
    pub level: u32,
    pub stats: Stats,
    /// Cosmetic / flair unlocks the user has earned.
    /// Stored as sorted, de-duplicated keys.
    pub unlocks: Vec<String>,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            version: PROFILE_VERSION,
            xp: 0,
            level: 1,
            stats: Stats::default(),
            unlocks: Vec::new(),
        }
    }
}

impl Profile {
    /// Load the profile from disk, or return a fresh default profile.
    ///
    /// Malformed values silently fall back — the app must never crash
    /// because of user-edited profile files.
    pub fn load() -> Self {
        let path = match Self::profile_path() {
            Some(p) => p,
            None => return Self::default(),
        };
        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => return Self::default(),
        };
        Self::from_text(&content)
    }

    /// Parse from a text blob. Exposed for tests.
    pub fn from_text(content: &str) -> Self {
        let mut p = Self::default();
        for raw in content.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (k, v) = match line.split_once('=') {
                Some(kv) => kv,
                None => continue,
            };
            let k = k.trim();
            let v = v.trim();
            match k {
                "version" => {
                    if let Ok(n) = v.parse() {
                        p.version = n;
                    }
                }
                "xp" => {
                    if let Ok(n) = v.parse() {
                        p.xp = n;
                    }
                }
                "level" => {
                    if let Ok(n) = v.parse::<u32>() {
                        p.level = n.max(1);
                    }
                }
                "commits_created" => parse_u32_into(v, &mut p.stats.commits_created),
                "files_staged" => parse_u32_into(v, &mut p.stats.files_staged),
                "branches_created" => parse_u32_into(v, &mut p.stats.branches_created),
                "branches_switched" => parse_u32_into(v, &mut p.stats.branches_switched),
                "combos_completed" => parse_u32_into(v, &mut p.stats.combos_completed),
                "rebases_completed" => parse_u32_into(v, &mut p.stats.rebases_completed),
                "unlocks" => {
                    p.unlocks = v
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    p.unlocks.sort();
                    p.unlocks.dedup();
                }
                _ => {} // unknown key — ignore
            }
        }
        p
    }

    /// Serialise to the on-disk text format.
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        out.push_str("# GitCactus local progression profile.\n");
        out.push_str("# Edit at your own risk — invalid values are ignored on load.\n");
        out.push_str(&format!("version={}\n", self.version));
        out.push_str(&format!("xp={}\n", self.xp));
        out.push_str(&format!("level={}\n", self.level));
        out.push_str(&format!(
            "commits_created={}\n",
            self.stats.commits_created
        ));
        out.push_str(&format!("files_staged={}\n", self.stats.files_staged));
        out.push_str(&format!(
            "branches_created={}\n",
            self.stats.branches_created
        ));
        out.push_str(&format!(
            "branches_switched={}\n",
            self.stats.branches_switched
        ));
        out.push_str(&format!(
            "combos_completed={}\n",
            self.stats.combos_completed
        ));
        out.push_str(&format!(
            "rebases_completed={}\n",
            self.stats.rebases_completed
        ));
        out.push_str(&format!("unlocks={}\n", self.unlocks.join(",")));
        out
    }

    /// Save to disk. Best-effort; errors are swallowed.
    pub fn save(&self) {
        if let Some(path) = Self::profile_path() {
            if let Some(dir) = path.parent() {
                let _ = fs::create_dir_all(dir);
            }
            let _ = fs::write(&path, self.to_text());
        }
    }

    fn profile_path() -> Option<PathBuf> {
        let home = std::env::var("HOME").ok()?;
        Some(
            PathBuf::from(home)
                .join(".config")
                .join("gitcactus")
                .join("profile"),
        )
    }
}

fn parse_u32_into(v: &str, slot: &mut u32) {
    if let Ok(n) = v.parse() {
        *slot = n;
    }
}
