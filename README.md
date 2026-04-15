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

## Why GitCactus?

Most Git UIs sit on opposite ends: plain `git` (powerful but intimidating) or full GUI clients (fine, but they pull you out of the terminal).

GitCactus aims to be the middle ground:

- **Terminal-native** — it lives where you already work.
- **Safe by default** — no operation mutates your repo without an explicit confirmation step.
- **Teaches Git** — the Hybrid terminology mode shows "Checkpoint (Commit)" so beginners learn the real words over time rather than being walled off from them.
- **Searchable** — press `/` to filter history and branches instantly.
- **Editor-friendly** — press `o` to open the selected file in `$EDITOR`.
- **Keyboard-first** — arrow keys, Vim-style (`j/k`), and WASD all work everywhere.

## Status

**v0.1.0 — Early development but actively usable.** Core workflows are implemented: status, interactive staging, commit, commit history, commit details, diff preview, branch list with safe switching and creation, and settings for terminology mode. Remote sync is still a placeholder.

## Features

- Retro 80s/90s-style intro animation (skippable, remembered in settings)
- Keyboard-navigable main menu (arrow keys, j/k/w/s, Enter)
- Live git status view with branch + grouped file lists
- Interactive staging with file selection and confirmation
- Guided commit flow with inline message input
- Commit history browser with search (`/`) and scrollable entries
- Commit details view with file-change summary
- Read-only diff preview (`d`) for working-tree files and historical commits
- Branches / Saved Paths screen with safe switching and "New Game" branch creation
- Terminology modes — Beginner / Hybrid (default) / Git — switchable in-app
- In-app Settings screen
- Check for Updates screen with version display and release notes
- "Open in Editor" (`o`) using `$EDITOR` with safe fallbacks
- Fighting-game-inspired Controls / command list
- CLI flags: `--help`, `--version`, `--intro`, `--skip-intro`
- Clean terminal setup and teardown

## Beginner-Friendly Terminology

GitCactus translates Git's technical vocabulary into clearer language to help new users learn. Three modes are available:

| Concept     | Beginner         | Hybrid (default)              | Git            |
|-------------|------------------|-------------------------------|----------------|
| Commit      | Checkpoint       | Checkpoint (Commit)           | Commit         |
| Branch      | Saved Path       | Saved Path (Branch)           | Branch         |
| Staged      | Ready to Save    | Ready to Save (Staged)        | Staged         |
| Modified    | Changed          | Changed (Modified)            | Modified       |
| Untracked   | New Files        | New Files (Untracked)         | Untracked      |
| Diff        | Compare Changes  | Compare Changes (Diff)        | Diff           |
| HEAD        | Last Saved       | Last Saved (HEAD)             | HEAD           |

**Philosophy**: Git semantics stay real — the UI just helps translate concepts into clearer language. Hybrid mode (the default) teaches both the friendly label and the real Git term, so users gradually learn the standard vocabulary.

Set your preferred mode in `~/.config/gitcactus/settings`:
```
terminology=beginner
```

## Themes

GitCactus ships with a small set of presets. Change the active theme
from the **Settings** screen (main menu → Settings) — preset selection
is in-app, no config-file editing needed:

- **Default** — restrained grayscale with cyan accents
- **Terminal Blue** — cool cyan/blue palette
- **Matrix** — bright green-on-black
- **Retro Danger** — red + yellow arcade palette

Power users can still edit the settings file directly for per-role
color overrides:

```ini
# On Unix: ~/.config/gitcactus/settings
# On Windows: %APPDATA%\gitcactus\settings

# Preset picks the whole palette.
theme=matrix        # or: default, terminal_blue, retro_danger

# Per-role overrides sit on top of whatever preset is active.
theme.primary=blue
theme.highlight=lightcyan
```

Overrides survive preset changes made from the Settings screen, and
the in-app UI flags when any are active. Available roles: `primary`,
`success`, `warning`, `error`, `muted`, `cactus`, `highlight`.
Unknown presets and invalid color names fall back silently — the app
never fails to start because of theme config.

See [docs/theme.md](docs/theme.md) for the full reference.

## Local progression

