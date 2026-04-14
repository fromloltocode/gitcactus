# Manual validation — Rebase Portal Phase 2

Automated tests cover state transitions, preflight rejection of invalid
paths, and "no mutation before confirm" safety. They cannot cover the
actual rebase of a live repository. Run through this checklist on a
throwaway repo before merging Phase 2.

All commands below assume you are in a **disposable** directory. Do
**not** run this on work you care about.

## Setup

```bash
mkdir /tmp/gitcactus-rebase-test && cd /tmp/gitcactus-rebase-test
git init -q
git config user.email test@example.com
git config user.name "Test"

# Base history on main.
echo one > a.txt && git add a.txt && git commit -q -m "add a"
echo two >> a.txt && git commit -q -am "extend a"

# Create a feature branch with two commits.
git checkout -q -b feature
echo alpha > b.txt && git add b.txt && git commit -q -m "add b"
echo beta >> b.txt && git commit -q -am "extend b"

# Add a new commit to main so feature diverges.
git checkout -q main
echo three >> a.txt && git commit -q -am "third a"

# Back on feature.
git checkout -q feature

# Launch gitcactus.
gitcactus
```

## Case 1 — simple successful rebase

Expected:

1. Open **Branches** (`b` from menu or navigate).
2. Highlight `main`. Press `p` → portal preview opens.
3. Preview shows two commits from `feature` that will be replayed onto
   `main`. Portal diagram, source/target labels, and "Press Enter to
   confirm" are visible.
4. Press `Enter` → confirm dialog appears in red. Source/target names
   and commit count are correct.
5. Press `y` → animation plays (grey cactus hops across grey portals).
6. Status header reads "Rebase complete" at the end. No commits lost.
7. `git log --oneline -5` outside the app shows the feature commits
   now on top of main's new commit.

## Case 2 — conflicting rebase, then abort

Reset the test repo and create conflicting content:

```bash
cd /tmp/gitcactus-rebase-test
git checkout main
echo conflict-main > shared.txt && git add shared.txt && git commit -q -m "shared from main"
git checkout feature
git checkout -q main -- shared.txt
echo conflict-feature > shared.txt && git add shared.txt && git commit -q -m "shared from feature"
gitcactus
```

Expected:

1. Portal preview still shows the replay list.
2. Confirmation + `y` starts the rebase.
3. Screen flips to **Conflict — rebase paused** state. Status reads
   "Stopped at conflict". Cactus is shown fallen (red).
4. Next-steps panel lists two sections:
   - "What to do next (outside GitCactus)": open files, resolve
     markers, `git add <file>`.
   - "Then, here in GitCactus": `c` continue, `a` abort, `Esc` leave
     paused.
5. A small honest note reads: "GitCactus will not auto-resolve
   conflicts or auto-stage files."
6. Press `a` → runs `git rebase --abort` cleanly. Status flips to
   "Aborted" in green.
7. Outside the app: `git status` shows a normal branch, no rebase in
   progress.

## Case 2b — conflicting rebase, then resolve + continue (NEW)

Same setup as Case 2. Choose `c` instead of `a` after resolving:

```bash
# gitcactus is showing the Conflict state.
# In another terminal:
cd /tmp/gitcactus-rebase-test
$EDITOR shared.txt            # resolve the <<<<<<< / ======= / >>>>>>> markers
git add shared.txt
# Back in gitcactus: press c
```

Expected:

1. `c` runs `git rebase --continue` (refuses if no rebase is in progress;
   this is enforced at the git layer).
2. If no further conflicts: status flips to green "Rebase complete".
   Outside the app, `git log --oneline` shows the feature history on
   top of main.
3. If another conflict appears on a later commit: the Conflict state
   returns with a fresh stderr message prefixed
   "Another conflict after continue:". Repeat resolve → add → `c`.
4. If you press `c` **without** staging a resolution first: the
   Conflict state returns with a "Continue blocked:" message pulled
   straight from git. Nothing has mutated. You can still press `a`.

Expected honesty:

- At no point does GitCactus run `git add` on your behalf.
- At no point does GitCactus retry `--continue` silently.
- `Esc` always returns to Branches without touching the paused rebase.
- `c` pressed on a stale screen (no rebase in progress) produces a
  clean failure message, not a hang.

## Case 3 — already up to date

```bash
cd /tmp/gitcactus-rebase-test
git checkout feature
git rebase main  # make feature contain main
gitcactus
```

Expected:

1. Portal preview shows `Nothing to replay — source is up to date`.
2. No "Press Enter to confirm" prompt. Enter does nothing.
3. Esc returns to branches.

## Case 4 — blocked dirty tree

```bash
cd /tmp/gitcactus-rebase-test
echo dirty >> a.txt          # unstaged change
gitcactus
```

Expected:

1. Portal preview shows the commits that would replay.
2. A yellow warning reads: "Working tree has uncommitted changes. \
   Execution is blocked. Commit or stash first."
3. The "Press Enter to confirm" prompt is absent.
4. Enter does nothing; the screen stays in preview.

## Case 5 — same branch selected as target

Attempting this via the UI is blocked at the Branches screen (the `p`
hint does not appear on the current branch). If you reach the portal
anyway (e.g. by constructing state manually), the preview message
reads "Source and target are the same path."

## Case 6 — detached HEAD

```bash
cd /tmp/gitcactus-rebase-test
git checkout <some-commit-hash>   # enter detached HEAD
gitcactus
```

Expected:

1. If you somehow reach the portal (there is no current branch to
   select from, but preflight enforces this) the preview message
   reads "You are in detached HEAD state."
2. Execution is blocked.

## Teardown

```bash
cd /
rm -rf /tmp/gitcactus-rebase-test
```
