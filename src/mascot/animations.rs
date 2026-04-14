//! Cactus-themed animations for Clone / Pull / Push.
//!
//! These animations are **visual only** — they never modify git state
//! and never interfere with stdout/stderr. They run as an overlay on
//! top of whichever screen triggered them, advanced one frame per idle
//! tick of the main event loop (same pattern as the intro animation).
//!
//! Each animation has two phases:
//!
//! 1. **Playing** — frame-by-frame ASCII art showing data flow.
//!    Any key press skips to the final frame + teaching phase.
//! 2. **Teaching** — the final frame plus a 3–5 line explanation
//!    panel that reinforces the git mental model. Any key dismisses.
//!
//! If animations are disabled in settings, [`play`] is a no-op —
//! the caller's success path simply carries on.

use crate::terminology::Terms;

// ── Kind / phase / state ─────────────────────────────────────────────

/// Which operation the animation illustrates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationKind {
    /// A fresh local repo grown from a remote — "Cactus Propagation".
    Clone,
    /// Remote commits arriving locally — "Rope Pull".
    Pull,
    /// Local commits shot out to a remote — "Laser Transmission".
    Push,
}

/// Which phase of the animation we're in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationPhase {
    /// Frames are advancing.
    Playing,
    /// Final frame is held and the teaching panel is visible.
    Teaching,
}

/// Transient state for an active animation.
///
/// Stored on `App` in an `Option` — `None` means no animation is
/// running and the app renders/interacts normally.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnimationState {
    pub kind: AnimationKind,
    pub phase: AnimationPhase,
    /// Current playback frame. Clamped to `frames().len() - 1`.
    pub frame: usize,
}

impl AnimationState {
    pub fn new(kind: AnimationKind) -> Self {
        Self {
            kind,
            phase: AnimationPhase::Playing,
            frame: 0,
        }
    }

    /// Advance one frame. Transitions to [`AnimationPhase::Teaching`]
    /// as soon as the last frame is reached, so the final art is
    /// displayed *with* the teaching panel rather than one idle tick
    /// before it. Returns `true` once the animation has transitioned
    /// to teaching.
    pub fn tick(&mut self) -> bool {
        if self.phase == AnimationPhase::Teaching {
            return true;
        }
        let last = self.total_frames().saturating_sub(1);
        if self.frame < last {
            self.frame += 1;
        }
        if self.frame >= last {
            self.phase = AnimationPhase::Teaching;
            return true;
        }
        false
    }

    /// Jump straight to the teaching panel. Used when the user
    /// presses any key during playback.
    pub fn skip_to_teaching(&mut self) {
        self.frame = self.total_frames().saturating_sub(1);
        self.phase = AnimationPhase::Teaching;
    }

    pub fn total_frames(&self) -> usize {
        frames(self.kind).len()
    }

    /// The ASCII art for the current frame.
    pub fn current_art(&self) -> &'static str {
        let fs = frames(self.kind);
        let i = self.frame.min(fs.len().saturating_sub(1));
        fs[i]
    }

    /// The teaching-panel lines for this animation, terminology-aware.
    /// Always 3–5 lines.
    pub fn teaching_lines(&self, terms: &Terms) -> Vec<&'static str> {
        teaching_lines(self.kind, terms)
    }

    /// Screen-title label shown above the animation (terminology-aware).
    pub fn title(&self, terms: &Terms) -> &'static str {
        title(self.kind, terms)
    }
}

// ── Public entry points ──────────────────────────────────────────────

/// Start an animation if animations are enabled.
///
/// `dest` is typically `&mut app.animation`. When `enabled` is false,
/// this is a no-op so callers can always invoke it unconditionally
/// from the effect-handling layer.
///
/// These entry points are public library API — they are called from
/// tests today and from the Clone / Pull / Push effect handlers once
/// those operations land. The `#[allow(dead_code)]` keeps the binary
/// crate happy in the meantime without compromising availability.
#[allow(dead_code)]
pub fn play(dest: &mut Option<AnimationState>, kind: AnimationKind, enabled: bool) {
    if enabled {
        *dest = Some(AnimationState::new(kind));
    }
}

