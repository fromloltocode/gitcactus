//! Tests for the Clone / Pull / Push overlay animations.
//!
//! Covers:
//! - State-machine correctness (tick advances, skip jumps to end,
//!   teaching phase latches).
//! - The `play_*` helpers respect the `enabled` flag — disabling
//!   animations must be a true no-op.
//! - Teaching messages exist for every (kind, terminology) pair.
//! - Integration with `App::handle_action`: any key during an
//!   animation consumes the input and returns a non-mutating effect.
//! - Safety: animations never trigger on failure (i.e. the app does
//!   not call `play_*` on its own — it must be invoked explicitly).

use gitcactus::app::{App, Effect, Screen};
use gitcactus::input::Action;
use gitcactus::mascot::animations::{
    play, play_clone_animation, play_pull_animation, play_push_animation, AnimationKind,
    AnimationPhase, AnimationState,
};
use gitcactus::terminology::{TermMode, Terms};

// ── AnimationState: tick / skip / frames ─────────────────────────────

#[test]
fn new_state_starts_on_frame_zero_playing() {
    let s = AnimationState::new(AnimationKind::Clone);
    assert_eq!(s.kind, AnimationKind::Clone);
    assert_eq!(s.phase, AnimationPhase::Playing);
    assert_eq!(s.frame, 0);
    assert!(s.total_frames() >= 6, "clone should have 6+ frames");
}

#[test]
fn tick_advances_frame_until_final_then_flips_to_teaching() {
    for kind in [AnimationKind::Clone, AnimationKind::Pull, AnimationKind::Push] {
        let mut s = AnimationState::new(kind);
        let total = s.total_frames();
        assert!(total >= 6, "{kind:?} should have 6+ frames");

        // The tick that lands on the last frame flips to Teaching
        // immediately (so the final art is shown *with* the teaching
        // panel). All earlier ticks stay in Playing and advance frame.
        for i in 1..total {
            let done = s.tick();
            if i < total - 1 {
                assert_eq!(s.phase, AnimationPhase::Playing);
                assert!(!done, "tick {i} should not report done");
                assert_eq!(s.frame, i);
            } else {
                // Final frame + transition to Teaching, all in one tick.
                assert_eq!(s.phase, AnimationPhase::Teaching);
                assert!(done, "final tick should report done");
                assert_eq!(s.frame, total - 1);
            }
        }

        // Further ticks while in Teaching are idempotent.
        let frame_before = s.frame;
        assert!(s.tick());
        assert_eq!(s.phase, AnimationPhase::Teaching);
        assert_eq!(s.frame, frame_before);
    }
}

#[test]
fn skip_to_teaching_jumps_to_final_frame() {
    let mut s = AnimationState::new(AnimationKind::Pull);
    s.skip_to_teaching();
    assert_eq!(s.phase, AnimationPhase::Teaching);
    assert_eq!(s.frame, s.total_frames() - 1);

    // And it's idempotent.
    s.skip_to_teaching();
    assert_eq!(s.phase, AnimationPhase::Teaching);
}

#[test]
fn current_art_is_non_empty_for_every_frame() {
    for kind in [AnimationKind::Clone, AnimationKind::Pull, AnimationKind::Push] {
        let mut s = AnimationState::new(kind);
        for _ in 0..s.total_frames() + 2 {
            let art = s.current_art();
            assert!(!art.is_empty(), "{kind:?} frame {} empty", s.frame);
            // Every frame is the same height so the overlay layout is
            // stable (no flicker from shifting art).
            let line_count = art.lines().count();
            assert_eq!(
                line_count, 7,
                "{kind:?} frame {} should be 7 lines, got {line_count}",
                s.frame
            );
            s.tick();
        }
    }
}

// ── Teaching text: exists for every (kind, mode) ─────────────────────

#[test]
fn teaching_lines_cover_every_terminology_mode() {
    for kind in [AnimationKind::Clone, AnimationKind::Pull, AnimationKind::Push] {
        for mode in [TermMode::Beginner, TermMode::Hybrid, TermMode::Git] {
            let s = AnimationState::new(kind);
            let terms = Terms::new(mode);
            let lines = s.teaching_lines(&terms);
            assert!(
                (3..=5).contains(&lines.len()),
                "{kind:?}/{mode:?} should have 3-5 teaching lines, got {}",
                lines.len()
            );
            for line in lines {
                assert!(!line.is_empty(), "{kind:?}/{mode:?} empty teaching line");
            }

            // Title is also defined for every combo.
            let title = s.title(&terms);
            assert!(!title.trim().is_empty(), "{kind:?}/{mode:?} empty title");
        }
    }
}

// ── play_* helpers: enabled / disabled semantics ─────────────────────

