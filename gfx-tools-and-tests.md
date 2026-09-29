# gfx port: helper scripts and tests

What the gfx integration (the `gfx-dll-backend` branch) added to test and compare the gfx build
against the wgpu build: the helper scripts, the switches built into the client, and the unit
tests. Every script runs from the repo root.

## Where things live

- **`.claude/skills/gfx-dll-port/tools/`:** the comparison and live-run scripts. `.claude/` is
  gitignored, so a new file there needs `git add -f`.
- **`crates/benilla-gfx/`:** the parity example (`examples/parity.rs`) and the shader build
  script (`shaders/compile.sh`).
- **The unit tests** sit in the crates beside the code they cover (below).

Most scripts compare two builds of the same client:

- `target/ab/wgpu/benilla`: built without the feature (`cargo build -p benilla`)
- `target/ab/gfx/benilla`: built with it (`cargo build -p benilla --features gfx`), with a copy of
  the current `libgfx.so` beside it

Both builds write `target/debug/benilla`: build one, copy it out, build the other.

A window and device pair is written `window:device`, for example `x11:vk` or `sdl:gles3`. Two
special pairs run on the CPU instead of the GPU:

- `lvp:vk`: Vulkan on lavapipe, with the validation layer on
- `soft:gl3`: OpenGL on llvmpipe

## Scripts

### Building

| Script | What it does |
|---|---|
| `crates/benilla-gfx/shaders/compile.sh [names]` | Recompiles every `.gfxs` shader under `shaders/src/` (or only the named ones) into the four families the loader picks from: `shaders/` (gl3, gl4, gles3), `shaders_vk/` (SPIR-V), `shaders_d3/` (HLSL for d3d11 and d3d12) and `shaders_gles3_dark/`. Uses `wc_compiler_rs` (`$GFX_SHADER_COMPILER`, or the build under `Code2/General/gfx/wc_compiler_rs`) |
| `tools/validate_gfx.py crates/benilla-gfx/shaders` | Runs glslangValidator over the GL and GLES text and the d3d11 HLSL inside every compiled `.gfx` file, so shader code that cannot be built on this machine is still checked |

### Rendering comparisons: capture scenarios

The capture harness (`WOW_CAPTURE=<scenario>`) boots without a server, pins the clock and
camera, and writes one PNG. These scripts run the same scenario through both builds.