/// Convenience wrapper — `play(_, Clone, enabled)`.
#[allow(dead_code)]
pub fn play_clone_animation(dest: &mut Option<AnimationState>, enabled: bool) {
    play(dest, AnimationKind::Clone, enabled);
}

/// Convenience wrapper — `play(_, Pull, enabled)`.
#[allow(dead_code)]
pub fn play_pull_animation(dest: &mut Option<AnimationState>, enabled: bool) {
    play(dest, AnimationKind::Pull, enabled);
}

/// Convenience wrapper — `play(_, Push, enabled)`.
#[allow(dead_code)]
pub fn play_push_animation(dest: &mut Option<AnimationState>, enabled: bool) {
    play(dest, AnimationKind::Push, enabled);
}

// ── Frame data ───────────────────────────────────────────────────────
//
// All frames are exactly 8 lines tall. Width varies (consuming screens
// centre them horizontally). Each animation runs in 6–8 frames; at the
// 120ms idle-poll rate used during animations, the full playback lands
// under 1 s before moving into the teaching panel.

/// Return the frame array for a given animation kind.
fn frames(kind: AnimationKind) -> &'static [&'static str] {
    match kind {
        AnimationKind::Clone => CLONE_FRAMES,
        AnimationKind::Pull => PULL_FRAMES,
        AnimationKind::Push => PUSH_FRAMES,
    }
}

/// Clone — a new cactus grows from a remote source.
const CLONE_FRAMES: &[&str] = &[
    // 0 — remote stands alone
    "\
     REMOTE                                              LOCAL
       ▒▒
     ▒▒▒▒▒▒                                             . . . .
       ▒▒                                              empty soil
      ██████
     [  git  ]                                              ·
                                                            ·",
    // 1 — signal leaves remote
    "\
     REMOTE                                              LOCAL
       ▒▒
     ▒▒▒▒▒▒   ·                                         . . . .
       ▒▒                                              empty soil
      ██████
     [  git  ]                                              ·
                                                            ·",
    // 2 — signal travels
    "\
     REMOTE                                              LOCAL
       ▒▒
     ▒▒▒▒▒▒          · · ·                              . . . .
       ▒▒                                              empty soil
      ██████
     [  git  ]                                              ·
                                                            ·",
    // 3 — signal closer to local
    "\
     REMOTE                                              LOCAL
       ▒▒
     ▒▒▒▒▒▒                      · · · · ·              . . . .
       ▒▒                                              empty soil
      ██████
     [  git  ]                                              ·
                                                            ·",
    // 4 — seed lands
    "\
     REMOTE                                              LOCAL
       ▒▒
     ▒▒▒▒▒▒                                ·  ·  ·
       ▒▒                                                   ·
      ██████                                                v
     [  git  ]                                          (seed)
                                                            ·",
    // 5 — sprout
    "\
     REMOTE                                              LOCAL
       ▒▒
     ▒▒▒▒▒▒                                                ▒▒
       ▒▒                                                  ▒▒
      ██████                                              ████
     [  git  ]                                         (sprout)
                                                            ·",
    // 6 — full clone
    "\
     REMOTE                                              LOCAL
       ▒▒                                                  ▒▒
     ▒▒▒▒▒▒                                             ▒▒▒▒▒▒
       ▒▒                                                  ▒▒
      ██████                                              ██████
     [  git  ]                                        [ origin ↔ ]
                                                         cloned!",
];