#[test]
fn play_when_enabled_sets_the_state() {
    let mut slot: Option<AnimationState> = None;
    play_clone_animation(&mut slot, true);
    assert_eq!(slot.as_ref().map(|s| s.kind), Some(AnimationKind::Clone));

    play_pull_animation(&mut slot, true);
    assert_eq!(slot.as_ref().map(|s| s.kind), Some(AnimationKind::Pull));

    play_push_animation(&mut slot, true);
    assert_eq!(slot.as_ref().map(|s| s.kind), Some(AnimationKind::Push));
}

#[test]
fn play_when_disabled_is_a_no_op() {
    let mut slot: Option<AnimationState> = None;
    play_clone_animation(&mut slot, false);
    assert!(slot.is_none(), "disabled play must not start animation");

    play_pull_animation(&mut slot, false);
    assert!(slot.is_none());

    play_push_animation(&mut slot, false);
    assert!(slot.is_none());
}

#[test]
fn play_when_disabled_preserves_existing_animation() {
    // If a previous animation is somehow still in the slot, a
    // disabled play() must not replace or clear it.
    let mut slot = Some(AnimationState::new(AnimationKind::Pull));
    play(&mut slot, AnimationKind::Push, false);
    assert_eq!(slot.as_ref().map(|s| s.kind), Some(AnimationKind::Pull));
}

// ── App integration: input is swallowed while animation is active ────

#[test]
fn any_action_during_playing_skips_to_teaching() {
    // Exhaustive: every action we could conceivably receive should
    // skip playback instead of firing a mutating effect.
    let all_actions = [
        Action::Quit,
        Action::Back,
        Action::MoveUp,
        Action::MoveDown,
        Action::Select,
        Action::Toggle,
        Action::ToggleAll,
        Action::Refresh,
        Action::Confirm,
        Action::Deny,
        Action::Preview,
        Action::Search,
        Action::Open,
        Action::Portal,
        Action::Backspace,
        Action::Char('x'),
        Action::Other,
    ];

    for action in all_actions {
        let mut app = App::new();
        app.screen = Screen::Menu;
        app.animation = Some(AnimationState::new(AnimationKind::Pull));

        let effect = app.handle_action(action);
        assert_eq!(
            effect,
            Effect::None,
            "animation should swallow {action:?} into None"
        );
        let anim = app.animation.as_ref().expect("still present");
        assert_eq!(anim.phase, AnimationPhase::Teaching);
    }
}

#[test]
fn any_action_during_teaching_dismisses_animation() {
    let mut app = App::new();
    app.screen = Screen::Menu;
    let mut s = AnimationState::new(AnimationKind::Push);
    s.skip_to_teaching();
    app.animation = Some(s);

    let effect = app.handle_action(Action::Select);
    assert_eq!(effect, Effect::None);
    assert!(app.animation.is_none(), "teaching press should clear");
}

#[test]
fn animation_does_not_change_screen() {
    // Dismissing an animation must keep the user where they were —
    // the overlay is visual only, not a navigation event.
    let mut app = App::new();
    app.screen = Screen::RemoteSync;
    let mut s = AnimationState::new(AnimationKind::Clone);
    s.skip_to_teaching();
    app.animation = Some(s);

    let _ = app.handle_action(Action::Quit);
    assert_eq!(app.screen, Screen::RemoteSync);
}

// ── Safety: animations don't trigger themselves ──────────────────────

#[test]
fn new_app_has_no_animation() {
    let app = App::new();
    assert!(app.animation.is_none());
    // Default is "on" so the flag is ready when operations land.
    assert!(app.animations_enabled);
}

#[test]
fn animation_never_triggers_from_action_handling() {
    // The state machine must never create an animation on its own —
    // only the Effect handlers for successful Clone/Pull/Push should.
    // Walk every screen + every action and assert no animation appears.
    let screens = [
        Screen::Title,
        Screen::Menu,
        Screen::Status,
        Screen::Stage,
        Screen::Commit,
        Screen::Branches,
        Screen::History,
        Screen::RemoteSync,
        Screen::Help,
        Screen::Update,
        Screen::DiffPreview,
        Screen::Settings,
        Screen::CommitDetails,
        Screen::SkillTree,
        Screen::RebasePortal,
    ];
    let actions = [
        Action::Quit,
        Action::Back,
        Action::MoveUp,
        Action::MoveDown,
        Action::Select,
        Action::Refresh,
        Action::Confirm,
        Action::Deny,
    ];

    for screen in screens {
        for action in actions {
            let mut app = App::new();
            app.screen = screen;
            let _ = app.handle_action(action);
            assert!(
                app.animation.is_none(),
                "{screen:?} + {action:?} must not spawn an animation"
            );
        }
    }
}
