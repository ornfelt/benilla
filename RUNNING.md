# Running benilla (the trimmed-Bevy gfx build)

This checkout is the `trimmed-bevy` branch at the tag `trimmed-bevy-frozen`. It is benilla on
Bevy trimmed to what benilla uses. It draws only through the gfx library: winit is not in the
build, and Bevy's `RenderPlugin` is never added. It is also the Rust reference that cenilla, the C
port, is translated from and compared against: `target/debug/benilla` is the binary cenilla's
capture sweep, schedule census and difftests run.

This page covers:

- running the client;
- picking the gfx window and device;
- the debug panel and the other dev tools;
- the test scripts.

[`GFX.md`](GFX.md) has the gfx details, and [`gfx-tools-and-tests.md`](gfx-tools-and-tests.md)
the gfx port's A/B tools. [`docs/CONTRIBUTING.md`](docs/CONTRIBUTING.md) has the full setup and
the unattended switches, and [`docs/METHOD.md`](docs/METHOD.md) the rules.

## Keeping the frozen reference intact

- **Use the debug build that already exists** (`target/debug/benilla`, beside its copy of
  `libgfx.so`) for anything cenilla is compared against.
- **The sources at the tag never change,** so `cargo build -p benilla` in debug only relinks what
  is stale.
- **Never `cargo clean`.** Before a fresh Rust build, run `../benilla-trimmed-bevy/prune_target.py`
  from this root: it keeps the newest build per crate and leaves `target/ab` alone.
- **A release build is separate** (`target/release`) and leaves the debug reference alone.

## Build and run

Build the gfx library first, as [`GFX.md`](GFX.md) says
(`Code2/General/gfx/gfx_dll/gfx_benilla`, `bin/<Release|Debug>_x64`). `crates/benilla-gfx/build.rs`
finds it, links it and copies it beside the binary. The search is `GFX_DIR`, else
`<code_root>/Code2/General/gfx/gfx_dll/gfx_benilla`. `GFX_CONFIGURATION` picks `Release` or
`Debug`, and `Release` wins when both are built.

```sh
WOW_DATA=/path/to/WoW/Data cargo run --release -p benilla
WOW_DATA="$wow_classic_dir/Data" cargo run --release -p benilla
```

```powershell
$env:WOW_DATA="$env:wow_classic_dir/Data"; cargo run --release -p benilla
```

| Command | What it builds |
|---|---|
| `cargo run --release -p benilla` | the optimized client (`target/release/benilla`) |
| `cargo play` | the same optimization plus incremental compilation, the edit-and-run loop (`.cargo/config.toml`) |
| `cargo run -p benilla` | the dev profile (`target/debug/benilla`, the cenilla reference): benilla's crates at opt-level 1, dependencies at 3 |
| `cargo build --profile ship -p benilla` | fat LTO, one codegen unit: the shipped binary |
| `cargo build --release -p benilla --no-default-features` | the player build: no `dev` feature |
| `cargo run --release -p benilla-worldview` | the engine viewer, with no game attached |

The binaries already built can also be run directly, without cargo:
`WOW_DATA=... target/debug/benilla`. On Linux, the rpath finds the `libgfx.so` beside it.

Features of `benilla-app`, forwarded by `benilla`:

- `dev` (default) compiles in the instruments (captures, probes), the egui debug panel, the perf
  HUD and the dev chords.
- `tracy` adds Tracy spans.
- `trace_chrome` writes every system's spans to `trace-<ts>.json` in the working directory.

### Where it reads and writes

- **The install** is only read. The first of these that exists wins:
  1. `$WOW_DATA`. Set and empty (`WOW_DATA=`) means "no install".
  2. A dev build's `<repo root>/WoW/Data`: a link named `WoW`, made with `ln -s /path/to/WoW WoW`,
     or on Windows `New-Item -ItemType Junction -Path WoW -Target C:\path\to\WoW`.
  3. `Data/` or `WoW/Data/` beside the binary.

  `cargo run -q -p benilla-formats --example where` prints what it resolves.
- **Local state** (`config.toml`, saved variables, macros, camera, chat and layout per
  character, screenshots, `AddOns/`) lives in one folder. The first of these wins:
  1. `$BENILLA_HOME`.
  2. A dev build's `<repo root>/benilla-config/`.
  3. `benilla-config/` beside the binary.

  A capture neither reads nor writes it.

### The server and the login

- `WOW_HOST` names the server. The default is `localhost:3724`, and `host:port` remaps the port.
  The login screen's Realmlist button sets it too, and remembers it.