/// Pull — rope pulls remote commits into the local cactus.
const PULL_FRAMES: &[&str] = &[
    // 0 — both cacti, no rope
    "\
     REMOTE                                              LOCAL
       ▒▒                                                  ▒▒
     ▒▒▒▒▒▒                                             ▒▒▒▒▒▒
       ▒▒                                                  ▒▒
      ██████                                              ██████

     origin/main                                       main",
    // 1 — rope extends from local
    "\
     REMOTE                                              LOCAL
       ▒▒                                                  ▒▒
     ▒▒▒▒▒▒                                      ·───── ▒▒▒▒▒▒
       ▒▒                                                  ▒▒
      ██████                                              ██████

     origin/main                                       main",
    // 2 — rope crosses further
    "\
     REMOTE                                              LOCAL
       ▒▒                                                  ▒▒
     ▒▒▒▒▒▒                           ·──────────────── ▒▒▒▒▒▒
       ▒▒                                                  ▒▒
      ██████                                              ██████

     origin/main                                       main",
    // 3 — rope connected
    "\
     REMOTE                                              LOCAL
       ▒▒                                                  ▒▒
     ▒▒▒▒▒▒ ·═══════════════════════════════════════════ ▒▒▒▒▒▒
       ▒▒                                                  ▒▒
      ██████                                              ██████

     origin/main                                       main",
    // 4 — commits traveling leftward to rightward (remote → local)
    "\
     REMOTE                                              LOCAL
       ▒▒                                                  ▒▒
     ▒▒▒▒▒▒ ═══════ ◄ ◄ ◄ ◄ ════════════════════════════ ▒▒▒▒▒▒
       ▒▒               commits                            ▒▒
      ██████                                              ██████

     origin/main                                       main",
    // 5 — commits arriving at local
    "\
     REMOTE                                              LOCAL
       ▒▒                                                  ▒▒
     ▒▒▒▒▒▒ ══════════════════════════════════ ◄ ◄ ◄ ◄  ▒▒▒▒▒▒
       ▒▒                                     commits      ▒▒
      ██████                                              ██████

     origin/main                                       main (ahead!)",
    // 6 — local updated
    "\
     REMOTE                                              LOCAL
       ▒▒                                                  ▒▒
     ▒▒▒▒▒▒                                             ▒▒▒▒▒▒
       ▒▒                                                  ▒▒
      ██████                                              ██████

     origin/main                                       main — up to date",
];

/// Push — local cactus fires a laser of commits at the remote.
const PUSH_FRAMES: &[&str] = &[
    // 0 — both cacti, local charging
    "\
     LOCAL                                              REMOTE
       ▒▒                                                  ▒▒
     ▒▒▒▒▒▒ *                                           ▒▒▒▒▒▒
       ▒▒                                                  ▒▒
      ██████                                              ██████

     main (ahead)                                      origin/main",
    // 1 — laser starts
    "\
     LOCAL                                              REMOTE
       ▒▒                                                  ▒▒
     ▒▒▒▒▒▒ ═══>                                        ▒▒▒▒▒▒
       ▒▒                                                  ▒▒
      ██████                                              ██████

     main                                              origin/main",
    // 2 — laser extends
    "\
     LOCAL                                              REMOTE
       ▒▒                                                  ▒▒
     ▒▒▒▒▒▒ ══════════════>                             ▒▒▒▒▒▒
       ▒▒                                                  ▒▒
      ██████                                              ██████

     main                                              origin/main",
    // 3 — laser nearly there
    "\
     LOCAL                                              REMOTE
       ▒▒                                                  ▒▒
     ▒▒▒▒▒▒ ════════════════════════════════════>       ▒▒▒▒▒▒
       ▒▒                                                  ▒▒
      ██████                                              ██████

     main                                              origin/main",
    // 4 — laser connects
    "\
     LOCAL                                              REMOTE
       ▒▒                                                  ▒▒
     ▒▒▒▒▒▒ ════════════════════════════════════════════ ▒▒▒▒▒▒
       ▒▒                                        commits    ▒▒
      ██████                                              ██████ *

     main                                              origin/main",
    // 5 — impact, commits received
    "\
     LOCAL                                              REMOTE
       ▒▒                                                ✦ ▒▒
     ▒▒▒▒▒▒                                             ▒▒▒▒▒▒
       ▒▒                                                  ▒▒
      ██████                                              ██████ *

     main                                              origin/main +updated",
    // 6 — done
    "\
     LOCAL                                              REMOTE
       ▒▒                                                  ▒▒
     ▒▒▒▒▒▒                                             ▒▒▒▒▒▒
       ▒▒                                                  ▒▒
      ██████                                              ██████

     main — pushed!                                    origin/main",
];