GitCactus has a lightweight local progression system: XP, levels,
stats, unlocks, and a Skill Tree screen. Everything lives under
`~/.config/gitcactus/profile`. It's cosmetic — **no Git feature is
gated behind progression**.

- Meaningful actions (commits, staging, branch creation, safe branch
  switches, full status→stage→commit combos, clean rebases) earn XP.
- Levels grow quadratically and cap at 100.
- Unlocks are stored locally and never revoked.
- The Skill Tree (main menu → Skill Tree) shows your progress in a
  retro command-console style.

Everything is **local-first**. No accounts, no network calls, no
telemetry in this build. A future opt-in connection for friendly
rankings is documented in
[docs/progression.md](docs/progression.md) but explicitly out of
scope here.

## Safety philosophy

GitCactus will **never silently mutate your repository**. Every future git action will be:
- Intentional — you choose what happens
- Visible — you see what will happen before it does
- Confirmable — you approve before any change is made

## Install

### Homebrew (recommended on macOS/Linuxbrew)

GitCactus is distributed through a custom Homebrew tap:

```bash
brew install fromloltocode/tap/gitcactus
```

Update later with:

```bash
brew upgrade gitcactus
```

> The longer-term goal is to publish to `homebrew-core` so the install
> command collapses to `brew install gitcactus`. For now the custom tap
> keeps release cadence flexible.

### Prebuilt binaries

Tagged releases on GitHub include prebuilt binaries for:

- macOS (`x86_64`, `aarch64`) — `.tar.gz`
- Linux (`x86_64`) — `.tar.gz`
- Windows (`x86_64`) — `.zip` (run from PowerShell / Windows Terminal;
  needs [Git for Windows](https://git-scm.com/download/win) on PATH)

Download from the [Releases page](https://github.com/fromloltocode/gitcactus/releases), extract, and move `gitcactus` (or `gitcactus.exe` on Windows) into your `PATH`.

On Windows, configuration lives in `%APPDATA%\gitcactus\` (typically
`C:\Users\<you>\AppData\Roaming\gitcactus\`) instead of `~/.config/gitcactus/`.

### Build from source

Prerequisites:

- [Rust](https://rustup.rs/) (1.70+)
- A C compiler (for `libgit2`):
  - macOS / Linux: usually already installed
  - Windows: [Build Tools for Visual Studio](https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022)
    (selects MSVC + Windows SDK) plus [Git for Windows](https://git-scm.com/download/win)
- [just](https://github.com/casey/just) (optional, for dev task runner)

```bash
git clone https://github.com/fromloltocode/gitcactus.git
cd gitcactus
cargo build --release
./target/release/gitcactus            # macOS / Linux
.\target\release\gitcactus.exe        # Windows (PowerShell)
```

Or for development: `cargo run`.

### Editor integration

GitCactus reads `$EDITOR` when you press `o` to open a file. If it's
unset, it falls back (in order) to `nvim`, `vim`, `vi`, `nano`, `code`,
`emacs`. If `EDITOR` contains arguments (e.g. `"code --wait"`), the
whole string is honored so non-blocking editors can wait properly.

### CLI flags

```text
gitcactus --help         # print help
gitcactus --version      # print version
gitcactus --intro        # force the retro intro animation
gitcactus --skip-intro   # skip the intro this session
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

Shipped:

- [x] Interactive staging with preview (diff for highlighted file)
- [x] Guided commit flow with message input
- [x] Commit history browser with search
- [x] Commit details view
- [x] Branch list with safe switching and creation ("New Game")
- [x] Controls / command-list screen
- [x] Settings screen for terminology mode
- [x] "Open in Editor" using `$EDITOR`
- [x] Homebrew tap support

Planned:

- [ ] Remote sync (fetch/pull/push) with confirmation
- [ ] Prebuilt release binaries on GitHub Releases
- [ ] Self-update via the in-app update screen
- [ ] Color theme system
- [ ] File-level cursor inside commit details
- [ ] Publication to `homebrew-core`

See the [CONTRIBUTING.md](CONTRIBUTING.md) guide if you'd like to help,
and [docs/releasing.md](docs/releasing.md) for the release process.

## License

MIT. See [LICENSE](LICENSE).
