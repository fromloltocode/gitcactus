# GitCactus local progression

GitCactus has a small, **local-only** progression layer. It rewards
real usage without gating any core Git functionality, and it lives
entirely on your machine.

## Where progression lives

```
~/.config/gitcactus/profile
```

Plain-text `key=value` format, same style as the settings file.
Safe to inspect, back up, edit, or delete — invalid values are
ignored on load, and a missing file just means "fresh profile".

Example:

```
# GitCactus local progression profile.
version=1
xp=175
level=2
commits_created=7
files_staged=34
branches_created=2
branches_switched=5
combos_completed=4
rebases_completed=0
unlocks=rank_apprentice
```

## What earns XP

Only meaningful actions grant XP. Navigation and reading don't.

| Event                                   | XP        | Counts toward          |
|-----------------------------------------|-----------|------------------------|
| Create a commit                         | 15        | `commits_created`      |
| Complete a status → stage → commit chain| 25        | `combos_completed`     |
| Stage files                             | 1 per file, capped at 10 per event | `files_staged` |
| Create a branch ("New Game")            | 10        | `branches_created`     |
| Switch branches safely                  | 3         | `branches_switched`    |
| Complete a clean rebase                 | 20        | `rebases_completed`    |

The file-staging cap (10 XP per event regardless of file count) is
deliberate: it stops anyone from farming XP by hammering space on a
huge file list.

## How levels work

XP thresholds grow quadratically:

```
level 1 →   0 XP
level 2 →  50 XP
level 3 → 200 XP
level 4 → 450 XP
level 5 → 800 XP
```

Formula: `xp_for_level(n) = (n - 1)² × 50` (clamped at level 100).

## Unlocks

Unlocks are cosmetic. They never change functionality. Once earned,
they stay earned even if you reset your XP.

| Unlock key              | Shown as             | Earned when                       |
|-------------------------|----------------------|-----------------------------------|
| `rank_apprentice`       | `Rank: Apprentice`   | Level 2                           |
| `cactus_hat`            | `Cactus Hat`         | Level 3                           |
| `rank_journeyman`       | `Rank: Journeyman`   | Level 5                           |
| `combo_chainer`         | `Combo Chainer`      | 5 combos                          |
| `saved_path_surveyor`   | `Path Surveyor`      | 3 branches created                |

The Cactus Hat is a cosmetic hook for a future flair system. For now
it's just a flag in your profile and an entry in the Skill Tree
screen — no visual changes yet, by design.

## The Skill Tree

**Main menu → Skill Tree.** Retro command-console styling, still
grayscale-first. Shows:

- Your level, XP progress bar, and raw stat counters
- A simple top-to-bottom node graph of skills you've unlocked
- What you've earned so far, plus what's coming next

Node unlock conditions are purely derived from your stat counters —
no separate tracking, no duplication.

## Design philosophy

- **Local-first.** Every byte of progression lives under your home
  directory.
- **Fun, not gated.** No Git feature is hidden behind levels or XP.
  A fresh profile sees everything.
- **Respectful.** Progression is there if you want it. Nothing pops
  up to interrupt your workflow to announce a level-up.
- **User-owned data.** Delete the file and you delete your
  progression, cleanly.

## Future direction: optional connected profiles

This phase intentionally ships **no backend, no accounts, no cloud
sync, no telemetry**. The long-term vision is:

1. Your profile stays local and is the source of truth.
2. At some future point, users may **optionally** connect their
   profile to a community service — for friendly rankings, seasonal
   events, shared cactus cosmetics, etc.
3. That connection will always be **opt-in**, owned by the user, and
   revocable.
4. Nothing in that future path changes how the local profile works
   today. Offline will always be a first-class mode.

In other words: local-first now, optional friendly competition later,
never surveillance.
