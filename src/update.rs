//! Update-awareness module.
//!
//! Provides version info and mock update-check results. Real network-based
//! update checking and self-update will be added in a future release.

/// Current version of GitCactus, kept in sync with Cargo.toml.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Information returned by an update check.
pub struct UpdateInfo {
    pub current: &'static str,
    pub latest: String,
    pub is_up_to_date: bool,
    pub changelog: Vec<ChangelogEntry>,
}

/// A single entry in the changelog.
pub struct ChangelogEntry {
    pub version: String,
    pub summary: String,
}

/// Check for updates. Currently returns mock data.
///
/// In a future release this will query a remote endpoint (e.g. GitHub
/// Releases API) to determine the real latest version.
pub fn check_for_updates() -> UpdateInfo {
    let current = VERSION;

    // Mock: pretend a newer version exists so the UI has something to show.
    let latest = "0.2.0".to_string();
    let is_up_to_date = current == latest;

    let changelog = vec![
        ChangelogEntry {
            version: "0.2.0".into(),
            summary: "Interactive staging, commit workflow, branch management".into(),
        },
        ChangelogEntry {
            version: "0.1.1".into(),
            summary: "Bug fixes and improved status display".into(),
        },
        ChangelogEntry {
            version: "0.1.0".into(),
            summary: "Initial release — TUI skeleton, status screen, cactus mascot".into(),
        },
    ];

    UpdateInfo {
        current,
        latest,
        is_up_to_date,
        changelog,
    }
}
