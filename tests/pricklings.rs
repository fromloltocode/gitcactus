//! Integration tests for Pricklings.
//!
//! Covers:
//! - State navigation on Launchpad, Hub, Scan Locations, Results
//! - Cursor boundaries and section transitions
//! - Effect dispatch from keyboard inputs
//! - Safety: discovery never emits a repo-mutating Effect; Save/Open
//!   only fire after an explicit user action
//! - Store round-trip (dedupe + parse invariants)
//!
//! Discovery-level unit tests live inline in `src/pricklings/`.

use gitcactus::app::{
    App, Effect, HubSelection, PricklingsHubState, PricklingsResultsState,
    ScanLocationsMode, ScanLocationsSelection, ScanLocationsState, Screen,
};
use gitcactus::input::Action;
use gitcactus::pricklings::{Prickling, PricklingsStore};
use std::path::PathBuf;

// ── Hub state ───────────────────────────────────────────────────────

fn seeded_hub(n_saved: usize) -> PricklingsHubState {
    let mut hub = PricklingsHubState::new();
    for i in 0..n_saved {
        hub.store.saved.push(Prickling {
            path: PathBuf::from(format!("/tmp/proj-{i}")),
            display_name: format!("proj-{i}"),
        });
    }
    hub
}

#[test]
fn hub_cursor_walks_saved_then_actions() {
    let mut h = seeded_hub(2);
    assert!(matches!(h.selection(), HubSelection::Saved(0)));
    h.move_down();
    assert!(matches!(h.selection(), HubSelection::Saved(1)));
    h.move_down();
    assert_eq!(h.selection(), HubSelection::Find);
    h.move_down();
    assert_eq!(h.selection(), HubSelection::ManageLocations);
    h.move_down();
    assert_eq!(h.selection(), HubSelection::ManageLocations); // clamps
}

#[test]
fn hub_cursor_move_up_clamps_at_zero() {
    let mut h = seeded_hub(3);
    h.move_up();
    assert_eq!(h.cursor, 0);
}

#[test]
fn hub_clamp_cursor_shrinks_with_saved_list() {
    let mut h = seeded_hub(3);
    h.cursor = 2;
    h.store.saved.truncate(1);
    h.clamp_cursor();
    // After shrinking to 1 saved entry, cursor must still be valid:
    // max index is saved_len (1) + ACTION_ROWS (2) - 1 = 2.
    assert!(h.cursor <= 2);
}

#[test]
fn hub_with_no_saved_lands_on_find() {
    let h = seeded_hub(0);
    assert_eq!(h.cursor, 0);
    assert_eq!(h.selection(), HubSelection::Find);
}

// ── Scan Locations state ────────────────────────────────────────────

fn seeded_scan(n_roots: usize) -> ScanLocationsState {
    let mut s = ScanLocationsState::new();
    for i in 0..n_roots {
        s.roots.push(PathBuf::from(format!("/tmp/root-{i}")));
    }
    s
}

#[test]
fn scan_cursor_walks_roots_then_actions() {
    let mut s = seeded_scan(2);
    assert_eq!(s.selection(), ScanLocationsSelection::Root(0));
    s.move_down();
    assert_eq!(s.selection(), ScanLocationsSelection::Root(1));
    s.move_down();
    assert_eq!(s.selection(), ScanLocationsSelection::AddPath);
    s.move_down();
    assert_eq!(s.selection(), ScanLocationsSelection::ScanNow);
    s.move_down();
    assert_eq!(s.selection(), ScanLocationsSelection::ScanNow);
}

#[test]
fn scan_cursor_with_no_roots_lands_on_add_path() {
    let s = seeded_scan(0);
    assert_eq!(s.selection(), ScanLocationsSelection::AddPath);
}

// ── Results state ────────────────────────────────────────────────────

#[test]
fn results_cursor_is_bounds_safe_when_empty() {
    let mut r = PricklingsResultsState::new();
    r.move_down();
    r.move_up();
    assert_eq!(r.cursor, 0);
    assert_eq!(r.selected(), None);
}

