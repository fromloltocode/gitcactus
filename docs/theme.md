# GitCactus themes

GitCactus stays restrained by default — mostly grayscale with a few
subtle accent colors. Customization exists for users who want to make
the tool feel like theirs without turning it into a neon playground.

## Where config lives

```
~/.config/gitcactus/settings
```

This is the same file that stores `skip_intro=true` and
`terminology=hybrid`. Theme keys simply live alongside them.

## Picking a preset

```
# ~/.config/gitcactus/settings
theme=matrix
```

Built-in presets (parsing is case-insensitive, hyphens and underscores
both work):

| Name            | Vibe                                                |
|-----------------|-----------------------------------------------------|
| `default`       | Grayscale with cyan / green / yellow / red accents  |
| `terminal_blue` | Blue primary, tasteful light-blue highlights        |
| `matrix`        | Green-on-black, no surprises                        |
| `retro_danger`  | Red primary, yellow warnings, for cautious moods    |

Unknown preset names fall back silently to `default`. The app never
refuses to start because of a typo in your config.

## Overriding individual color roles

Each preset is a set of **roles**, not a fixed palette. You can
override any single role on top of a preset:

```
theme=matrix
theme.primary=blue
theme.highlight=lightcyan
```

Roles:

- `theme.primary`   — headings, active elements
- `theme.success`   — "done" states, positive affirmations
- `theme.warning`   — cautions, "preview only" banners
- `theme.error`     — errors, blocked states
- `theme.muted`     — secondary text, dim borders
- `theme.cactus`    — cactus mascot tint
- `theme.highlight` — cursor highlights, values in the progression HUD

Accepted color names (case-insensitive, `-` and `_` both work):

```
black red green yellow blue magenta cyan white
gray grey darkgray lightred lightgreen lightyellow
lightblue lightmagenta lightcyan
```

Unknown color names are silently ignored — the role keeps its preset
default.

## In-app indicator

Open **Settings** from the main menu. The right-hand panel shows the
current preset and a small `(overrides active)` flag when any
per-role override is in effect.

In-app theme editing is not supported in this phase. Editing the
config file directly is the source of truth.

## Design philosophy

- Sane defaults are always the best first experience.
- Customization should never make GitCactus harder to read.
- No screen is forced to use accent colors. Roles are opt-in at the
  render site — many screens still draw in raw grayscale because it
  fits the retro aesthetic better.
- If you want GitCactus to stay boring, do nothing. The default
  preset is `default`, which is exactly the look the app shipped with.
