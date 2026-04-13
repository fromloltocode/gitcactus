//! Tests for the input mapping layer, including WASD support.

use crossterm::event::KeyCode;
use gitcactus::input::{map_key, map_key_text, Action};

// ── Arrow keys ───────────────────────────────────────────────────────

#[test]
fn arrow_up_maps_to_move_up() {
    assert_eq!(map_key(KeyCode::Up), Action::MoveUp);
}

#[test]
fn arrow_down_maps_to_move_down() {
    assert_eq!(map_key(KeyCode::Down), Action::MoveDown);
}

// ── Vim keys ─────────────────────────────────────────────────────────

#[test]
fn k_maps_to_move_up() {
    assert_eq!(map_key(KeyCode::Char('k')), Action::MoveUp);
}

#[test]
fn j_maps_to_move_down() {
    assert_eq!(map_key(KeyCode::Char('j')), Action::MoveDown);
}

// ── WASD keys ────────────────────────────────────────────────────────

#[test]
fn w_maps_to_move_up() {
    assert_eq!(map_key(KeyCode::Char('w')), Action::MoveUp);
}

#[test]
fn s_maps_to_move_down() {
    assert_eq!(map_key(KeyCode::Char('s')), Action::MoveDown);
}

// ── WASD in text mode should type characters ─────────────────────────

#[test]
fn w_types_char_in_text_mode() {
    assert_eq!(map_key_text(KeyCode::Char('w')), Action::Char('w'));
}

#[test]
fn s_types_char_in_text_mode() {
    assert_eq!(map_key_text(KeyCode::Char('s')), Action::Char('s'));
}

// ── Other key mappings ───────────────────────────────────────────────

#[test]
fn q_maps_to_quit() {
    assert_eq!(map_key(KeyCode::Char('q')), Action::Quit);
}

#[test]
fn esc_maps_to_back() {
    assert_eq!(map_key(KeyCode::Esc), Action::Back);
}

#[test]
fn enter_maps_to_select() {
    assert_eq!(map_key(KeyCode::Enter), Action::Select);
}

#[test]
fn space_maps_to_toggle() {
    assert_eq!(map_key(KeyCode::Char(' ')), Action::Toggle);
}

#[test]
fn a_maps_to_toggle_all() {
    assert_eq!(map_key(KeyCode::Char('a')), Action::ToggleAll);
}

#[test]
fn r_maps_to_refresh() {
    assert_eq!(map_key(KeyCode::Char('r')), Action::Refresh);
}

#[test]
fn y_maps_to_confirm() {
    assert_eq!(map_key(KeyCode::Char('y')), Action::Confirm);
}

#[test]
fn n_maps_to_deny() {
    assert_eq!(map_key(KeyCode::Char('n')), Action::Deny);
}

#[test]
fn unknown_key_maps_to_other() {
    assert_eq!(map_key(KeyCode::F(1)), Action::Other);
    assert_eq!(map_key(KeyCode::Tab), Action::Other);
}

// ── Text mode edge cases ─────────────────────────────────────────────

#[test]
fn text_mode_esc_is_back() {
    assert_eq!(map_key_text(KeyCode::Esc), Action::Back);
}

#[test]
fn text_mode_enter_is_select() {
    assert_eq!(map_key_text(KeyCode::Enter), Action::Select);
}

#[test]
fn text_mode_backspace_is_backspace() {
    assert_eq!(map_key_text(KeyCode::Backspace), Action::Backspace);
}

#[test]
fn text_mode_arrows_still_navigate() {
    assert_eq!(map_key_text(KeyCode::Up), Action::MoveUp);
    assert_eq!(map_key_text(KeyCode::Down), Action::MoveDown);
}

#[test]
fn text_mode_q_types_q_not_quit() {
    assert_eq!(map_key_text(KeyCode::Char('q')), Action::Char('q'));
}

#[test]
fn text_mode_special_chars_type_normally() {
    assert_eq!(map_key_text(KeyCode::Char('!')), Action::Char('!'));
    assert_eq!(map_key_text(KeyCode::Char('@')), Action::Char('@'));
    assert_eq!(map_key_text(KeyCode::Char(' ')), Action::Char(' '));
}

// ── Security: no action can be created that bypasses the enum ────────

#[test]
fn all_printable_ascii_handled_in_text_mode() {
    for c in ' '..='~' {
        let action = map_key_text(KeyCode::Char(c));
        assert_eq!(action, Action::Char(c), "Char {c:?} should map to Action::Char in text mode");
    }
}
