---
name: benilla-upstream-sync
description: Bring new upstream benilla commits (the main branch) into the two fork branches, gfx-dll-backend and trimmed-bevy, one upstream commit at a time, oldest first. Each commit is merged into each branch and investigated against that branch's own rules before it lands - gfx-dll-backend keeps the feature-off path identical to upstream and makes the gfx side follow, trimmed-bevy is gfx-only on a trimmed, vendored Bevy - so it lands as is, adapted, or not at all. Records every commit and how it landed in ledger.md, and commits on both branches at the end of the run with no co-author.
disable-model-invocation: true
argument-hint: "[empty or continue | setup | status | fetch | skip <sha> <reason> | replay <sha> | continue gfx-dll-backend | continue trimmed-bevy]"
---

# Sync upstream benilla into gfx-dll-backend and trimmed-bevy

## The goal

`main` is upstream benilla. Two fork branches came off it at the same commit and have not taken
anything from it since:

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

**I want the new upstream commits in both branches.** For each commit on `main` that a branch does
not have yet, this skill merges it into the branch and investigates whether and how it fits the
branch's behaviour, then lands it **as is**, **adapted** (the branch's rules reshape it, or the gfx
side has to follow it), or **not at all** (with the reason). Never just take a commit blindly. Each
branch must still work with the gfx DLL afterwards, but every upstream change is attempted.

`ledger.md` in this skill's folder records every upstream commit and how it landed on each branch.

## Where things are

`$code_root_dir` is `/home/jonas` on Linux and `C:\Users\jonas` on Windows unless the
`code_root_dir` environment variable says otherwise. The commands below are bash; on Windows they
run in Git Bash (the shader rebuild has a PowerShell twin, `compile.ps1`).

- **The repo:** `$code_root_dir/Code2/Wow/tools/benilla`, remote `origin`
  (`https://github.com/ornfelt/benilla`). `main` there is kept level with upstream benilla on
  GitHub by the maintainer. Upstream moves only by squash-merged pull requests, so `main` is
  linear: one commit per pull request, subjects ending in `(#NNNN)`.
- **The fork point** of both branches is `ba7fe6e2` - "release 0.2.0 (#468)" (2026-09-27). Check it
  with `git merge-base main <branch>` on a setup run; once the first commit is merged, the
  merge-base is the newest upstream commit the branch has.
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
- This skill only brings upstream in. It does not carry gfx-dll-backend's own new commits over to
  trimmed-bevy, finish the gfx port (the `gfx-dll-port` skill's job), or continue Phase A.

## Fetching

At the start of every run:

```bash
cd "$GFX"
git fetch origin
git rev-list --left-right --count main...origin/main     # behind / ahead
```

When `main` is strictly behind `origin/main`, fast-forward it - `git fetch origin main:main` (it
refuses anything but a fast-forward) - and report how many commits arrived. When `main` has
commits `origin/main` lacks, or the fetch fails, say so, work with the `main` that is there, and
change nothing about it.

## Modes

Decide from the argument:

1. **Continue** - empty or `continue`. Setup first when `ledger.md` is missing. Then fetch and
   sync the next upstream commits, oldest first, per "A run". `continue gfx-dll-backend` or
   `continue trimmed-bevy` works on that branch only (to let one catch up with the other).
2. **Setup** - `setup`. Check the fork point, find or create both checkouts, write `ledger.md` from
   the template, commit it on gfx-dll-backend, then carry on as continue.
3. **Status** - `status`. Fetch, then report per branch: merged through, how many commits remain,
   the next one, the open questions, disk space. **Write nothing** (the `main` fast-forward
   aside).
4. **Fetch** - `fetch`. Do "Fetching", report, stop.
5. **Skip** - `skip <sha> <reason>`. For the next unmerged commit of a branch that the maintainer
   decided not to land (usually the answer to an open question): merge it with `-s ours` on each
   branch where it is next, record it `skipped (<reason>)`, clear its question, commit.
6. **Replay** - `replay <sha>`. Redo an upstream commit that already landed - adapted wrongly,
   left out, or skipped and now wanted. See "Replaying". The existing ledger row is updated, never
   duplicated.

## The ledger

`ledger.md` sits beside this file on gfx-dll-backend and is the only ledger, for both branches. It
is a hint: **git is the truth.** `git rev-list --reverse <branch>..main` lists exactly what a
branch still lacks, because every decided commit - including one that does not apply - is merged.
Where the ledger disagrees with git, git wins and the ledger is fixed in the same run.

```markdown
# benilla upstream sync ledger

Maintained by the `benilla-upstream-sync` skill. A hint, not the truth: `git rev-list --reverse
<branch>..main` lists what a branch still lacks.

Upstream: `main` (origin, kept level with upstream benilla; linear, one commit per pull request).
Fork point of both branches: `ba7fe6e2` release 0.2.0 (#468) (2026-09-27)
gfx-dll-backend merged through: `<sha>` <subject> - <n> remaining
trimmed-bevy merged through: `<sha>` <subject> - <n> remaining
`main` when last fetched: `<sha>` <subject> (<date of the run>)

Statuses: `as-is` (merged, upstream's change unchanged, nothing else needed), `adapted` (merged,
and reshaped for the branch or followed by a gfx-side change - the notes say what), `already-in`
(the branch had the change already; the merge kept the branch's version), `not-applicable` (merged
with `-s ours`, nothing of it fits the branch - the notes say why), `skipped` (the maintainer
decided not to land it), `asked` (waiting on the maintainer, nothing merged on that branch).

| # | Upstream | Date | Subject | gfx-dll-backend | trimmed-bevy | Notes |
| --- | --- | --- | --- | --- | --- | --- |

## Open questions

<one bullet per `asked` commit: the branch, the question, the recommended answer, the size>
```

- One row per upstream commit, in `main`'s order. A branch cell is the status plus the branch's
  merge commit (`adapted 1a2b3c4`); a branch that has not reached the commit leaves its cell empty.
- The **Notes** say what was adapted and why, what was dropped, which gfx-side files followed,
  what was verified on which platform and backend, and whether the change touched the wire
  format, a shader, the schedule (`frame.txt`) or the vendored Bevy.
- "merged through" and "remaining" are recomputed every run from git, not carried over.

## A run

**Size.** Take upstream commits oldest first, each through both branches, until about 10 commits
are done or the adaptation work this run (the code written beyond the merge itself) reaches about
1500-3000 lines, whichever comes first. Stop earlier at a good stopping point or at the first
question. Never stop in a state that does not build, and never leave one upstream commit merged on
gfx-dll-backend and half-done on trimmed-bevy.

**Before the first merge:** both checkouts clean (`git status --short` empty - otherwise report
what is there and stop), the fetch done, the disk checked, the local server checked
(`timeout 2 bash -c 'echo > /dev/tcp/localhost/3724'`, `WOW_HOST` if set).

Per upstream commit `$U`:

### 1. Read it whole

```bash
git -C "$GFX" show --stat "$U"
git -C "$GFX" show "$U"
```

Classify the change before merging: which crates and areas it touches, and which **gfx triggers**
it hits (next section). Most upstream commits are UI script, Lua, formats, protocol or game logic
and need nothing beyond the merge on either branch - but establish that from the diff, not the
subject.

### 2. gfx-dll-backend

```bash
cd "$GFX"
git merge --no-ff --no-commit "$U"
```

Resolve conflicts, then apply the branch's rules:

- **The feature-off path is upstream, exactly.** Every upstream change lands as upstream wrote it.
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
shas) in `$GFX` and commit the gfx-dll-backend merge with the ledger in it - one commit per upstream
commit per branch, nothing left uncommitted at the end of the run. Stage explicit paths.

