//! Launch the user's preferred editor for a given file.
//!
//! Resolution order:
//!   1. `$EDITOR` environment variable (may include arguments)
//!   2. Fallback to common editors found on `PATH`. The list is
//!      platform-specific — see
//!      [`crate::platform::default_editor_candidates`]:
//!      - Unix: `nvim`, `vim`, `vi`, `nano`, `code`, `emacs`
//!      - Windows: `code`, `nvim`, `vim`, `notepad`
//!
//! Execution is blocking — the caller is expected to suspend the TUI
//! before invoking `open_in_editor` and restore it afterwards so the
//! editor has full control of the terminal while it's running.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::platform;

/// Result of an "open in editor" attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorResult {
    /// Editor was launched and exited normally.
    Ok,
    /// File does not exist or is not accessible.
    FileNotFound(String),
    /// No editor found in $EDITOR or on PATH.
    NoEditor,
    /// File appears to be binary — we refuse to open it.
    BinaryFile(String),
    /// Editor process failed to launch or exited with error.
    LaunchFailed(String),
}

/// Find the editor command to use, or `None` if nothing is available.
///
/// Returns the raw command string — may contain spaces (e.g. `"code --wait"`).
pub fn find_editor() -> Option<String> {
    if let Ok(ed) = std::env::var("EDITOR") {
        let trimmed = ed.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    for candidate in platform::default_editor_candidates() {
        if is_on_path(candidate) {
            return Some((*candidate).to_string());
        }
    }
    None
}

/// Check whether a binary exists on `$PATH`.
///
/// On Windows, also tries common executable extensions
/// (`.exe`, `.cmd`, `.bat`) when `name` has no extension. This matches
/// what the OS loader does when the user runs a bare name like
/// `notepad` rather than `notepad.exe`.
fn is_on_path(name: &str) -> bool {
    let path = match std::env::var_os("PATH") {
        Some(p) => p,
        None => return false,
    };

    #[cfg(windows)]
    let candidates: Vec<String> = {
        if std::path::Path::new(name).extension().is_some() {
            vec![name.to_string()]
        } else {
            ["", ".exe", ".cmd", ".bat"]
                .iter()
                .map(|ext| format!("{name}{ext}"))
                .collect()
        }
    };
    #[cfg(not(windows))]
    let candidates: Vec<String> = vec![name.to_string()];

    std::env::split_paths(&path).any(|dir| {
        candidates.iter().any(|c| {
            let full = dir.join(c);
            full.exists() && full.is_file()
        })
    })
}

/// Open `file_path` in the user's editor. Blocking — run after the TUI
/// is suspended.
pub fn open_in_editor(file_path: &str) -> EditorResult {
    let path = PathBuf::from(file_path);

    // Validate file exists.
    if !path.exists() {
        return EditorResult::FileNotFound(file_path.to_string());
    }
    if !path.is_file() {
        return EditorResult::FileNotFound(file_path.to_string());
    }

    // Guard against accidentally opening binary files in a text editor.
    if is_binary(&path) {
        return EditorResult::BinaryFile(file_path.to_string());
    }

    let editor = match find_editor() {
        Some(e) => e,
        None => return EditorResult::NoEditor,
    };

    // Split the editor string so things like "code --wait" work.
    let parts: Vec<&str> = editor.split_whitespace().collect();
    let program = match parts.first() {
        Some(p) => *p,
        None => return EditorResult::NoEditor,
    };
    let extra_args: Vec<&str> = parts.iter().skip(1).copied().collect();

    match Command::new(program)
        .args(&extra_args)
        .arg(file_path)
        .status()
    {
        Ok(status) => {
            if status.success() {
                EditorResult::Ok
            } else {
                EditorResult::LaunchFailed(format!(
                    "Editor exited with status {}",
                    status.code().unwrap_or(-1)
                ))
            }
        }
        Err(e) => EditorResult::LaunchFailed(format!("Could not launch {program}: {e}")),
    }
}

/// Best-effort binary detection: read the first 8 KiB and look for null bytes.
fn is_binary(path: &Path) -> bool {
    use std::io::Read;
    let mut file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return false, // if we can't read, let the editor try
    };
    let mut buf = [0u8; 8192];
    let n = match file.read(&mut buf) {
        Ok(n) => n,
        Err(_) => return false,
    };
    buf[..n].contains(&0)
}

/// Human-readable message for an [`EditorResult`] — used to populate
/// the app-wide editor message after an attempt.
pub fn result_message(r: &EditorResult) -> (String, bool) {
    match r {
        EditorResult::Ok => ("Returned from editor.".into(), true),
        EditorResult::FileNotFound(p) => (format!("File not found: {p}"), false),
        EditorResult::NoEditor => (
            format!("No editor found. {}", platform::editor_suggestion()),
            false,
        ),
        EditorResult::BinaryFile(p) => (
            format!("Refusing to open binary file: {p}"),
            false,
        ),
        EditorResult::LaunchFailed(msg) => (format!("Editor launch failed: {msg}"), false),
    }
}
