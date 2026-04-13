# GitCactus Controls

A complete reference for every keybinding in GitCactus, organized by category.

> Also available in-app: select **Controls** from the main menu.

---

## Navigation Schemes

GitCactus supports three navigation styles — use whichever feels natural:

| Style       | Up    | Down  |
|-------------|-------|-------|
| Arrow keys  | `↑`   | `↓`   |
| Vim keys    | `k`   | `j`   |
| WASD keys   | `w`   | `s`   |

All three work identically across every screen.

---

## Basic Moves

| Key              | Name        | Description                        |
|------------------|-------------|------------------------------------|
| `↑` / `k` / `w` | Scroll Up   | Navigate up through menus and lists |
| `↓` / `j` / `s` | Scroll Down | Navigate down through menus and lists |
| `Enter`        | Strike      | Select the highlighted item         |
| `Esc`          | Guard       | Go back to the previous screen      |
| `q`            | Retreat     | Quit GitCactus entirely             |
| `Space`        | Mark        | Toggle selection on a file          |

## Special Moves

| Key            | Name        | Description                        |
|----------------|-------------|------------------------------------|
| `a`            | Mark All    | Select or deselect every file at once |
| `r`            | Reload      | Refresh repository data from disk   |
| `y`            | Confirm     | Accept a confirmation dialog        |
| `n`            | Deny        | Reject a confirmation dialog        |
| _(typing)_     | Inscribe    | Type characters into the commit message |
| `Backspace`    | Erase       | Delete the last character you typed  |

## Power Moves (Combos)

| Sequence                           | Name        | Description                        |
|------------------------------------|-------------|------------------------------------|
| `Space` → `Enter` → `y`           | Stage Combo | Select files, confirm, then stage them |
| _(type)_ → `Enter` → `y`          | Commit Combo | Write a message, confirm, then commit |
| Status → Stage → Commit           | Full Chain  | The complete Git workflow           |
| `--version` (CLI)                  | Identity    | Show GitCactus version              |

## Defensive Moves

| Key / Behavior | Name        | Description                        |
|----------------|-------------|------------------------------------|
| `Esc`          | Block       | Cancel any operation and go back safely |
| `n`            | Counter     | Reject a dangerous confirmation     |
| `q`            | Flee        | Exit the arena (quit the app)       |
| _(automatic)_  | Shield      | GitCactus never changes your repo without asking |
| _(automatic)_  | Barrier     | Empty commit messages are blocked   |
| _(automatic)_  | Ward        | Staging requires explicit file selection |

---

## Screen-Specific Controls

### Title Screen
- Any key → advance to menu

### Main Menu
- `↑`/`↓`/`j`/`k`/`w`/`s` → navigate
- `Enter` → select item
- `q` → quit

### Status Screen
- `Esc` → back to menu
- `q` → quit

### Stage Screen
- `↑`/`↓`/`j`/`k`/`w`/`s` → move through file list
- `Space` → toggle file selection
- `a` → toggle all files
- `r` → refresh file list
- `Enter` → stage selected files (opens confirmation)
- `y`/`Enter` → confirm staging
- `n`/`Esc` → cancel
- `Esc` → back to menu
- `q` → quit

### Commit Screen
- Type normally → edit commit message
- `Backspace` → delete last character
- `Enter` → create commit (opens confirmation)
- `y`/`Enter` → confirm commit
- `n`/`Esc` → cancel
- `Esc` → back to menu

### Controls Screen
- `↑`/`↓`/`j`/`k`/`w`/`s` → switch page
- `Esc` → back to menu
- `q` → quit

### Update Screen
- `↑`/`↓`/`j`/`k`/`w`/`s` → navigate actions
- `Enter` → select action
- `Esc` → back to menu
- `q` → quit