| Script | What it does |
|---|---|
| `tools/probe_ab.sh <wgpu-bin> <gfx-bin> <pairs...>` | One scenario (`SCENARIO`) through wgpu and through gfx on each pair, each build's own PNG, and the probe lines from the logs (`WOW_DEPTH`, `WOW_PHASE` and others) diffed side by side. `KEEP='zzzz'` compares images only; `REUSE_WGPU=1` keeps an existing wgpu run; `DIFF_ONLY=1` re-diffs; `SHOW=n` prints diff lines |
| `tools/sweep.sh <pairs...>` | Every capture scenario (`WOW_CAPTURE=list` plus the `ui-*` fixtures, or `SCEN`) through `probe_ab.sh` into `target/ab/sweep/<scenario>`: one line per pair with the diff against wgpu, the diff again outside the chat dock, and the count of validation, GL-error and panic lines. Binaries from `WGPU` and `GFX` |
| `tools/client_shot.sh <bin> <out.png>` | Takes a screenshot of the client window with ImageMagick `import` when a log line appears (`ON`, default the capture's "scene aged") or after `AT` seconds. Written before the gfx build could read back its own frames; `probe_ab.sh` covers it now |
| `tools/type_run.sh <bin> <out-prefix> [env...]` | A `ui-bag` capture that types `/console renderScale 1` and then `/console renderScale 2` into benilla's own window mid-run (through `xsend`), for testing live setting changes. The keys follow this machine's Swedish layout |

### Rendering comparisons: engine viewer and parity scene

| Script | What it does |
|---|---|
| `tools/world_ab.sh <wgpu-bin> <gfx-bin> <pairs...>` | `benilla-worldview` over the install at one fixed view (the Northshire overview, or `WOW_WORLDVIEW_AT`) with noon pinned, through both builds; each window is screenshotted after `AT` seconds and diffed |
| `tools/fx_series.sh <label> <bin>` | Particle coverage: `benilla-worldview` with every effect pixel solid magenta and depth off, five shots 2 s apart, and the magenta pixel counts and blob widths. Particles are not deterministic, so a backend passes when its counts fall inside wgpu's own run-to-run spread |
| `tools/parity.sh <pairs...>` | Draws the parity scene (below) through wgpu and through gfx on each pair, screenshots each window and diffs it against the wgpu one |
| `crates/benilla-gfx/examples/parity.rs` | A small Bevy scene drawn through either backend (`parity wgpu` or `parity gfx`, built with `--features gfx`), for checking basics without the game. Switches: `PARITY_DEPTH="x,y;..."` prints gfx depth at pixels, `PARITY_ORIGIN=x,y,z` moves the scene to benilla-sized coordinates, `PARITY_MSAA`, `PARITY_EGUI=1` (with `--features egui`), `PARITY_WINDOW` (window options) |

### Live runs against the local server

These log in as the checkout's `.probe-identity` account and need the realm up. Extra
environment passes through (`WOW_GFX_WINDOW`, `WOW_GFX_DEVICE`, `WOW_MSAA`, `WOW_PROBE_LUA`,
`WOW_LIVE_FPS`, `WOW_BOOTH_LOG` and others).

| Script | What it does |
|---|---|
| `tools/live_run.sh <bin> <out.png>` | One login into the world: a screenshot at `AT` seconds (default 25) through `WOW_LIVE_SHOT`, exit at `EXIT`, the log beside the PNG. Prints the exit code, the ERROR, WARN and panic counts, and the preflight line (character, level, map, position) |
| `tools/charselect_shot.sh <bin> <out.png>` | Logs in without picking a character, waits for the character list, then `SETTLE` seconds (default 8) for the model and scene, and screenshots the character screen |
| `tools/smoke_gfx.sh` | Runs `scripts/smoke.sh` (two logins with a logout between, the realm-list walk, the install read-only check) on the gfx build, by adding `--features gfx` to smoke's cargo commands. smoke.sh clears inherited `WOW_*` variables, so it runs the default pair (x11 and gl4 on Linux) |

### Window and input checks

| Script | What it does |
|---|---|
| `tools/live_input.sh <window> <device>` | Sends synthetic key, mouse and focus events to benilla's own window only (through `xsend`, plus one small pointer nudge that is undone) and writes the `WOW_GFX_INPUT_TRACE` trace of what the gfx build made of them |
| `tools/live_grab.sh <window> <device>` | Mouse grab check on the engine viewer, no game data needed: a right-drag asks for a locked, hidden cursor; the trace shows the grab, the raw motion under it and the release. The pointer is put back afterwards |
| `tools/window_props.sh <spec> <delay> <label> <bin> <wgpu or gfx>` | What the window manager made of the parity window under one `PARITY_WINDOW` setting: geometry, decoration hints and size hints. Run once per backend and compare |
| `tools/xsend.c` | A small X11 program (build with `cc xsend.c -lX11`) that sends key presses, key holds, mouse motion, clicks, drags, focus changes and the close request to one window only, so a scripted check never types or clicks anywhere else on the desktop |

### Image diff helpers

| Script | What it does |
|---|---|
| `tools/parity_diff.py a.png b.png out.png` | Per-pixel difference of two images: sizes, max and mean difference, the share of pixels off by more than 1, 4 and 16 levels, and an amplified (x8) difference image |
| `tools/masked_diff.py` | The same, counted only where the second image drew something (mask eroded past the glow's reach), so a background only one build draws does not count |
| `tools/rectmask.py a.png b.png x0,y0,x1,y1` | The diff shares outside a rectangle (by default the chat dock, which a late window-manager resize moves on wgpu), and the bounding box of what is left |

## Switches built into the client (gfx build only)

| Variable | What it does |
|---|---|
| `WOW_GFX_WINDOW`, `WOW_GFX_DEVICE` | Pick the window backend (`x11`, `sdl`, `glfw`, `win32`) and device backend (`gl4`, `gl3`, `gles3`, `vk`, `d3d11`, `d3d12`). An unavailable pair fails at boot naming the choices |
| `WOW_GFX_SHADERS=<dir>` | Load compiled shaders from a directory instead of the ones built into the binary |
| `WOW_GFX_INPUT_TRACE=<path>` | Log every input message and window call the gfx window produces |
| `WOW_GFX_DEPTH_REMAP=1` | Force the GL fallback for drivers without clip control (bottom-up rows and a depth remap in the shaders), to test that path on hardware that has clip control |

The existing client instruments (`WOW_CAPTURE`, `WOW_DEPTH`, `WOW_PHASE`, `WOW_GPU_MS`,
`WOW_LIVE_SHOT`, `WOW_LIVE_FPS`, `WOW_PROBE_LUA`, `WOW_MSAA`, `WOW_RESIZE`) work on the gfx
build too; the port made them read back through gfx.

## Unit tests

47 tests, in `crates/benilla-gfx` (41) and in the gfx half of `crates/benilla-world` (6). Run
them with:

```sh
cargo test -p benilla-gfx --features gfx
cargo test -p benilla-world --features gfx --lib gfx
```

They cover the parts that can be checked without a GPU or a window: data layouts, format
tables, coordinate conventions and the mapping from gfx events to Bevy's.

| Area | Tests | What they pin |
|---|---|---|
| Backend selection (`backend.rs`) | 3 | an unset variable takes the first available backend in priority order; names are case-blind; a missing or unknown backend fails naming the choices |
| FFI and shader files (`ffi.rs`, `shader_def.rs`, `shader_loader.rs`) | 5 | the event struct and shader file records match the C layout; a short or foreign shader file is an error; every shader source is compiled in every family; the family follows the device |
| Window and input (`context.rs`, `window.rs`, `input.rs`) | 8 | the logical window size scales and rounds as winit does; cursor images flip as asked; the swap interval follows the present mode; every gfx key has its own key code, placed where the key sits; logical keys and the extra mouse buttons match winit; typed text stops at the terminator |
| Images and meshes (`images.rs`, `meshes.rs`) | 5 | BLP sampler keys; BC mip levels round up to whole blocks; BGRA is swizzled into the sRGB format; every float vertex attribute has a gfx format; filler attributes repeat their default |
| Data textures (`data.rs`) | 2 | a write marks only the layers it touches; rows round up to whole layers and overflow is dropped |
| Drawing (`draw.rs`, `material.rs`, `target.rs`) | 8 | the ambient row matches Bevy's ambient light and exposure and ends the view block; rects count from the bottom unless the target runs top-down; the flags for bottom-up rows and depth remap stay apart; inverted culling swaps the face; the uniform ring aligns every block; a lit `StandardMaterial` packs Bevy's lighting inputs; MSAA sample counts stay within the CVar's range |
| UI, overlays, gizmos (`bevy_ui.rs`, `overlay.rs`, `gizmos.rs`) | 5 | the UI view maps the viewport from the top left; a clip pulls corners in; tiling repeats over the stretched image; an overlay draw rebases its indices; a gizmo strip closes on its NaN separator |
| Post, probes, screenshots, timer (`post.rs`, `probe.rs`, `screenshot.rs`, `timer.rs`) | 5 | only the claimed glow combine arms the gamma exit; the quarter-size target floors at 8 px; a packed depth texel is the float's bits red first; screenshot rows flip in place; a frame reads the timer pair it is about to write |
| Light and static-geometry data (`benilla-world`: `gfx/light.rs`, `static_gx/gfx.rs`) | 6 | the model, static and terrain shaders read every light region where the packer writes it; a tint word keeps black apart from "no tint"; a new light texture starts with "no tint" in its tint region (the portrait fix) and zero elsewhere; scalars pack four to a row; static-geometry record rows carry every bit exactly |

The rest of the gfx port was verified by the scripts above rather than unit tests: the capture
sweep of every scenario on every Linux pair, the worldview and parity comparisons, CPU runs on
lavapipe and llvmpipe, and live logins on all 12 Linux pairs.