- Credentials go in at the login screen. Setting `WOW_USER` and `WOW_PASS` skips it, and
  `WOW_CHAR` picks the character.
- `WOW_REALM` picks the realm by name, case-insensitively.
- `WOW_UNATTENDED=1` reconnects instead of waiting at a dialog, and exits non-zero on a login it
  cannot pass.
- `WOW_NOSOUND=1` runs silent.

A scripted run logs in only as the checkout's `.probe-identity`: `WOW_USER=`, `WOW_PASS=` and
`WOW_CHAR=`, one per line, never committed. The client refuses any other account on a scripted
run (`run_mode::account_guard`; `WOW_ALLOW_ACCOUNT=1` overrides it).

```sh
cargo run --release -p benilla                                            # log in by hand
WOW_USER=me WOW_PASS=secret WOW_CHAR=Name cargo run --release -p benilla  # straight into the world
```

## Pick the window and device

Two variables, read once at boot (`crates/benilla-gfx/src/backend.rs`):

| Variable | Values | Unset |
|---|---|---|
| `WOW_GFX_WINDOW` | `x11`, `sdl`, `glfw`, `win32` | the first the library has: x11, win32, glfw, sdl |
| `WOW_GFX_DEVICE` | `gl4`, `gl3`, `gles3`, `vk`, `d3d11`, `d3d12` | the first the library has: gl4, gl3, d3d11, gles3, vk, d3d12 |

A name the library was not built with stops the boot, listing the names it has. The boot log's
`gfx:` line names the pair it opened.

```sh
WOW_GFX_WINDOW=sdl WOW_GFX_DEVICE=vk cargo run --release -p benilla
WOW_GFX_WINDOW=glfw WOW_GFX_DEVICE=gl3 target/debug/benilla
```

```powershell
$env:WOW_GFX_WINDOW="win32"; $env:WOW_GFX_DEVICE="d3d12"; cargo run --release -p benilla
```

Off the GPU, through the drivers' own switches:

```sh
# Vulkan on lavapipe, with the Khronos validation layer
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json VK_INSTANCE_LAYERS=VK_LAYER_KHRONOS_validation \
  WOW_GFX_WINDOW=x11 WOW_GFX_DEVICE=vk target/debug/benilla
# OpenGL on llvmpipe
LIBGL_ALWAYS_SOFTWARE=1 WOW_GFX_WINDOW=x11 WOW_GFX_DEVICE=gl4 target/debug/benilla
```

[`GFX.md`](GFX.md) has the table of supported pairs and the known differences from upstream's
wgpu build. Its other switches:

- `WOW_GFX_SHADERS=crates/benilla-gfx/shaders` reads the compiled shaders from disk.
- `WOW_GFX_INPUT_TRACE=<file>` logs every input message and window call.
- `WOW_GFX_DEPTH_REMAP=1` forces GL's no-clip-control path.

Graphics settings:

- `WOW_MSAA` sets the MSAA sample count.
- `WOW_RENDER_SCALE` sets the render scale.
- `WOW_FARCLIP` sets the far clip.
- `WOW_NOVSYNC=1` uncaps presentation from boot.
- `WOW_UI_SCALE` sets the UI scale.

The same settings are in the in-game options and `/console`.

## The dev tools

In a `dev` build (the default), the dev plane is **Ctrl+Shift** plus a key, with Alt and Super
up (`benilla_world::modkeys`). The chords cannot be typed into an edit box:

| Chord | What it does |
|---|---|
| Ctrl+Shift+D | the debug panel, open and closed |
| Ctrl+Shift+P | the perf HUD: a pill with fps and the CPU cost per frame |
| Ctrl+Shift+I | inspect mode: hover the world to name what is under the cursor |
| Ctrl+Shift+F | free-fly: detach the camera from the player, and back |
| Ctrl+Shift+G | land here: ask the server to put the player on the ground below (needs GM rights) |
| Ctrl+Shift+M | mute and unmute |

The panel's footer lists them too. The player build (`--no-default-features`) has none of them.

