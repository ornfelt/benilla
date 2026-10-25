---
name: benilla-upstream-sync
description: Bring the new commits on benilla's main branch - upstream benilla's, the fork's own and the fork's merges - into the two fork branches, gfx-dll-backend and trimmed-bevy, one commit at a time in commit-date order. Each commit is merged into each branch and investigated against that branch's own rules before it lands - gfx-dll-backend keeps the feature-off path identical to main and makes the gfx side follow, trimmed-bevy is gfx-only on a trimmed, vendored Bevy - so it lands as is, adapted, or not at all; a merge is skipped only when it changes nothing. Records every commit after the fork point in ledger.md (pending until looked at, with its origin, its handling and any special handling or upstream bug flag), can rebuild the ledger from git, and commits on both branches with no co-author.
disable-model-invocation: true
argument-hint: "[empty or continue | setup | status | fetch | rebuild | skip <sha> <reason> | replay <sha> | continue gfx-dll-backend | continue trimmed-bevy]"
---

# Sync upstream benilla into gfx-dll-backend and trimmed-bevy

## The goal

`main` is upstream benilla as the fork (`ornfelt/benilla`) has it: upstream's commits, plus the
fork's own commits and the merges that bring upstream in. Two fork branches came off it at the same
commit and have not taken anything from it since:

- **gfx-dll-backend** - benilla with an optional backend on my gfx DLL (C,
  `$code_root_dir/Code2/General/gfx/gfx_dll/gfx_benilla`). gfx is opt-in behind the `gfx` cargo
  feature. With it on, only the gfx DLL draws and handles the window and input (Bevy's
  `WinitPlugin` and `RenderPlugin` are never added). With it off, benilla is stock Bevy 0.18.1 on
  winit/wgpu, unchanged - the parity reference. The `gfx-dll-port` skill built it.
- **trimmed-bevy** - gfx-dll-backend made gfx-only, on a trimmed copy of Bevy, to make the C port
  (cenilla) possible. The `gfx` feature switch is gone and every `cfg(feature = "gfx")` is resolved
  to its gfx side; the wgpu path (the WGSL, every `RenderApp` / render-world half, the no-op wgpu
  device) is deleted; winit is out of Bevy's features; Bevy is vendored into `third_party/bevy`
  and trimmed to what benilla uses, with the same API, and whole crates are cut (`bevy_gltf`,
  `bevy_gilrs`, `bevy_anti_alias`, `bevy_post_process`, `bevy_picking`, ...). The trimming must not
  change behaviour: `crates/benilla-app/src/game_plugins/frame.txt` pins the schedule. Phase A of
  `c-port-plan.md`; `third_party/bevy/BENILLA.md` lists every cut.

**I want the new commits on `main` in both branches.** For each commit on `main` that a branch does
not have yet, this skill merges it into the branch and investigates whether and how it fits the
branch's behaviour, then lands it **as is**, **adapted** (the branch's rules reshape it, or the gfx
side has to follow it), or **not at all** (with the reason). Never just take a commit blindly. Each
branch must still work with the gfx DLL afterwards, but every change on `main` is attempted.

- **Fork commits are ported too.** A commit made on the fork's `main` (origin `fork`, see the
  ledger) is merged under the same rules as upstream's. Being fork-only - not in upstream benilla -
  is never a reason to skip one.
- **Merge commits are skipped only when they change nothing.** A merge on `main` whose
  `git show --remerge-diff <sha>` (cross-checked with `git show --cc <sha>`) is empty only joins
  its parents, which land as their own rows. A merge that resolved a conflict in code, or changed
  anything beyond its parents, is ported like any other commit.
- **Caught up means nothing behind.** Every commit the walk has passed - landed, adapted,
  `not-applicable`, `skipped` - ends up in the branch's ancestry, so GitHub never says the branch
  is behind `main` for a commit already dealt with. GitHub's "N commits behind" is
  `git rev-list --count <branch>..main`: a branch is level with `main` only when that is 0, and
  while it is behind, the count is exactly the commits not walked yet (`pending`, `asked` and what
  follows them). That is why every commit lands by `git merge` (a cherry-pick leaves the original
  out of the ancestry) and why a skipped no-change merge is recorded by an empty merge when no later
  merge brings it in (step 0, "At the end of the run"). Only the maintainer's push shows it on
  GitHub; `main` moving on GitHub afterwards makes the branch behind again until the next run.

`ledger.md` in this skill's folder records every commit on `main` after the fork point and how it
landed on each branch - including the ones not looked at yet.

## Where things are

`$code_root_dir` is `/home/jonas` on Linux and `C:\Users\jonas` on Windows unless the
`code_root_dir` environment variable says otherwise. The commands below are bash; on Windows they
run in Git Bash (the shader rebuild has a PowerShell twin, `compile.ps1`).

