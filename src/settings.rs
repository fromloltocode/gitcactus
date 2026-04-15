//! Lightweight settings persistence.
//!
//! Stores user preferences in the platform config directory (see
//! [`crate::platform::config_dir`]) as simple `key=value` lines.
//! No serde dependency — just plain text.

use std::fs;
use std::path::PathBuf;

use crate::platform;
use crate::terminology::TermMode;
use crate::theme::Theme;

pub struct Settings {
    /// Skip the retro intro animation on startup.
    pub skip_intro: bool,
    /// Which terminology mode to use.
    pub term_mode: TermMode,
    /// Resolved theme (preset + any per-role overrides).
    pub theme: Theme,
    /// Whether to play overlay animations after successful
    /// Clone / Pull / Push operations. Defaults to on.
    pub animations: bool,
}

impl Settings {
    /// Load settings from disk, returning defaults if the file doesn't exist.
    pub fn load() -> Self {
        let defaults = || Self {
            skip_intro: false,
            term_mode: TermMode::Hybrid,
            theme: Theme::default(),
            animations: true,
        };

        let path = match Self::config_path() {
            Some(p) => p,
            None => return defaults(),
        };
        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => return defaults(),
        };
        let skip_intro = content.lines().any(|l| l.trim() == "skip_intro=true");
        let term_mode = content
            .lines()
            .find_map(|l| {
                let l = l.trim();
                l.strip_prefix("terminology=")
                    .and_then(TermMode::parse)
            })
            .unwrap_or(TermMode::Hybrid);
        // Theme parsing is its own best-effort: unknown preset or invalid
        // colors silently fall back to the default palette. The app never
        // fails to start because of theme config.
        let theme = Theme::from_config_lines(content.lines());
        // animations defaults to on; only the explicit "animations=off"
        // (or =false) turns them off. Any other value keeps the default.
        let animations = !content.lines().any(|l| {
            let t = l.trim();
            t == "animations=off" || t == "animations=false"
        });
        Self {
            skip_intro,
            term_mode,
            theme,
            animations,
        }
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

    /// Persist the terminology mode to the settings file.
    pub fn save_term_mode(mode: TermMode) {
        if let Some(path) = Self::config_path() {
            if let Some(dir) = path.parent() {
                let _ = fs::create_dir_all(dir);
            }
            let existing = fs::read_to_string(&path).unwrap_or_default();
            // Remove any existing terminology= line, then append the new one.
            let filtered: String = existing
                .lines()
                .filter(|l| !l.trim().starts_with("terminology="))
                .map(|l| format!("{l}\n"))
                .collect();
            let new = format!("{filtered}terminology={}\n", mode.as_str());
            let _ = fs::write(&path, new);
        }
    }

    fn config_path() -> Option<PathBuf> {
        platform::config_file("settings")
    }
}
