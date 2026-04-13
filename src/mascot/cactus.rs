//! ASCII art and personality for the GitCactus mascot.

/// The main cactus art used on the title screen.
pub const CACTUS_LARGE: &str = "\
      _  _
     | || |
    _| || |_
   |   __   |
   |  |  |  |
   |  |  |  |
  _|  |  |  |_
 |    |  |    |
 |    |  |    |
  \\   |  |  /
   \\__|  |_/
      |  |
      |  |
    __|  |__
   /        \\
  /____  ____\\
       ||
       ||";

/// Compact cactus for the menu sidebar.
pub const CACTUS_SMALL: &str = "\
    _|_
   | . |
  _| . |_
 |  \\_/  |
  \\  |  /
   \\_|_/
    | |
   _|_|_
  /_____\\";

/// A short greeting from the cactus for the title screen.
pub const TITLE_TAGLINE: &str = "Your prickly-but-friendly Git companion.";

/// Short contextual tips the cactus can show.
pub const TIPS: &[&str] = &[
    "GitCactus will never change your repo without asking first.",
    "Use arrow keys or j/k to navigate, Enter to select.",
    "Press q to quit, Esc to go back.",
    "Commit early, commit often!",
    "Branches are cheap. Don't be afraid to experiment.",
];

/// Tip shown on the update screen sidebar.
pub fn update_tip(is_up_to_date: bool) -> &'static str {
    if is_up_to_date {
        "You're running the latest version. No thorns out of place!"
    } else {
        "A new version is available! Updating keeps your tools sharp and your workflow smooth."
    }
}

/// Tip shown on the commit screen sidebar.
pub fn commit_tip(staged_count: usize, has_message: bool) -> &'static str {
    if staged_count == 0 {
        "No files staged yet! Head to Stage Changes first to pick what goes in your commit."
    } else if !has_message {
        "Write a short message describing what changed and why. Good messages help future-you!"
    } else {
        "Looking good! Press Enter to review and confirm your commit."
    }
}

/// Tip shown on the stage screen sidebar.
pub fn stage_tip(selected: usize, total: usize) -> &'static str {
    if total == 0 {
        "Nothing to stage right now. All clean!"
    } else if selected == 0 {
        "Use Space to pick which files to stage. Staging tells Git: \"include this in my next commit.\""
    } else {
        "Good picks! Press Enter when you're ready, and I'll ask you to confirm before anything changes."
    }
}

/// Return a contextual tip based on the current repo state.
///
/// The three counts are (staged, modified, untracked). Pass `None` for any
/// value that could not be determined (non-repo case).
pub fn status_tip(staged: Option<usize>, modified: Option<usize>, untracked: Option<usize>) -> &'static str {
    let s = staged.unwrap_or(0);
    let m = modified.unwrap_or(0);
    let u = untracked.unwrap_or(0);

    if s == 0 && m == 0 && u == 0 {
        "Looking clean! Nothing to do here. Go grab a coffee."
    } else if s > 0 {
        "You have staged changes! When you're ready, commit them to save a snapshot."
    } else if m > 0 && u == 0 {
        "Files have been modified. Stage the ones you want to include in your next commit."
    } else if m == 0 && u > 0 {
        "New files detected! Stage them so Git starts tracking their history."
    } else {
        "You have modified and untracked files. Stage the changes you want to keep."
    }
}