**`.claude/` is not gitignored on these branches**, unlike upstream, whose `.gitignore` ignores
`/.claude/` as each person's own. Keep it that way: a merge that touches `.gitignore` keeps
upstream's other changes but never brings the `/.claude/` rule (or its comment) back.

The message is a merge commit's, in benilla's style - one sentence saying what landed and how -
naming the upstream commit, followed by what was verified and on which platform and backends.
**No `Co-Authored-By` and no other trailer.**

```text
Upstream 05a9be68 (#437) merged as is - UnitBuff and UnitDebuff answer for party1-4 and partypet1-4; no gfx side (UI script only); cargo check with and without gfx and the benilla-ui tests green, built on Linux only
```

```text
Upstream 9c1d2e3f (#1012) merged, adapted - the terrain shader's new shadow term is ported to terrain.fs.gfxs and the four families rebuilt; the Elwynn capture matches wgpu within the known GL edge pixels on x11 gl4 and vk (lavapipe first), built and run on Linux only
```

### At the end of the run

- `scripts/check.sh` in each checkout (with the sync `CARGO_TARGET_DIR`), the output teed to a file
  and grepped. A failure caused by this run's merges is fixed in a follow-up commit on that branch
  that names the upstream commit it belongs to, and the ledger notes it. Pre-existing breakage is
  reported, not fixed.
- When the local server is up and `.probe-identity` exists: `scripts/smoke.sh` on the gfx build
  of each branch the run changed beyond as-is merges. Without a probe account, never log in - say
  so.
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
whatever the run finished before it. Merges go in order, so **a branch stops at its first asked
commit** - merging a later one would bring the asked one in with it. gfx-dll-backend may go on
while trimmed-bevy waits (trimmed-bevy reuses its gfx side later from the merge commits); when
gfx-dll-backend waits, trimmed-bevy stops at the same commit. Make the judgement call yourself
when the answer is clear.

## Replaying

`replay <sha>` redoes a commit that is already merged: the upstream change is in the branch's
ancestry, so a second merge is a no-op. Bring the change in by hand - `git show <sha>` and
`git cherry-pick --no-commit <sha>` as a starting point, then the rules of step 2 or 3 - on top
of the branch as it is now, keeping what later upstream commits changed in the same lines. Commit
it as an ordinary commit, `Upstream <sha> (#NNNN) replayed - ...`, and update its ledger row and
notes.

## Output expectations

Every run ends with:

1. **What came in** - the fetch result, and per branch how many upstream commits remain.
2. **Per upstream commit** - sha, pull request, subject, and per branch the status, the merge
   commit, what was adapted or dropped and why, the gfx triggers it hit and the gfx-side files that
   followed.
3. **Behaviour** - in plain words what the commits change for a player, and anything that touches
   the wire format, the shaders, the schedule or the vendored Bevy.
4. **Verification** - the commands run and their results, the captures and how they were judged,
   the platforms and backends built and run, what was not verified (another platform, a backend
   that cannot be built here, no live login).
5. **Questions** - the open ones, each with its recommended answer.
6. **Commits** - per branch the commits made (short sha and subject), and in the gfx repo if any.
   Never pushed.
7. A last line: `Next: upstream <sha> <subject> (#NNNN)` - or that both branches are level with
   `main`, which is a correct run. **Do not invent work** when nothing is left.