// ── Launchpad input handling ─────────────────────────────────────────

#[test]
fn launchpad_select_pricklings_enters_hub_and_loads() {
    let mut app = App::new();
    app.screen = Screen::Launchpad;
    app.launchpad.cursor = 0; // Pricklings row

    let effect = app.handle_action(Action::Select);
    assert_eq!(effect, Effect::LoadPricklings);
    assert_eq!(app.screen, Screen::PricklingsHub);
}

#[test]
fn launchpad_select_settings_transitions() {
    let mut app = App::new();
    app.screen = Screen::Launchpad;
    app.launchpad.cursor = 1;
    let _ = app.handle_action(Action::Select);
    assert_eq!(app.screen, Screen::Settings);
}

#[test]
fn launchpad_select_exit_quits() {
    let mut app = App::new();
    app.screen = Screen::Launchpad;
    app.launchpad.cursor = 2;
    let effect = app.handle_action(Action::Select);
    assert_eq!(effect, Effect::Quit);
}

#[test]
fn launchpad_q_quits() {
    let mut app = App::new();
    app.screen = Screen::Launchpad;
    let effect = app.handle_action(Action::Quit);
    assert_eq!(effect, Effect::Quit);
}

// ── Hub input handling ──────────────────────────────────────────────

#[test]
fn hub_enter_on_saved_emits_open_prickling() {
    let mut app = App::new();
    app.screen = Screen::PricklingsHub;
    app.pricklings_hub.store.saved.push(Prickling {
        path: PathBuf::from("/tmp/some-repo"),
        display_name: "some-repo".into(),
    });
    app.pricklings_hub.cursor = 0;

    let effect = app.handle_action(Action::Select);
    assert_eq!(
        effect,
        Effect::OpenPrickling(PathBuf::from("/tmp/some-repo"))
    );
}

#[test]
fn hub_enter_on_find_emits_scan_when_roots_exist() {
    let mut app = App::new();
    app.screen = Screen::PricklingsHub;
    app.pricklings_hub
        .store
        .scan_roots
        .push(PathBuf::from("/tmp"));
    // No saved entries -> cursor already on Find row.
    assert_eq!(app.pricklings_hub.selection(), HubSelection::Find);

    let effect = app.handle_action(Action::Select);
    assert_eq!(effect, Effect::ScanPricklings);
}

#[test]
fn hub_enter_on_find_without_roots_redirects_to_scan_locations() {
    let mut app = App::new();
    app.screen = Screen::PricklingsHub;
    // No roots and no saved entries.
    let effect = app.handle_action(Action::Select);
    assert_eq!(effect, Effect::None);
    assert_eq!(app.screen, Screen::ScanLocations);
}

#[test]
fn hub_enter_on_manage_locations_transitions_and_copies_roots() {
    let mut app = App::new();
    app.screen = Screen::PricklingsHub;
    let root = PathBuf::from("/tmp/home");
    app.pricklings_hub.store.scan_roots.push(root.clone());
    app.pricklings_hub.cursor = 1; // skip the Find row

    let _ = app.handle_action(Action::Select);
    assert_eq!(app.screen, Screen::ScanLocations);
    assert_eq!(app.scan_locations.roots, vec![root]);
}

#[test]
fn hub_d_on_saved_emits_remove() {
    let mut app = App::new();
    app.screen = Screen::PricklingsHub;
    app.pricklings_hub.store.saved.push(Prickling {
        path: PathBuf::from("/tmp/x"),
        display_name: "x".into(),
    });

    let effect = app.handle_action(Action::Deny);
    assert_eq!(effect, Effect::RemoveSavedPrickling(0));
}

#[test]
fn hub_d_on_action_row_is_a_noop() {
    let mut app = App::new();
    app.screen = Screen::PricklingsHub;
    // No saved → cursor on action row.
    let effect = app.handle_action(Action::Deny);
    assert_eq!(effect, Effect::None);
}