- **The repo:** `$code_root_dir/Code2/Wow/tools/benilla`, remote `origin`
  (`https://github.com/ornfelt/benilla`), the fork. `main` there is kept level with upstream
  benilla on GitHub by the maintainer, and also carries the fork's own commits (e.g. `3503592a`,
  q3_movement) and the merges that bring upstream in (GitHub's "Sync fork", a `git pull`), so
  `main` is **not linear**.
- **Upstream** is `https://github.com/samwhosung/benilla`, the remote `upstream` (setup adds it
  when missing; it is only ever fetched). Upstream moves only by squash-merged pull requests, so
  `upstream/main` is linear: one commit per pull request, subjects ending in `(#NNNN)`. It is the
  test for a row's origin, nothing is merged from it directly - `main` is what the branches take.
- **The fork point** of both branches is `ba7fe6e2` - "release 0.2.0 (#468)" (2026-09-27). Check it
  with `git merge-base main <branch>` on a setup run. Every commit on `main` after it gets a ledger
  row.
- **The gfx-dll-backend checkout** (called `$GFX` below): the checkout this skill runs from, on
  `gfx-dll-backend` - this file lives on that branch only. If the current checkout is on another
  branch or detached, use the worktree `git worktree list` shows on `gfx-dll-backend`, else create
  one (`git worktree add ../benilla-sync-gfx gfx-dll-backend`) and say so.
- **The trimmed-bevy worktree** (`$TB` below): the worktree `git worktree list` shows on
  `trimmed-bevy`, else create one -
  `git worktree add ../benilla-sync-trimmed trimmed-bevy` (with `-b trimmed-bevy ...
  origin/trimmed-bevy` when the local branch does not exist yet). Never switch the branch of an
  existing checkout to get there.
- **The gfx library**: `gfx_dll/gfx_benilla` (never edit `gfx_dll/gfx`), its own git repo at
  `$code_root_dir/Code2/General/gfx`. The shader compiler is `Code2/General/gfx/wc_compiler_rs`.

### The frozen reference must not move

cenilla is translated from trimmed-bevy **at the tag `trimmed-bevy-frozen`**, and its tools run a
binary built from that tag - `benilla/target/debug/benilla` by default
(`cenilla/tools/capture_sweep.sh`), else `cenilla/build/rust-ref/benilla`. So:

- **Never move, delete or recreate the tag `trimmed-bevy-frozen`.** The trimmed-bevy branch moving
  past it is expected; the tag stays where it is.
- **Never rebuild, clean or overwrite the reference binary.** Every cargo command this skill runs,
  in either checkout, uses its own target dir: `export CARGO_TARGET_DIR=target/upstream-sync`.
  Never `cargo clean`. `scripts/visual.sh` hardcodes `./target/debug/benilla-visual`; call the
  binary from the sync target dir instead.
- **Never switch branches in, build in or touch a checkout whose HEAD is the tag**
  (`git describe --exact-match --tags HEAD`) - that is the reference checkout.
- Each target dir holds a full Bevy build. **Check the free disk space before the first build of a
  run** (`df -h "$GFX" "$TB"`); under about 40 GB free, say so and ask before building.

## Scope guard

- Write only in `$GFX` (gfx-dll-backend) and `$TB` (trimmed-bevy), plus `gfx_dll/gfx_benilla` in
  the gfx repo when a gfx-side change truly needs the library (see "Changes to the gfx DLL").
- **Never commit to `main`, never push, never open a pull request** - the maintainer does that.
  Moving the local `main` ref by a fast-forward to `origin/main` is allowed (see "Fetching").
- Never edit cenilla, `gfx_dll/gfx`, the WoW install (read-only, `docs/METHOD.md`) or another
  checkout of the repo.
- `AGENTS.md` and `docs/METHOD.md` bind this work too: the reference first, measure never eyeball,
  prove the run, `WOW_UNATTENDED=1` / `WOW_NOSOUND=1` on scripted runs, the `.probe-identity`
  account only.
- This skill only brings `main` in - upstream's commits, the fork's own commits on `main` and its
  merges. It does not carry gfx-dll-backend's own new commits over to trimmed-bevy, finish the gfx
  port (the `gfx-dll-port` skill's job), or continue Phase A.

## Fetching

At the start of every run:

```bash
cd "$GFX"
git fetch origin
git fetch upstream
git rev-list --left-right --count main...origin/main     # behind / ahead
```

When `main` is strictly behind `origin/main`, fast-forward it - `git fetch origin main:main` (it
refuses anything but a fast-forward) - and report how many commits arrived. When `main` has
commits `origin/main` lacks, or the fetch fails, say so, work with the `main` that is there, and
change nothing about it. A failed `upstream` fetch only makes origins less certain - say so.

Then, before any merge (not in status mode), **add a `pending` row for every commit on `main` the
ledger has no row for** (see "The ledger"), so the ledger shows what is ported and what is not
even when the run stops early.

## Modes

Decide from the argument:

1. **Continue** - empty or `continue`. Setup first when `ledger.md` is missing. Then fetch, add
   the pending rows and sync the next commits in ledger order per "A run". `continue
   gfx-dll-backend` or `continue trimmed-bevy` works on that branch only (to let one catch up with
   the other).
2. **Setup** - `setup`. Check the fork point, add the `upstream` remote when missing, find or
   create both checkouts, write `ledger.md` from the template and fill it per "Picking up the
   current state" (every commit after the fork point, decided or pending), commit it on
   gfx-dll-backend, then carry on as continue.
3. **Status** - `status`. Fetch, then report per branch: merged through, how many commits remain
   (the pending rows plus the commits on `main` the ledger has no row for yet), the next one, the
   open questions, the open upstream flags, disk space, and per branch
   `git rev-list --count <branch>..main` (what GitHub shows as behind) with any skipped no-change
   merge it still lists. **Write nothing** (the `main` fast-forward aside).
4. **Fetch** - `fetch`. Do "Fetching", report, stop.
5. **Rebuild** - `rebuild`. Redo the ledger from git per "Picking up the current state", keeping
   the notes, Special handling entries and flags of rows that are still right, commit it on
   gfx-dll-backend, report what changed, stop.
6. **Skip** - `skip <sha> <reason>`. For the next unmerged commit of a branch that the maintainer
   decided not to land (usually the answer to an open question): merge it with `-s ours` on each
   branch where it is next, record it `skipped (<reason>)` with Handling `requested` and a Special
   handling entry quoting the reason, clear its question, commit.
7. **Replay** - `replay <sha>`. Redo a commit that already landed - adapted wrongly, left out,
   skipped and now wanted, or flagged for upstream and now to be fixed on the branches too. See
   "Replaying". The existing ledger row is updated, never duplicated.

## The ledger

`ledger.md` sits beside this file on gfx-dll-backend and is the only ledger, for both branches. It
is a hint: **git is the truth.** `git rev-list --reverse <branch>..main` lists exactly what a
branch still lacks, because every decided commit - including one that does not apply - is merged
(a skipped merge that changes nothing rides in with the next merged commit that descends from it,
or is recorded by an empty merge at the end of the run, see step 0 of "A run"; between those it is
the one decided commit still listed). Where the ledger disagrees with git, git wins and the rows
are rebuilt per "Picking up the current state" in the same run.

```markdown
# benilla upstream sync ledger

Maintained by the `benilla-upstream-sync` skill. A hint, not the truth: `git rev-list --reverse
<branch>..main` lists what a branch still lacks.

Source: `main` (origin `ornfelt/benilla`, the fork: upstream benilla's commits, the fork's own
commits and its merges; not linear). Upstream: `samwhosung/benilla` (remote `upstream`).
Fork point of both branches: `ba7fe6e2` release 0.2.0 (#468) (2026-09-27)
gfx-dll-backend merged through: `<sha>` <subject> - <n> remaining
trimmed-bevy merged through: `<sha>` <subject> - <n> remaining
`main` when last fetched: `<sha>` <subject> (<date of the run>)

Statuses (per branch): `pending` (not looked at yet), `as-is` (merged, the change unchanged,
nothing else needed), `adapted` (merged, and reshaped for the branch or followed by a gfx-side
change - the notes say what), `already-in` (the branch had the change already; the merge kept the
branch's version), `not-applicable` (merged with `-s ours`, nothing of it fits the branch - the
notes say why), `skipped` (a merge that changes nothing, or the maintainer decided not to land it;
a no-change merge recorded by an empty merge carries that merge's sha), `asked` (waiting on the
maintainer, nothing merged on that branch). A branch is level with `main` - nothing behind on
GitHub - when `git rev-list --count <branch>..main` is 0.

Origin: `upstream` (in samwhosung/benilla), `fork` (made on ornfelt/benilla, not in upstream),
`merge` (a merge commit on `main`).

Handling: `as-is` (taken as `main` has it on both branches, apart from the branch rules every
commit gets), `needed` (changed because the branch needed it), `requested` (changed, skipped or
redone because the maintainer asked). Anything but `as-is` has an entry under Special handling.

| # | Commit | Date | Origin | Subject | gfx-dll-backend | trimmed-bevy | Handling | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |

## Special handling

<one entry per row whose Handling is not `as-is`: `#<n> <sha> <subject>` - per branch what was
done and why; for `requested`, the maintainer's request in their own words>

## Flagged for upstream

<one entry per upstream bug flag: `#<n> <sha> <subject>` - the bug, a repro, a suggested fix,
where a PR belongs (samwhosung/benilla or ornfelt/benilla), state `open` or `closed (<the
maintainer's decision>)`>

## Open questions

<one bullet per `asked` commit: the branch, the question, the recommended answer, the size>
```

- **Every commit after the fork point has a row** - landed, skipped, `asked` or `pending`. A run
  first adds `pending` rows for the commits on `main` the ledger has none for (the fetch may have
  brought several, through a merge), so the file always shows what is ported and what is not.
- **Rows are chronological and never move.** New rows are appended in commit-date order, never a
  commit before its parent: `git log --reverse --date-order --format='%H %ct %s' ba7fe6e2..main`,
  minus the commits that already have a row. `#` keeps counting up; existing rows
  are never reordered, renumbered or removed. An old upstream commit that only arrives through a
  later merge is appended when it arrives, so its date can be older than the row above it - that is
  expected. **The ledger's row order is the merge order.**
- **Origin**: `merge` when the commit has more than one parent; else `upstream` when
  `git merge-base --is-ancestor <sha> upstream/main`; else `fork`. A fork commit that later
  reached upstream through a pull request comes back as a new squashed commit with its own row;
  the fork's original stays `fork`.
- **Handling** sums up both branches: `needed` when either branch adapted the commit,
  `not-applicable` it, or kept its own version over it (`already-in` where `main`'s version won
  unchanged stays `as-is`); `requested` when the maintainer asked for anything about it - a skip,
  a replay, a different adaptation, a fix of a flagged bug. `requested` wins over `needed`. A
  skipped no-change merge is `as-is`: skipping it is the rule, not a change.
- A branch cell is the status plus the branch's merge commit (`adapted 1a2b3c4`); a branch that
  has not reached the commit says `pending`.
- The **Notes** say what was adapted and why, what was dropped, which gfx-side files followed,
  what was verified on which platform and backend, and whether the change touched the wire
  format, a shader, the schedule (`frame.txt`) or the vendored Bevy. The Special handling entry
  carries the full story; the note can point to it.
- "merged through" (the last row before the branch's first row that is still `pending` or `asked`)
  and "remaining" are recomputed every run from git, not carried over: "remaining" is
  `git rev-list --count <branch>..main`, the number GitHub shows as behind.

### Picking up the current state

The ledger can be rebuilt from git alone - on setup, in `rebuild` mode, and for the rows concerned
whenever the ledger disagrees with git or the code. For every commit in `git log --reverse
--date-order ba7fe6e2..main` (a full rebuild numbers the rows in that order), origin as above, then
per branch:

1. **In the branch's ancestry** (`git merge-base --is-ancestor <sha> <branch>`): find the earlier
   sync - `git log --format='%h %s' --grep='<short sha>' <branch>` finds this skill's merge or
   replay commit (`Upstream|Fork|Merge <sha> ... merged as is` / `merged, adapted` / `not
   applicable` / `skipped` / `replayed`), whose wording gives the status. With no such commit,
   the change came in by another merge of `main` (`git log --merges --ancestry-path --reverse
   <sha>..<branch>`, the first one); read that merge's resolution of the commit's files and judge
   `as-is` or `adapted`, noting `rebuilt, merged outside this skill (<merge sha>)`.
2. **Not in the ancestry**: check whether the change is already in the branch's code - an earlier
   cherry-pick or hand port (`git log --grep='<short sha>' <branch>`, a matching `git patch-id`
   among the branch's own commits since the fork point), or the diff already applied
   (`git show <sha> | git apply --check -R` in that checkout; for a merge, its `--remerge-diff`).
   If it is, the row stays `pending` with the note `already in the code (<how found>) - expect
   already-in`; the walk still merges it, so git agrees. Otherwise `pending`.
3. A merge whose `--remerge-diff` is empty is `skipped (merge, changes nothing)` on both branches,
   whether or not a later merge has brought it into the branch yet.

Keep the notes, Special handling entries, flags and open questions of rows the rebuild confirms;
for a row it changes, say in the run's output what it was and what it is now.

## A run

**How a commit is applied.** Merge it as is first and let git apply it. Then resolve the
conflicts, and make only the manual fixes the branch's own behaviour needs: gfx-dll-backend's gated
gfx side (step 2), trimmed-bevy's gfx-only code and trimmed Bevy (step 3). Everything else stays as
`main` has it. Always land a commit with `git merge`, never `git cherry-pick`: a cherry-pick makes
a new commit, so `<branch>..main` would still list the original and the next merge of `main` would
apply its change again. A cherry-pick is only the starting point of a replay (see "Replaying"),
where the commit is already in the branch's ancestry.

**Order.** Take the `pending` rows in ledger order - commit-date order, parents first - each
through both branches. Upstream, fork and merge rows are walked alike.

**Size.** Go on until about 10 commits are done or the adaptation work this run (the code written
beyond the merge itself) reaches about 1500-3000 lines, whichever comes first. Skipped no-change
merges do not count. Stop earlier at a good stopping point or at the first question. Never stop in
a state that does not build, and never leave one commit merged on gfx-dll-backend and half-done on
trimmed-bevy.

**Before the first merge:** both checkouts clean (`git status --short` empty - otherwise report
what is there and stop), the fetch done and the pending rows added, the disk checked, the local
server checked (`timeout 2 bash -c 'echo > /dev/tcp/localhost/3724'`, `WOW_HOST` if set).

Per commit `$U` (upstream, fork or merge - the steps below say "upstream" for `main`'s side):

### 0. A merge commit

```bash
git -C "$GFX" show --remerge-diff "$U"
git -C "$GFX" show --cc "$U"
```

Its parents have rows above it, so they are already through. **Empty** - it only joins its
parents: record it `skipped (merge, changes nothing)` on both branches, Handling `as-is`, and go
on with the next row; no merge of its own mid-walk. It enters each branch with the next merged
commit that descends from it. When the run ends with no such commit merged - the merge is `main`'s
tip, or the run stops right after it - it is recorded by an empty merge then ("At the end of the
run"), so the branch is not left behind `main` for it. **Not empty** - a conflict
resolved in code, or anything beyond its parents: it is ported like any other commit, through
steps 1-5, where "the change" is its `--remerge-diff`. The merge into a branch then brings exactly
that.

### 1. Read it whole

```bash
git -C "$GFX" show --stat "$U"
git -C "$GFX" show "$U"
```

Classify the change before merging: which crates and areas it touches, and which **gfx triggers**
it hits (next section). Most upstream commits are UI script, Lua, formats, protocol or game logic
and need nothing beyond the merge on either branch - but establish that from the diff, not the
subject.

**Upstream bug flags.** When the commit's own change has an evident, reproducible bug - in the
lines it adds or changes, not in the code around them - flag it:

- The commit is still merged exactly as `main` has it (the feature-off path stays `main`'s, and
  trimmed-bevy keeps benilla's code as upstream wrote it), and the run carries on. Never fix it on
  a branch on your own.
- Add an entry under "Flagged for upstream": the bug, a repro (the input or steps and what goes
  wrong, a test or a capture where one shows it), a suggested fix, and where a pull request
  belongs: `samwhosung/benilla` for an `upstream` commit, `ornfelt/benilla` for a `fork` commit or
  a merge's resolution. The note names it. Never open the pull request.
- It stays `open` until the maintainer decides. When they want the branches fixed too, `replay
  <sha>` redoes the commit with the fix, Handling becomes `requested` and the Special handling entry
  quotes their request; the flag is closed with their decision either way.

### 2. gfx-dll-backend

```bash
cd "$GFX"
git merge --no-ff --no-commit "$U"
```

Resolve conflicts, then apply the branch's rules:

- **The feature-off path is upstream, exactly.** Every upstream change lands as upstream wrote it,
  and a fork commit's or a merge's change lands as `main` has it.
  A conflict with a gfx-gated block keeps both: upstream's code as upstream wrote it, the gated
  gfx block beside it, updated to the new code where it mirrors it. Check it:
  `git diff "$U" -- . ':!crates/benilla-gfx' ':!.claude' ':!GFX.md' ':!c-port-plan.md'
  ':!gfx-tools-and-tests.md' ':!qa-2026-09-28.md' ':!.gitignore' ':!Cargo.lock'` must show only
  `cfg(feature = "gfx")` additions and the feature plumbing in the `Cargo.toml`s, and compared with
  the same diff against `$U^` before the merge, nothing new apart from gfx-side follow-ups. Those
  excluded files are the branch's own (its docs, `.gitignore` without the `/.claude/` rule, the
  lock with `benilla-gfx`'s dependencies); a new branch-only file is added to the list. The
  `Cargo.lock` is checked on its own: upstream's lock changes must all be there, plus only the gfx
  crates' entries.
- **The gfx side follows** where the change hits a gfx trigger, so that `--features benilla/gfx`
  draws and behaves as the feature-off build now does. That is the `gfx-dll-port` skill's parity
  rule: the same colours, blending, fog, depth order, UI layout, cursor, input and window behaviour.
- **Already in.** A fix the branch already carries (gfx-dll-backend fixed something that upstream
  fixed later in its own way) is resolved to one copy: upstream's version wins on the feature-off
  path, a branch-local duplicate is removed, and the status is `already-in` when nothing of
  upstream's commit remained to take. Match on intent, not text. Never apply a change twice.
- **Does not apply at all** (rare on this branch, since the feature-off path takes everything):
  `git merge --abort`, then `git merge --no-ff -s ours "$U"` with the reason in the message.

### 3. trimmed-bevy

```bash
cd "$TB"
git merge --no-ff --no-commit "$U"
```

The same commit, under trimmed-bevy's rules:

- **gfx only.** Upstream changes to the wgpu path (`.wgsl`, `RenderApp` / `ExtractSchedule` /
  render-world systems, pipelines, graph nodes, the no-op device) are not taken as code: their
  gfx-side follow-up is. Start from what step 2 wrote for gfx-dll-backend - the same
  `crates/benilla-gfx` change and `.gfxs` edits, the gated blocks resolved to their gfx side with
  no `cfg(feature = "gfx")` left (`git diff` in `$GFX` before its commit, or `git show <its merge>`).
  A modify/delete conflict on a file trimmed-bevy deleted keeps the deletion.
- **benilla's own code stays as upstream wrote it** (Phase A's rule: benilla unchanged apart from
  making gfx the only backend). When upstream code uses a Bevy API the trim removed, restore that
  piece in `third_party/bevy` from stock Bevy 0.18.1 (the cargo registry copy), with the same API
  and behaviour, and list it in `third_party/bevy/BENILLA.md` - do not rewrite benilla's code around
  the gap. A whole cut crate coming back, or a restore larger than a few hundred lines, is a
  question (see "When to ask").
- **Cargo.** A new dependency must not bring Bevy, wgpu or winit back from crates.io (the A5
  checkpoint): after a `Cargo.toml` / `Cargo.lock` change, `cargo tree -i wgpu`, `-i winit` and
  `-i bevy_ecs` must show only the vendored path crates or nothing. Upstream's Bevy feature
  changes map onto the trimmed workspace's feature list, never re-enabling `bevy_winit`.
- **The schedule.** When the merge adds, removes or reorders a system, set or ordering, run the
  frame test (`cargo test -p benilla-app game_plugins::frame`). If it fails, re-record
  (`WOW_FRAME_RECORD=1 cargo test -p benilla-app game_plugins::frame`) and read the `frame.txt`
  diff: **every changed line must name a system, set or ordering this upstream commit added,
  removed or moved.** A change anywhere else (a Bevy system moving, `ApplyDeferred` points
  shifting for no reason in the diff) means the trimmed scheduler diverges from stock - that is a
  question, not a re-record. The PostUpdate ambiguity ceiling moves with the count as upstream's
  change moves it; raising it beyond that is a question.
- **Already in** and **does not apply** work as on gfx-dll-backend. Here `not-applicable` is
  common for a commit that only changes the wgpu path where the gfx side needs nothing, and
  `already-in` for one whose gfx-side follow-up step 2 already brought.

### 4. Verify (per commit)

- `$GFX`: `cargo check --workspace --all-targets` and the same with `--features benilla/gfx`;
  `cargo test -p <each crate the commit touched>`.
- `$TB`: `cargo check --workspace --all-targets`; `cargo test -p <each crate the commit touched>`;
  the frame test when the schedule may have changed.
- **Test rebuilds use the `dev-fast` profile**: every `cargo test` above, the frame test (and its
  re-record) included, runs with `--profile dev-fast` - our crates at opt-level 0 with no
  debuginfo, so the rebuild after each merge is faster and needs less memory. `cargo check` stays
  on `dev` (it generates no code, so the profile buys nothing there), and so does
  `scripts/check.sh`, which takes no profile. Both branches got the profile directly, so when the
  walk reaches the fork's `8561deaa` (the same profile on `main`), its `Cargo.toml` part is
  already in: keep a single `[profile.dev-fast]`, since a duplicate table breaks the manifest.
- A shader edited: `crates/benilla-gfx/shaders/compile.sh <base name>` (`compile.ps1` on
  Windows) in each checkout, the compiled families committed with the source. **GPU safety**
  (from the `gfx-dll-port` skill's `progress.md`): a new or changed program runs first off the
  GPU - vk on lavapipe with the validation layer, GL on llvmpipe - and only then on the real GPU;
  afterwards `sudo -n dmesg | grep -cE "ring gfx timeout|GPU reset"` stays 0. Where no software
  device exists (Windows), say the program ran on the real GPU only.
- A gfx trigger hit: a capture of the affected view, wgpu against gfx on gfx-dll-backend
  (`.claude/skills/gfx-dll-port/tools/probe_ab.sh`, a `WOW_CAPTURE` scenario that shows it), and the
  same scenario on trimmed-bevy against gfx-dll-backend's gfx shot. Settle parity with numbers, as
  the `gfx-dll-port` skill does.

### 5. Commit (per commit, trimmed-bevy first)

Commit the trimmed-bevy merge first, then fill in the ledger row (both branches' cells and merge
shas, the Handling, a Special handling entry when it is not `as-is`, any upstream flag) in `$GFX`
and commit the gfx-dll-backend merge with the ledger in it - one commit per commit on `main` per
branch, nothing left uncommitted at the end of the run. The run's new pending rows and skipped
no-change merges ride along in the next gfx-dll-backend commit; when no merge follows, they go in a
ledger-only commit on gfx-dll-backend (`benilla-upstream-sync ledger: <n> new pending rows, <n>
no-change merges skipped`). Stage explicit paths.

**`.claude/` is not gitignored on these branches**, unlike upstream, whose `.gitignore` ignores
`/.claude/` as each person's own. Keep it that way: a merge that touches `.gitignore` keeps
upstream's other changes but never brings the `/.claude/` rule (or its comment) back.

The message is a merge commit's, in benilla's style - one sentence saying what landed and how -
naming the commit on `main` (`Upstream <sha> (#NNNN)`, `Fork <sha>` or `Merge <sha>` by its
origin), followed by what was verified and on which platform and backends. **No `Co-Authored-By`
and no other trailer.**

```text
Upstream 05a9be68 (#437) merged as is - UnitBuff and UnitDebuff answer for party1-4 and partypet1-4; no gfx side (UI script only); cargo check with and without gfx and the benilla-ui tests green, built on Linux only
```

```text
Upstream 9c1d2e3f (#1012) merged, adapted - the terrain shader's new shadow term is ported to terrain.fs.gfxs and the four families rebuilt; the Elwynn capture matches wgpu within the known GL edge pixels on x11 gl4 and vk (lavapipe first), built and run on Linux only
```

```text
Fork 3503592a merged as is - Quake 3 movement behind the q3_movement feature (off by default); no gfx side (movement only); cargo check with and without gfx and the touched crates' tests green, built on Windows only
```

### At the end of the run

- `scripts/check.sh` in each checkout (with the sync `CARGO_TARGET_DIR`), the output teed to a file
  and grepped. A failure caused by this run's merges is fixed in a follow-up commit on that branch
  that names the upstream commit it belongs to, and the ledger notes it. Pre-existing breakage is
  reported, not fixed.
- When the local server is up and `.probe-identity` exists: `scripts/smoke.sh` on the gfx build
  of each branch the run changed beyond as-is merges. Without a probe account, never log in - say
  so.
- **No skipped merge left behind.** Per branch (trimmed-bevy first), list the rows the walk has
  passed that are still outside the ancestry: `git rev-list <branch>..main` minus the `pending` and
  `asked` rows and everything after the branch's first one. Only skipped no-change merges can be
  left; anything else is a ledger/git disagreement to rebuild. Merge the last of them (the others it
  descends from come with it; merge any that do not separately), then prove it changes nothing
  before committing: `git merge --no-ff --no-commit <sha>` then `git diff --cached HEAD --stat` is
  empty - a non-empty diff means a parent is not through, so abort and say so. Commit it as
  `Merge <sha> skipped, changes nothing - recorded with an empty merge so <branch> is not behind
  main for it`, put the merge sha in that row's cell (`skipped (merge, changes nothing) <sha>`),
  and recompute the remaining counts. When both branches have walked every row,
  `git rev-list --count <branch>..main` is 0 on each.
- `git -C "$GFX" status --short` and `git -C "$TB" status --short` empty.

## gfx triggers

An upstream change needs a gfx-side follow-up when it touches any of these. Find the counterpart
in `crates/benilla-gfx/src` (`material.rs`, `draw.rs`, `meshes.rs`, `images.rs`, `bevy_ui.rs`,
`ui.rs`, `window.rs`, `events.rs`, `input.rs`, `screenshot.rs`, `post.rs`, `egui.rs`, ...) and
`crates/benilla-gfx/shaders/src`:

- **Shaders:** any `.wgsl` (in `benilla-app`, `benilla-assets`, `benilla-world`) - the matching
  `.gfxs` pair (same base name, or the family `GFX.md` names), rebuilt with `compile.sh`.
- **Materials and drawing:** a `Material` / `UiMaterial` / `AsBindGroup` impl or its uniforms, a
  `ShaderRef`, a new material type, blend / depth / cull state, `RenderLayers`, camera order or a
  new camera, render-to-texture, `Msaa`, the light buffer and its blobs.
- **The render app:** `RenderApp`, `ExtractSchedule`, render-graph nodes, `ui_pass`, anything
  added under `get_sub_app_mut(RenderApp)` - gfx has no render app, so the effect must be done by
  `benilla-gfx` from the main world.
- **Window and input:** `Window` fields, `CursorOptions`, cursor icons, window modes, focus, IME,
  anything named `winit`, and new input messages - `window.rs` / `events.rs` mirror bevy_winit.
- **UI:** new bevy_ui node kinds, borders, slices, text features, `ImageNode` modes.
- **Instruments:** screenshots, `WOW_CAPTURE` scenarios (`capture/scenarios.rs`), `WOW_GPU_MS`,
  `WOW_DEPTH`, `WOW_PHASE`, the dev egui panel.
- **Boot:** `benilla-world/src/boot.rs`, plugin groups, `DefaultPlugins` changes - a new plugin
  that expects a render app may log errors under gfx (`boot.rs::LOG_FILTER`).
- **Dependencies:** Bevy, wgpu or winit versions or features.

A change the gfx side cannot match yet is not dropped: it lands on the feature-off path, the gap
goes in `GFX.md`'s "Known differences", and the ledger notes it as `adapted` with the gap named.

## Changes to the gfx DLL

When the gfx side needs the library to do something new, follow the `gfx-dll-port` skill's rules:
change `gfx_dll/gfx_benilla` (never `gfx_dll/gfx`), add functions rather than change signatures,
make the change in every window backend (sdl, glfw, x11, win32) and device backend (gl3, gl4,
gles3, vk, d3d11, d3d12) it applies to, say which could not be built here, rebuild, and commit it
in the gfx repo with a matching message and no co-author line.

## When to ask

Stop before merging the commit on that branch and ask - with a recommended answer, not only a
question - when:

- upstream bumps **Bevy, wgpu or winit** (for trimmed-bevy that means vendoring and trimming the
  new Bevy again; a likely recommendation is to hold trimmed-bevy at the last commit before the
  bump and carry on with gfx-dll-backend);
- the change needs a **cut Bevy crate back**, a large restore in `third_party/bevy`, or a
  `frame.txt` change that the commit's own diff does not explain;
- the gfx side would need a **redesign** of a `benilla-gfx` piece or a gfx DLL API change, not a
  follow-up;
- the intent is unclear from the diff, the pull request number and the subject;
- the change conflicts with a decision recorded in `GFX.md`, `BENILLA.md`, `c-port-plan.md` or
  the `gfx-dll-port` skill's `progress.md`.

Record the commit `asked` on that branch with the question under "Open questions", and commit
whatever the run finished before it. Merges go in ledger order, so **a branch stops at its first
asked commit** - a later one that descends from it would bring it in. gfx-dll-backend may go on
while trimmed-bevy waits (trimmed-bevy reuses its gfx side later from the merge commits); when
gfx-dll-backend waits, trimmed-bevy stops at the same commit. Make the judgement call yourself
when the answer is clear.

## Replaying

`replay <sha>` redoes a commit that is already merged: the upstream change is in the branch's
ancestry, so a second merge is a no-op. Bring the change in by hand - `git show <sha>` and
`git cherry-pick --no-commit <sha>` as a starting point, then the rules of step 2 or 3 - on top
of the branch as it is now, keeping what later upstream commits changed in the same lines. Commit
it as an ordinary commit, `Upstream <sha> (#NNNN) replayed - ...` (`Fork <sha>` / `Merge <sha>`
by origin), and update its ledger row and notes. A replay the maintainer asked for - a different
adaptation, a skipped commit now wanted, or the fix of a flagged upstream bug - sets Handling to
`requested`, and the Special handling entry says what was redone and quotes their request in their
own words; a flag it fixes is closed with that decision.

## Output expectations

Every run ends with:

1. **What came in** - the fetch result, the pending rows added (how many upstream, fork and
   merge, and any that arrived older than the rows above them), and per branch how many commits
   remain.
2. **Per commit** - sha, origin, pull request, subject, and per branch the status, the merge
   commit, the Handling, what was adapted or dropped and why, the gfx triggers it hit and the
   gfx-side files that followed. Skipped no-change merges in one line.
3. **Behaviour** - in plain words what the commits change for a player, and anything that touches
   the wire format, the shaders, the schedule or the vendored Bevy.
4. **Verification** - the commands run and their results, the captures and how they were judged,
   the platforms and backends built and run, what was not verified (another platform, a backend
   that cannot be built here, no live login).
5. **Questions** - the open ones, each with its recommended answer.
6. **Upstream flags** - the new ones in full (bug, repro, suggested fix, where the pull request
   belongs), and how many older ones are still open.
7. **Ledger changes** - rows a rebuild or a disagreement with git changed, what they were and what
   they are now.
8. **Commits** - per branch the commits made (short sha and subject), and in the gfx repo if any.
   Never pushed.
9. A last line: `Next: <origin> <sha> <subject>` - or that both branches are level with `main`
   (`git rev-list --count <branch>..main` is 0 on each, so GitHub shows neither behind once
   pushed), which is a correct run. **Do not invent work** when nothing is left.
