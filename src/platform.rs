//! Cross-platform helpers.
//!
//! Everything in this module hides a platform difference from the rest
//! of the codebase. Two callers that previously used `$HOME` and the
//! hard-coded Unix editor list now go through the same helpers here,
//! and any future platform-divergent logic should land here too rather
//! than growing `#[cfg(…)]` blocks elsewhere.

use std::path::PathBuf;

/// Name of our config directory, shared across platforms.
const APP_DIRNAME: &str = "gitcactus";

/// Return the directory where GitCactus stores its config (settings,
/// profile, etc.).
///
/// - **Windows**: `%APPDATA%\gitcactus\` (Roaming). Falls back to
///   `%USERPROFILE%\AppData\Roaming\gitcactus\` if `APPDATA` is unset,
///   and finally to `%USERPROFILE%\gitcactus\` as a last resort.
/// - **Unix-like (macOS, Linux, BSD)**: `$HOME/.config/gitcactus/`,
///   matching the XDG-ish convention the app has used since 0.1.
///
/// Returns `None` only when we genuinely can't determine a writable
/// home — the app degrades to "no persistence" in that case, same as
/// it did on Unix before this module existed.
pub fn config_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            if !appdata.is_empty() {
                return Some(PathBuf::from(appdata).join(APP_DIRNAME));
            }
        }
        if let Ok(userprofile) = std::env::var("USERPROFILE") {
            if !userprofile.is_empty() {
                return Some(
                    PathBuf::from(userprofile)
                        .join("AppData")
                        .join("Roaming")
                        .join(APP_DIRNAME),
                );
            }
        }
        None
    }

    #[cfg(not(windows))]
    {
        let home = std::env::var("HOME").ok()?;
        if home.is_empty() {
            return None;
        }
        Some(PathBuf::from(home).join(".config").join(APP_DIRNAME))
    }
}

/// Convenience: a child of [`config_dir`] — e.g. `config_file("settings")`.
pub fn config_file(name: &str) -> Option<PathBuf> {
    config_dir().map(|d| d.join(name))
}

/// A human-readable path string for help / settings-screen hints.
///
/// We never resolve `%APPDATA%` at runtime for display — a literal
/// `%APPDATA%\gitcactus\<file>` is more useful to a Windows user than
/// an absolute path with their username embedded.
pub fn config_dir_display(name: &str) -> &'static str {
    #[cfg(windows)]
    {
        match name {
            "settings" => "%APPDATA%\\gitcactus\\settings",
            "profile" => "%APPDATA%\\gitcactus\\profile",
            _ => "%APPDATA%\\gitcactus\\",
        }
    }
    #[cfg(not(windows))]
    {
        match name {
            "settings" => "~/.config/gitcactus/settings",
            "profile" => "~/.config/gitcactus/profile",
            _ => "~/.config/gitcactus/",
        }
    }
}

/// Fallback editors searched on `$PATH` when `$EDITOR` is unset.
///
/// Ordered by "most likely to be present and produce a usable editing
/// experience" on each platform:
/// - **Windows**: `notepad` is universally available, `code` picks up
///   VS Code if installed, then the usual dev editors.
/// - **Unix-like**: the previous list, preserved so existing users
///   see no change in behaviour.
pub fn default_editor_candidates() -> &'static [&'static str] {
    #[cfg(windows)]
    {
        &["code", "nvim", "vim", "notepad"]
    }
    #[cfg(not(windows))]
    {
        &["nvim", "vim", "vi", "nano", "code", "emacs"]
    }
}

/// A short "try one of these" hint shown in the "no editor found"
/// error so the suggestion matches the actual platform defaults.
pub fn editor_suggestion() -> &'static str {
    #[cfg(windows)]
    {
        "Set %EDITOR% or install notepad/code/vim."
    }
    #[cfg(not(windows))]
    {
        "Set $EDITOR or install nvim/vim/nano/code."
    }
}