#[test]
fn hub_back_returns_to_launchpad() {
    let mut app = App::new();
    app.screen = Screen::PricklingsHub;
    let _ = app.handle_action(Action::Back);
    assert_eq!(app.screen, Screen::Launchpad);
}

// ── Scan Locations input handling ───────────────────────────────────

#[test]
fn scan_enter_on_add_path_opens_adding_mode() {
    let mut app = App::new();
    app.screen = Screen::ScanLocations;
    // No roots -> cursor on AddPath.
    assert_eq!(
        app.scan_locations.selection(),
        ScanLocationsSelection::AddPath
    );
    let _ = app.handle_action(Action::Select);
    assert_eq!(app.scan_locations.mode, ScanLocationsMode::Adding);
}

#[test]
fn scan_adding_mode_types_path_and_submits() {
    let mut app = App::new();
    app.screen = Screen::ScanLocations;
    app.scan_locations.mode = ScanLocationsMode::Adding;

    for c in "/tmp/foo".chars() {
        let effect = app.handle_action(Action::Char(c));
        assert_eq!(effect, Effect::None);
    }
    assert_eq!(app.scan_locations.input_buffer, "/tmp/foo");

    let effect = app.handle_action(Action::Select);
    assert_eq!(effect, Effect::AddScanRoot(PathBuf::from("/tmp/foo")));
}

#[test]
fn scan_adding_mode_empty_submit_does_nothing() {
    let mut app = App::new();
    app.screen = Screen::ScanLocations;
    app.scan_locations.mode = ScanLocationsMode::Adding;
    let effect = app.handle_action(Action::Select);
    assert_eq!(effect, Effect::None);
}

#[test]
fn scan_adding_mode_backspace_pops_char() {
    let mut app = App::new();
    app.screen = Screen::ScanLocations;
    app.scan_locations.mode = ScanLocationsMode::Adding;
    app.scan_locations.input_buffer = "abc".into();
    app.handle_action(Action::Backspace);
    assert_eq!(app.scan_locations.input_buffer, "ab");
}

#[test]
fn scan_adding_mode_esc_cancels() {
    let mut app = App::new();
    app.screen = Screen::ScanLocations;
    app.scan_locations.mode = ScanLocationsMode::Adding;
    app.scan_locations.input_buffer = "abc".into();
    let _ = app.handle_action(Action::Back);
    assert_eq!(app.scan_locations.mode, ScanLocationsMode::Browse);
    assert!(app.scan_locations.input_buffer.is_empty());
}

#[test]
fn scan_d_on_root_emits_remove() {
    let mut app = App::new();
    app.screen = Screen::ScanLocations;
    app.scan_locations.roots.push(PathBuf::from("/tmp/root"));
    app.scan_locations.cursor = 0;
    let effect = app.handle_action(Action::Deny);
    assert_eq!(effect, Effect::RemoveScanRoot(0));
}

#[test]
fn scan_now_with_no_roots_shows_error_instead_of_scanning() {
    let mut app = App::new();
    app.screen = Screen::ScanLocations;
    // No roots -> cursor walks AddPath then ScanNow; move down once.
    app.handle_action(Action::MoveDown);
    assert_eq!(
        app.scan_locations.selection(),
        ScanLocationsSelection::ScanNow
    );
    let effect = app.handle_action(Action::Select);
    assert_eq!(effect, Effect::None, "must not scan when no roots approved");
    assert_eq!(app.scan_locations.mode, ScanLocationsMode::Result);
}

#[test]
fn scan_now_with_roots_emits_scan_pricklings() {
    let mut app = App::new();
    app.screen = Screen::ScanLocations;
    app.scan_locations.roots.push(PathBuf::from("/tmp/root"));
    // Roots [0], AddPath [1], ScanNow [2]. Navigate there.
    app.scan_locations.cursor = 2;
    let effect = app.handle_action(Action::Select);
    assert_eq!(effect, Effect::ScanPricklings);
}

// ── Results input handling ──────────────────────────────────────────