// ── Terminology ──────────────────────────────────────────────────────

fn title(kind: AnimationKind, terms: &Terms) -> &'static str {
    use crate::terminology::TermMode;
    match (kind, terms.mode) {
        (AnimationKind::Clone, TermMode::Beginner) => " Grow New Cactus ",
        (AnimationKind::Clone, TermMode::Hybrid) => " Clone (Grow from Remote) ",
        (AnimationKind::Clone, TermMode::Git) => " Clone Repository ",
        (AnimationKind::Pull, TermMode::Beginner) => " Pull In Updates ",
        (AnimationKind::Pull, TermMode::Hybrid) => " Pull (Fetch + Merge) ",
        (AnimationKind::Pull, TermMode::Git) => " Pull ",
        (AnimationKind::Push, TermMode::Beginner) => " Send to Team ",
        (AnimationKind::Push, TermMode::Hybrid) => " Push (Send Commits) ",
        (AnimationKind::Push, TermMode::Git) => " Push ",
    }
}

fn teaching_lines(kind: AnimationKind, terms: &Terms) -> Vec<&'static str> {
    use crate::terminology::TermMode;
    match (kind, terms.mode) {
        // ── Clone ────────────────────────────────────────────────
        (AnimationKind::Clone, TermMode::Beginner) => vec![
            "Your new cactus is growing!",
            "The full history was downloaded from the remote.",
            "A new local copy has been created on your machine.",
            "It stays connected to 'origin' for later sync.",
        ],
        (AnimationKind::Clone, TermMode::Hybrid) => vec![
            "Clone complete.",
            "Full commit history was downloaded from the remote.",
            "Your local repo is ready to use.",
            "The remote 'origin' is configured for future fetch/pull/push.",
        ],
        (AnimationKind::Clone, TermMode::Git) => vec![
            "git clone finished.",
            "Every object and ref was fetched into a fresh repository.",
            "The default branch is checked out and tracking origin/<default>.",
            "'origin' is set as the fetch/push URL.",
        ],

        // ── Pull ─────────────────────────────────────────────────
        (AnimationKind::Pull, TermMode::Beginner) => vec![
            "You pulled in the team's latest work.",
            "New checkpoints were downloaded and added to your path.",
            "Your local copy now matches origin.",
        ],
        (AnimationKind::Pull, TermMode::Hybrid) => vec![
            "Pull complete.",
            "New commits were fetched from origin and merged into your branch.",
            "Your local branch is now up to date with its upstream.",
        ],
        (AnimationKind::Pull, TermMode::Git) => vec![
            "git pull finished.",
            "Fetched objects from origin and fast-forward/merged into HEAD.",
            "Local branch is level with its upstream (assuming no conflicts).",
        ],

        // ── Push ─────────────────────────────────────────────────
        (AnimationKind::Push, TermMode::Beginner) => vec![
            "Your work is on the way to the team!",
            "Your local checkpoints were sent to the remote.",
            "Teammates can now see and pull your changes.",
        ],
        (AnimationKind::Push, TermMode::Hybrid) => vec![
            "Push complete.",
            "Your commits were uploaded to the remote branch.",
            "Other clones can now fetch or pull them from origin.",
        ],
        (AnimationKind::Push, TermMode::Git) => vec![
            "git push finished.",
            "Local commits were uploaded to the remote ref.",
            "Upstream now points at the same tip as your local branch.",
        ],
    }
}