**The debug panel** (`crates/benilla-app/src/debug_panel`) is an egui window drawn through gfx
(`bevy_egui`, `benilla-gfx`'s `egui` feature). Its sections:

- **World:** session, map, room.
- **Models**
- **Lighting**
- **Weather**
- **Sound**
- **Net**
- **Step-up**

`WOW_PANEL=1` opens it at boot. `WOW_PERF_HUD=1` does the same for the HUD.

```sh
WOW_PANEL=1 cargo run --release -p benilla
WOW_PANEL=1 WOW_CAPTURE=water-noon WOW_CAPTURE_OUT=/tmp/shot.png target/debug/benilla   # a shot with the panel
```

**The engine viewer.** `benilla-worldview` boots `benilla-world` alone, with no server and no
game. It opens over Northshire at noon. `WOW_WORLDVIEW_AT=<x>,<y>[,<z>]` opens it elsewhere, in
raw WoW coordinates. To move:

- Hold the left or right mouse button to look around.
- WASD moves and Space/C rise and sink, faster with Ctrl.
- The wheel sets the flying speed.

`WOW_WORLDVIEW_SHOT=<png>` takes one screenshot once the view has loaded and exits.
`WOW_WORLDVIEW_CHECK=<s>` is the gates' boot check.

```sh
cargo run --release -p benilla-worldview
WOW_WORLDVIEW_AT=-9460,62 WOW_GFX_DEVICE=vk target/debug/benilla-worldview   # Goldshire
```

### Instruments

The unattended switches are [`docs/CONTRIBUTING.md`](docs/CONTRIBUTING.md), "Running it
unattended". Each variable's comment at its `std::env::var` site says what it does. The common
ones:

| Variable | What it does |
|---|---|
| `WOW_CAPTURE=<scenario>` | server-less capture mode: pin the clock and camera, wait until the image stops changing, write one PNG to `WOW_CAPTURE_OUT`, exit. `WOW_CAPTURE=list` prints the world scenarios |
| `WOW_CAPTURE_STABLE=<frames>` | how many unchanged frames the shutter waits for |
| `WOW_CAPTURE_UI=1` | paint the player UI over a world scenario |
| `WOW_LIVE_SHOT=<png>`, `WOW_LIVE_SHOT_AT=<s>` | a screenshot of a live session (default at 12 s); `WOW_LIVE_SHOT_COUNT` makes a burst for `benilla-visual flicker` |
| `WOW_LIVE_FPS=<frames>` | the frame-time probe on a live run: samples that many frames, reports, exits |
| `WOW_PROBE_EXIT_AT=<s>` | exit after a number of seconds |
| `WOW_PROBE_LUA="<lua>"` | run a Lua chunk in the live UI on each world entry |
| `WOW_PROBE_CHAT="<cmd>"` | send one chat or GM command and log the reply |
| `WOW_RIG="<race> <class> <level> ..."` | (vmangos) find or create a rigged character with premade gear |
| `WOW_MOVE_TRACE=<file>` | the movement trace (`WOW_MOVE_TRACE_TAGS` filters it) |
| `WOW_DEPTH`, `WOW_PHASE`, `WOW_GPU_MS` | the depth, phase and GPU-time probes |
| `WOW_SCHED_CENSUS=1`, `WOW_ARCH_CENSUS=1` | print every schedule's system order, or the archetype creation order |
| `WOW_UNATTENDED=1`, `WOW_NOSOUND=1` | the unattended pair every scripted run sets |

Capture scenarios:

- **World:** `water-noon`, `water-night`, `felwood-noon`, `felwood-night`.
- **Glue screens:** `glue-login`, `glue-charcreate`, `glue-realmlist`.
- **Interface fixtures:** 32 `ui-*` scenes, listed in `crates/benilla-app/src/capture/scenarios.rs`:
  `ui-unitframes`, `ui-bag`, `ui-spellbook` and the rest.

```sh
WOW_CAPTURE=ui-unitframes WOW_CAPTURE_OUT=/tmp/ui-unitframes.png target/debug/benilla
WOW_CAPTURE=felwood-night WOW_CAPTURE_STABLE=200 WOW_CAPTURE_OUT=/tmp/felwood.png \
  WOW_GFX_DEVICE=vk cargo run --release -p benilla
```

Captures hold Blizzard-derived imagery, so keep them under the gitignored `target/` (for example
`target/visual/`).

### Small tools

| Binary | What it does |
|---|---|
| `benilla-visual` (`cargo run -p benilla-visual -- <cmd>`) | the image tools: `diff`, `diff-dir`, `flicker`, `hotspot`, `series`, `flow`, `edge`, `compose-dir`, `stat`, `crop` (run it without arguments for the usage) |
| `benilla-extract` (benilla-formats) | a CLI over the 1.12.1 MPQ patch chain: listing, extraction and the reference scans |
| `mpqx <data-dir> <path> <out>`, `blpx <in.blp> <out.ppm>` | one file out of the patch chain; a BLP's top mip as a PPM (or raw RGBA) |
| `benilla-auth <account> <password> <host>` | an SRP6 logon against realmd, printing the realm list |
| `benilla-world <account> <password> <host>` | enters the world as a character and tallies the updates; `--<probe>` flags check one wire each |
| `cargo run -p benilla-formats --example where` | where the install resolves |

## The test scripts

Every script runs from the repo root.

### `cargo test`: the unit and integration tests

```sh
cargo test --workspace                                     # everything
cargo test -p benilla-app ui_chat::                        # one crate, filtered
cargo test -p benilla-formats -p benilla-app --no-default-features --lib   # the player build's tests
WOW_DATA= cargo test --workspace                           # as on a machine with no install
```

Well over a thousand tests read the install and skip without it. Some more read a corpus of
vanilla addons (`BENILLA_ADDON_CORPUS`, or a `wow-addons-vanilla` link). `BENILLA_REQUIRE_DATA=1`
turns those skips into failures.

### `scripts/check.sh` and `scripts/gates.sh`: the gates

`scripts/check.sh` is the in-loop verify:

- `fmt` everywhere;
- `clippy` and `test` on the crates changed since `main`, and on their dependents;
- a change outside a crate runs the full chain.

`CHECK_FULL=1` goes straight to the full chain.

`scripts/gates.sh` is the full land chain, stopping at the first failure:

1. `fmt`
2. `clippy -D warnings`
3. `test`, with and without the install
4. `doc-links`, the dead doc-link check
5. `pass-span-lint`
6. the player build and its tests
7. the engine viewer's boot check, with and without the install

It opens a window, so it needs a display. A green run stamps the tree, and `GATES_FORCE=1` runs
the chain anyway.

### `scripts/smoke.sh`: the live login gate

It needs a running 1.12.1 server (`WOW_HOST`, default `localhost`) and the checkout's
`.probe-identity` (else the shell's `WOW_USER`, `WOW_PASS` and `WOW_CHAR`). It opens a window for
about a minute and does four things:

1. **The build:** `cargo build -p benilla`, the debug profile.
2. **The round trip:** log in, enter the world, log out, re-enter.
3. **The realm list:** walk the realm list.
4. **The log check:** scan the log, with the install watched read-only.

`WOW_BG` passes through and `WOW_SMOKE_KEEP=1` keeps the logs.

```sh
scripts/smoke.sh
```

### `scripts/visual.sh`: the capture A/B harness

Each scenario runs in its own client process, server-less, and `benilla-visual` diffs the PNGs.
It builds through `cargo` (the debug profile).

```sh
scripts/visual.sh list                  # the scenario names
scripts/visual.sh baseline              # capture into target/visual/baseline/
scripts/visual.sh diff [--fail <mae>]   # capture target/visual/candidate/ and diff against the baseline
scripts/visual.sh selfcheck [<mae>]     # capture twice from one build and demand identical shots
scripts/visual.sh capture <dir>         # capture every scenario into <dir>/
```

### Other scripts

- `scripts/cine.sh [id]` plays one cinematic on the probe character and prints a timeline.
- `scripts/soundprobe.sh` records the mix before and after the limiter; read it back with
  `scripts/soundprobe.py`.
- `scripts/summon-live.sh` is a two-client summon probe (`SUMMON_USER`, `SUMMON_PASS` and
  `SUMMON_CHAR`, a second GM account).
- `scripts/api-coverage.sh [--missing|--beyond]` reports how much of 1.12's global Lua surface
  the UI VM presents.
- `scripts/genmap.sh` regenerates `docs/MAP.md`.
- `crates/benilla-gfx/shaders/compile.sh [names]` recompiles the `.gfxs` shaders with
  `wc_compiler_rs`.

The gfx port's own A/B scripts against upstream's wgpu build (`sweep.sh`, `probe_ab.sh`,
`world_ab.sh`, `parity.sh`, `live_run.sh` and the image diff helpers `parity_diff.py` and
`masked_diff.py`) are in `.claude/skills/gfx-dll-port/tools/`, described in
[`gfx-tools-and-tests.md`](gfx-tools-and-tests.md).

### Against the C port

cenilla (`../cenilla`) compares its C client against this checkout's `target/debug/benilla`:

- `tools/capture_sweep.sh` runs every capture scenario through both builds, per window:device
  pair.
- `difftest/difftest.py` runs the same inputs through dump tools built from these crates and
  their C twins.

Its `docs/RUNNING.md` is this page's C counterpart.