#[test]
fn results_enter_opens_selected_prickling() {
    let mut app = App::new();
    app.screen = Screen::PricklingsResults;
    app.pricklings_results.results.push(Prickling {
        path: PathBuf::from("/tmp/found"),
        display_name: "found".into(),
    });
    let effect = app.handle_action(Action::Select);
    assert_eq!(
        effect,
        Effect::OpenPrickling(PathBuf::from("/tmp/found"))
    );
}

#[test]
fn results_s_saves_selected_prickling() {
    let mut app = App::new();
    app.screen = Screen::PricklingsResults;
    app.pricklings_results.results.push(Prickling {
        path: PathBuf::from("/tmp/found"),
        display_name: "found".into(),
    });
    let effect = app.handle_action(Action::Search); // 's' key
    match effect {
        Effect::SavePrickling(p) => {
            assert_eq!(p.path, PathBuf::from("/tmp/found"));
        }
        other => panic!("expected SavePrickling, got {other:?}"),
    }
}

#[test]
fn results_refresh_emits_rescan() {
    let mut app = App::new();
    app.screen = Screen::PricklingsResults;
    let effect = app.handle_action(Action::Refresh);
    assert_eq!(effect, Effect::ScanPricklings);
}

#[test]
fn results_back_returns_to_hub() {
    let mut app = App::new();
    app.screen = Screen::PricklingsResults;
    let _ = app.handle_action(Action::Back);
    assert_eq!(app.screen, Screen::PricklingsHub);
}

#[test]
fn results_empty_list_ignores_open_and_save() {
    let mut app = App::new();
    app.screen = Screen::PricklingsResults;
    assert_eq!(app.handle_action(Action::Select), Effect::None);
    assert_eq!(app.handle_action(Action::Search), Effect::None);
}

// ── Safety: Pricklings surfaces never emit git-mutating effects ─────

#[test]
fn pricklings_screens_never_emit_git_mutations() {
    let screens = [
        Screen::Launchpad,
        Screen::PricklingsHub,
        Screen::ScanLocations,
        Screen::PricklingsResults,
    ];
    let actions = [
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

    for screen in screens {
        for action in actions {
            let mut app = App::new();
            app.screen = screen;
            // Seed a realistic state for Results / Hub so Select
            // actually has a valid target to operate on.
            app.pricklings_hub.store.saved.push(Prickling {
                path: PathBuf::from("/tmp/x"),
                display_name: "x".into(),
            });
            app.pricklings_hub.store.scan_roots.push(PathBuf::from("/tmp/x"));
            app.pricklings_results.results.push(Prickling {
                path: PathBuf::from("/tmp/x"),
                display_name: "x".into(),
            });

            let effect = app.handle_action(action);
            match effect {
                // Allowed: quit, nav, load pricklings store, scan,
                // open (chdir only), save/remove Pricklings, save/
                // remove scan root. None of these touch any Git
                // repo's on-disk state.
                Effect::None
                | Effect::Quit
                | Effect::LoadPricklings
                | Effect::ScanPricklings
                | Effect::OpenPrickling(_)
                | Effect::SavePrickling(_)
                | Effect::RemoveSavedPrickling(_)
                | Effect::AddScanRoot(_)
                | Effect::RemoveScanRoot(_) => {}
                // Anything else is a regression: a pricklings surface
                // should never trigger (e.g.) StageFiles, CreateCommit,
                // FetchFromRemote, ExecuteRebase, etc.
                other => panic!(
                    "{screen:?} + {action:?} emitted git-mutating effect {other:?}"
                ),
            }
        }
    }
}

// ── Store parse round-trip (smoke; unit tests cover the details) ────

#[test]
fn store_round_trip_through_parse_and_to_text() {
    let mut s = PricklingsStore::default();
    s.scan_roots.push(PathBuf::from("/tmp/a"));
    s.scan_roots.push(PathBuf::from("/tmp/b"));
    s.save_prickling(Prickling {
        path: PathBuf::from("/tmp/a/foo"),
        display_name: "Foo".into(),
    });

    let text = s.to_text();
    let back = PricklingsStore::parse(&text);
    assert_eq!(back, s);
}
