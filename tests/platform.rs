//! Tests for cross-platform helpers.
//!
//! These run on whatever platform CI is targeting — the `#[cfg]`
//! gates below exercise the Windows-specific branches only when the
//! tests are built for Windows. The platform-agnostic assertions
//! (list is non-empty, display strings are non-empty, path joins
//! `gitcactus/<name>`) run everywhere.

use gitcactus::platform;

// ── Platform-agnostic invariants ─────────────────────────────────────

#[test]
fn editor_candidates_is_non_empty() {
    let candidates = platform::default_editor_candidates();
    assert!(!candidates.is_empty(), "fallback list must never be empty");
    for c in candidates {
        assert!(!c.is_empty(), "candidate names must not be blank");
        // Don't allow whitespace in a candidate — we look them up on PATH
        // as bare executables.
        assert!(
            !c.contains(char::is_whitespace),
            "candidate {c:?} contains whitespace"
        );
    }
}

#[test]
fn editor_suggestion_mentions_editor_env_var() {
    let s = platform::editor_suggestion();
    assert!(!s.is_empty());
    // The hint refers the user at their platform's env-var syntax.
    assert!(s.to_uppercase().contains("EDITOR"));
}

#[test]
fn config_dir_display_for_known_names_is_non_empty() {
    for name in ["settings", "profile", "anything-else"] {
        let d = platform::config_dir_display(name);
        assert!(!d.is_empty(), "display for {name} must not be blank");
        assert!(
            d.to_lowercase().contains("gitcactus"),
            "display for {name} missing 'gitcactus' token: {d}"
        );
    }
}

#[test]
fn config_file_path_ends_in_requested_name() {
    // When the platform can resolve a home-ish dir, the returned
    // path must end with the file name we asked for. If we can't
    // resolve one, `None` is the contract — also valid.
    if let Some(p) = platform::config_file("settings") {
        assert_eq!(p.file_name().and_then(|s| s.to_str()), Some("settings"));
    }
    if let Some(p) = platform::config_file("profile") {
        assert_eq!(p.file_name().and_then(|s| s.to_str()), Some("profile"));
    }
}

#[test]
fn config_file_path_has_gitcactus_component() {
    if let Some(p) = platform::config_file("settings") {
        let found = p
            .components()
            .any(|c| c.as_os_str().to_string_lossy() == "gitcactus");
        assert!(
            found,
            "settings path must contain a 'gitcactus' segment: {p:?}"
        );
    }
}

// ── Unix-specific assertions ─────────────────────────────────────────

#[cfg(not(windows))]
mod unix_only {
    use super::*;

    #[test]
    fn unix_fallback_includes_vim_family() {
        let candidates = platform::default_editor_candidates();
        // We should always at least try a vi-family editor on Unix.
        assert!(
            candidates.iter().any(|c| *c == "vim" || *c == "vi"),
            "Unix fallback should include vi/vim: {candidates:?}"
        );
    }

    #[test]
    fn unix_settings_display_uses_home_tilde() {
        let d = platform::config_dir_display("settings");
        assert!(
            d.starts_with("~"),
            "Unix settings display should start with ~: {d}"
        );
        assert!(
            d.contains(".config"),
            "Unix settings display should mention .config: {d}"
        );
    }

    #[test]
    fn unix_config_file_follows_xdg_layout() {
        // Force HOME to a known value so the assertion is stable even
        // on machines where the real home has unusual punctuation.
        // SAFETY: we restore the previous value on drop.
        struct EnvGuard(&'static str, Option<String>);
        impl EnvGuard {
            fn set(key: &'static str, val: &str) -> Self {
                let prev = std::env::var(key).ok();
                std::env::set_var(key, val);
                Self(key, prev)
            }
        }
        impl Drop for EnvGuard {
            fn drop(&mut self) {
                match &self.1 {
                    Some(v) => std::env::set_var(self.0, v),
                    None => std::env::remove_var(self.0),
                }
            }
        }

        let _guard = EnvGuard::set("HOME", "/tmp/gctest-home");
        let p = platform::config_file("settings")
            .expect("HOME is set, path should resolve");
        assert_eq!(
            p,
            std::path::PathBuf::from("/tmp/gctest-home")
                .join(".config")
                .join("gitcactus")
                .join("settings")
        );
    }
}

// ── Windows-specific assertions ──────────────────────────────────────

#[cfg(windows)]
mod windows_only {
    use super::*;

    #[test]
    fn windows_fallback_includes_notepad() {
        let candidates = platform::default_editor_candidates();
        assert!(
            candidates.contains(&"notepad"),
            "Windows fallback must include notepad (always present): {candidates:?}"
        );
    }

    #[test]
    fn windows_suggestion_mentions_notepad_or_percent() {
        let s = platform::editor_suggestion();
        let has_notepad = s.to_lowercase().contains("notepad");
        let has_percent_editor = s.contains("%EDITOR%");
        assert!(
            has_notepad || has_percent_editor,
            "Windows hint should reference %EDITOR% or notepad: {s}"
        );
    }

    #[test]
    fn windows_settings_display_uses_percent_appdata() {
        let d = platform::config_dir_display("settings");
        assert!(
            d.to_uppercase().contains("%APPDATA%"),
            "Windows settings display should reference %APPDATA%: {d}"
        );
    }

    #[test]
    fn windows_config_file_uses_appdata_when_set() {
        struct EnvGuard(&'static str, Option<String>);
        impl EnvGuard {
            fn set(key: &'static str, val: &str) -> Self {
                let prev = std::env::var(key).ok();
                std::env::set_var(key, val);
                Self(key, prev)
            }
        }
        impl Drop for EnvGuard {
            fn drop(&mut self) {
                match &self.1 {
                    Some(v) => std::env::set_var(self.0, v),
                    None => std::env::remove_var(self.0),
                }
            }
        }

        let _guard = EnvGuard::set("APPDATA", "C:\\Users\\test\\AppData\\Roaming");
        let p = platform::config_file("settings")
            .expect("APPDATA is set, path should resolve");
        assert_eq!(
            p,
            std::path::PathBuf::from("C:\\Users\\test\\AppData\\Roaming")
                .join("gitcactus")
                .join("settings")
        );
    }
}
