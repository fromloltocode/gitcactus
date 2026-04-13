# GitCactus

A retro-inspired terminal Git assistant that makes version control more intuitive, visual, and less intimidating.

```
  ______ _ _    _____            _
 / _____|_) |_ / ____|          | |
| |  __ _| |_| |     __ _  ___| |_ _   _ ___
| | |_ | | __| |    / _` |/ __| __| | | / __|
| |__| | | |_| |___| (_| | (__| |_| |_| \__ \
 \_____|_|\__|\______\__,_|\___|\___|__,_|___/
```

> Your prickly-but-friendly Git companion.

GitCactus is a terminal-native Git assistant built in Rust. It aims to be modern, tasteful, slightly game-like, and beginner-friendly without becoming childish. Think "fun developer tool," not "toy."

## Status

**v0.1.0 — Early development.** The TUI skeleton is in place with a title screen, navigable main menu, a live git status screen, and placeholder screens for upcoming features. No destructive git operations are implemented yet.

## Features

- Retro-styled title screen with ASCII cactus mascot
- Keyboard-navigable main menu (arrow keys, j/k, Enter)
- Live git status view (branch, modified/staged/untracked file lists)
- Interactive staging with file selection and confirmation
- Check for Updates screen with version display and release notes
- `--version` / `-V` CLI flag
- Placeholder screens for: Commit, Branches, History, Remote Sync, Help
- Clean terminal setup and teardown

## Safety philosophy

GitCactus will **never silently mutate your repository**. Every future git action will be:
- Intentional — you choose what happens
- Visible — you see what will happen before it does
- Confirmable — you approve before any change is made

## Install

### Prerequisites

- [Rust](https://rustup.rs/) (1.70+)
- A C compiler (for `libgit2` — usually already installed on macOS/Linux)
- [just](https://github.com/casey/just) (optional, for task runner commands)

### Build and run

```bash
cargo run
```

Or with `just`:

```bash
just run
```

### Build for release

```bash
cargo build --release
./target/release/gitcactus
```

## Usage

| Key         | Action              |
|-------------|---------------------|
| Any key     | Dismiss title screen|
| Up / k      | Move up in menu     |
| Down / j    | Move down in menu   |
| Enter       | Select menu item    |
| Esc         | Go back             |
| q           | Quit                |

Run `gitcactus` from inside any git repository to see live status. Running outside a repo still works — the status screen will indicate no repo was found.

## Project structure

```
src/
├── main.rs          Entry point, terminal setup, event loop
├── app.rs           App state, screen enum, menu model
├── ui.rs            Rendering dispatcher
├── update.rs        Version info and update-check logic
├── git/
│   ├── mod.rs
│   ├── status.rs    Read-only git status via libgit2
│   └── stage.rs     Safe staging operations via libgit2
├── screens/
│   ├── mod.rs
│   ├── title.rs     Title/splash screen
│   ├── menu.rs      Main menu with sidebar
│   ├── status.rs    Status screen + placeholder renderer
│   ├── stage.rs     Interactive staging screen
│   └── update.rs    Check for updates screen
└── mascot/
    ├── mod.rs
    └── cactus.rs    ASCII art and cactus personality
```

See [docs/architecture.md](docs/architecture.md) for detailed design rationale.

## Development

```bash
just check    # type-check
just fmt      # format
just clippy   # lint
just test     # test
just ci       # all of the above
```

## Roadmap

Phase 2 goals:

- [ ] Stage changes interactively (file picker with preview)
- [ ] Commit with message editor
- [ ] Branch list with create/switch/delete
- [ ] Commit history browser
- [ ] Remote sync (fetch/pull/push) with confirmation
- [ ] Help screen with contextual git explanations
- [ ] Cactus tips that rotate based on context
- [ ] Color theme system
- [ ] Self-update (download and install new versions from within the app)

See the [CONTRIBUTING.md](CONTRIBUTING.md) guide if you'd like to help.

## License

MIT. See [LICENSE](LICENSE).
