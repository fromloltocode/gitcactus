//! Lightweight settings persistence.
//!
//! Stores user preferences in `~/.config/gitcactus/settings` as simple
//! key=value lines. No serde dependency — just plain text.

use std::fs;
use std::path::PathBuf;

pub struct Settings {
    /// Skip the retro intro animation on startup.
    pub skip_intro: bool,
}

impl Settings {
    /// Load settings from disk, returning defaults if the file doesn't exist.
    pub fn load() -> Self {
        let path = match Self::config_path() {
            Some(p) => p,
            None => return Self { skip_intro: false },
        };
        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => return Self { skip_intro: false },
        };
        let skip_intro = content.lines().any(|l| l.trim() == "skip_intro=true");
        Self { skip_intro }
    }

    /// Mark the intro as seen so subsequent launches skip it.
    pub fn mark_intro_seen() {
        if let Some(path) = Self::config_path() {
            if let Some(dir) = path.parent() {
                let _ = fs::create_dir_all(dir);
            }
            // Read existing settings and add skip_intro if not present.
            let existing = fs::read_to_string(&path).unwrap_or_default();
            if !existing.contains("skip_intro=true") {
                let new = format!("{existing}skip_intro=true\n");
                let _ = fs::write(&path, new);
            }
        }
    }

    fn config_path() -> Option<PathBuf> {
        let home = std::env::var("HOME").ok()?;
        Some(
            PathBuf::from(home)
                .join(".config")
                .join("gitcactus")
                .join("settings"),
        )
    }
}
