# Architecture

This document describes the module layout of GitCactus and the reasoning behind it.

## Overview

GitCactus follows a simple layered architecture:

```
┌─────────────────────────┐
│        main.rs           │  Terminal setup, event loop, input handling
├─────────────────────────┤
│         ui.rs            │  Rendering dispatcher (reads App, calls screens)
├──────────┬──────────────┤
│ screens/ │   mascot/     │  Individual screen renderers, ASCII art
├──────────┴──────────────┤
│         app.rs           │  Central state: current screen, menu index
├─────────────────────────┤
│          git/            │  Read-only git integration via libgit2
└─────────────────────────┘
```

Data flows downward: `main.rs` owns the `App` and passes `&App` (or `&mut App`) to the layers below.

## Modules

### `main.rs`

Owns the terminal lifecycle (raw mode, alternate screen) and the event loop. All input handling lives here — key events modify `App` state directly. This keeps the "what happens when the user presses a key" logic in one place.

### `app.rs`

The `App` struct holds all mutable state: which screen is active and the menu cursor position. The `Screen` enum lists every screen the app can display. Menu items are defined as a const array mapping labels to screens.

No rendering or I/O happens here. This module is pure data and state transitions.

### `ui.rs`

A thin dispatcher that matches on `app.screen` and calls the right screen module's `render()` function. This keeps the rendering entry point small and predictable.

### `screens/`

Each screen gets its own file:

- **`title.rs`** — Splash screen with ASCII title art, cactus mascot, and tagline.
- **`menu.rs`** — Main menu with a sidebar (cactus + tip) and a navigable item list.
- **`status.rs`** — Live git status display and a reusable `render_placeholder()` for unimplemented screens.

Screen modules receive `&App` (and any data they need, like `&RepoStatus`) and render into a `Frame`. They never mutate state.

### `git/`

Git integration lives here, isolated from the UI. Currently contains `status.rs` which uses `git2` to read branch name and file counts. This module only exposes **read** operations. Future write operations (stage, commit, push) will be added here with explicit confirmation requirements documented in the function signatures.

### `mascot/`

ASCII art and personality text for the cactus mascot. Kept separate so it's easy to find and update without touching logic.

## Design decisions

**Why no async?** The app is a simple TUI with synchronous git reads. Async would add complexity without benefit at this stage.

**Why pass `RepoStatus` from main instead of reading inside the screen?** Keeps rendering pure and fast. The event loop controls when expensive git operations happen (currently: on startup and when entering the status screen).

**Why a flat screen enum instead of a navigation stack?** A stack is overkill for a single-level menu. If nested navigation is needed later, the enum can be replaced with a stack without changing the rendering layer.

**Why `git2` instead of shelling out to `git`?** Programmatic access, no dependency on the user's `git` binary version, and better error handling. The tradeoff is a heavier compile (libgit2 C dependency), but it's worth it for correctness.
