---
name: gfx-dll-port
description: Port all graphics/window usage in benilla to the gfx DLL (Code2/General/gfx/gfx_dll) behind the `gfx` cargo feature, leaving the existing Bevy winit/wgpu path working and as is. Reusable - every run reports the latest status, picks up where the last run stopped, lands roughly 1000-3000 lines of change and commits.
disable-model-invocation: true
argument-hint: "[status | continue | <milestone or topic>] - empty means continue where the port left off"
---

# Port benilla's graphics/window usage to the gfx DLL

## The goal

Port all graphics/window usage into using my gfx DLL, but put it behind a Rust/cargo feature flag
(`gfx`) so the old code is still working and left as is.

- The gfx DLL: `$code_root_dir/Code2/General/gfx/gfx_dll/gfx` (C, CMake/autotools; public headers in
  `include/gfx`, the DLL surface in `src/gfx_dll.h`, built into `bin/<Configuration>_x64`).
- Example usage via Rust: `$code_root_dir/Code2/General/gfx/wc_clean_new_rs`, in particular
  `src/gfx_interop.rs` (FFI bindings), `src/gfx_shader_def.rs` (the `.gfx` binary shader format),
  `src/gfx_shader_loader.rs` (loads `.gfx` shaders per backend) and `build.rs` (finds and copies
  the library). `$code_root_dir/Code2/General/gfx/wow-rs-gfx` is an earlier port of a wgpu app to gfx
  and shows the shader layout (`src/shaders*`, `sync_shaders.sh`); `wc_compiler_rs` compiles
  `.gfxs` sources into `.gfx` binaries.
- `$code_root_dir` is `/home/jonas` on Linux and `C:\Users\jonas` on Windows unless the
  `code_root_dir` environment variable says otherwise.

If the `gfx` feature is toggled on, ONLY the gfx DLL is used for the window, input and all
rendering (world, UI, text, post). That is enforced at runtime: Bevy's `WinitPlugin` and
`RenderPlugin` are never added, so neither winit nor wgpu opens a window or a device. bevy_render
types (`Mesh`, `Image`, `StandardMaterial`, the material extensions, UI nodes) may stay compiled in:
the gfx renderer reads them from the ECS, so the game code stays as it is. With the feature off,
benilla builds and runs exactly as before - no behaviour change, no new dependency.

The project is done when all window/device (graphics) usage is ported to the gfx DLL behind the
feature flag, at parity with the wgpu path, and `GFX.md` exists (see "Finishing").

## The rules for every run

1. **Read first.** `AGENTS.md` and `docs/METHOD.md` bind this work too (reference first, measure
   never eyeball, prove the run, the install is read-only, `WOW_UNATTENDED=1` / `WOW_NOSOUND=1`
   on scripted runs). Then read `progress.md` in this skill's folder and `git log --oneline
   main..HEAD` on the branch.
2. **Report the latest status** before touching code: the milestones done, the one in flight,
   what the last run left open, and what this run will do. With the argument `status`, stop there.
3. **Work on the branch `gfx-dll-backend`** (create it from `main` if it is missing). Never commit
   to `main`, never push, never open a pull request - the maintainer does that.
4. **Size.** Aim for 1000-3000 lines of code change per run. It is not a strict rule: if it is more
   logical to stop at 2000 lines because it is a good stopping point, stop there. Never stop in a
   state that does not build.
5. **Feature-gate everything.** New code lives in the new crate `crates/benilla-gfx` and behind
   `#[cfg(feature = "gfx")]`. An edit to existing code is only a cfg-gated addition or a gated
   alternative; the feature-off code path must stay byte-for-byte the same in behaviour. The
   feature is declared on each crate that needs it and forwarded from the `benilla` launcher
   (`gfx = ["benilla-app/gfx"]`) like `dev` and `tracy`.
6. **Match the wgpu path.** The target is benilla as it looks and behaves today: same colours,
   blending, fog, depth order, UI layout, cursor, input and window behaviour. Port the WGSL shaders
   (`find crates -name '*.wgsl'`) and the Bevy built-ins benilla relies on (PBR/unlit, UI, text,
   sprite) to `.gfxs`, and settle parity with numbers: the capture harness (`scripts/visual.sh`,
   `crates/benilla-app/src/capture/mod.rs`) and `benilla-visual` diff a wgpu shot against a gfx
   shot of the same view.
7. **Commit at the end of each run** with a suitable message for what was done, in benilla's
   style (one sentence, what the change does). **No co-author line**, no attribution trailer.
   Stage explicit paths; `.claude/` is gitignored, so a new file under this skill needs
   `git add -f`. Commit `progress.md` in the same commit.

## Backend selection

- `WOW_GFX_WINDOW` = `sdl | glfw | x11 | win32` and `WOW_GFX_DEVICE` = `gl3 | gl4 | gles3 | vk |
  d3d11 | d3d12`, read once at boot; the defaults match `wc_clean_new_rs` for the platform. An
  unavailable combination (`gfx_dll_has_window_backend` / `gfx_dll_has_device_backend`) fails at
  boot with a message naming the choices, never a silent fallback.
- Shaders: `.gfxs` sources committed under `crates/benilla-gfx/shaders`, compiled with
  `wc_compiler_rs` into one directory per shader family (`shaders`, `shaders_vk`, `shaders_d3`,
  and `shaders_gles3_dark` where the loader asks for it), picked at runtime as
  `gfx_shader_loader.rs` does. Keep a script beside them that recompiles all of them.
- `crates/benilla-gfx/build.rs` finds the library the way `wc_clean_new_rs/build.rs` does, only
  when the feature is on, prefers `gfx_dll/gfx_benilla` once it exists, and `GFX_DIR` overrides.

## Changes to the gfx DLL

If any changes are needed inside the gfx DLL to make this work, create a new `gfx_benilla` inside
`$code_root_dir/Code2/General/gfx/gfx_dll`, based on the original (`gfx_dll/gfx`, copied without its
build outputs) but including the changes needed to make it work with benilla. Never edit
`gfx_dll/gfx` itself.

- Feel free to adjust the gfx code as much as is needed (bug fixes, and changes needed to make
  benilla work as similar as possible to current benilla code), but prefer not changing the API
  too much: add functions rather than change existing signatures.
- A gfx change is made in every window backend (sdl, glfw, x11, win32) and every device backend
  (gl3, gl4, gles3, vk, d3d11, d3d12) it applies to, not only in the one you test on. Code you
  cannot build here (win32, d3d11, d3d12 on Linux) is written carefully and said so in the commit
  and in `progress.md`.
- Rebuild the library after each change and record the build command in `progress.md`.
- The gfx folder (`Code2/General/gfx`) is its own git repository: commit the `gfx_benilla` changes
  there too, with a matching message and no co-author line.

Also feel free to adjust the benilla code as needed (guarded behind the feature flag) to make the
gfx integration work.

## Milestones

The first run writes these into `progress.md` as a checklist, refined by an inventory of the code
(`grep` for `Material`, `Mesh3d`, `Camera`, `Window`, `winit`, `wgpu::`, `RenderApp`, `ui_pass`,
`bevy::ui`, `Text`); later runs tick them off and split them as needed.

1. **Scaffold.** `gfx` features, `crates/benilla-gfx` with the FFI bindings, shader format and
   loader ported from `wc_clean_new_rs`, `build.rs`. The app boots with a gfx window, a custom Bevy
   runner that polls gfx events and clears and swaps each frame, no winit/wgpu plugins, clean exit.
2. **Window and input.** gfx events become Bevy's input messages (keyboard, text, mouse buttons,
   motion, wheel, cursor position), window resize, focus, close, title, vsync, cursor visibility
   and grab, all kept in sync with Bevy's `Window` so the existing systems keep working.
3. **GPU resources.** `Assets<Mesh>` and `Assets<Image>` (including BC formats) to gfx buffers and
   textures on asset change, samplers, cameras and views extracted from the ECS, draw ordering,
   opaque/alpha-test/blend passes, depth.
4. **World shaders.** Terrain, WDL, liquid, models (M2/WMO), sky, sun/moon/stars, clouds, and the
   remaining world materials, particles, weather, fog and lighting.
5. **UI.** bevy_ui nodes, images, borders, text, the UI pass (`ui_pass.rs`), `AddUiMaterial`, the
   glue screens and the cursor.
6. **The rest.** Render-to-texture views, screenshots and the capture harness, the `WOW_GPU_MS`
   meter, MSAA, fullscreen and window modes, and the `dev` tools that draw (egui via
   `gfx_imgui` or an equivalent).
7. **Backend matrix.** Boot to the character screen and into the world on every window/device
   combination the platform has, fix what differs, then write `GFX.md`.

## Verification each run

- `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets -- -D warnings` with and
  without `--features benilla/gfx`; `scripts/check.sh` (the feature-off path must stay green).
- `cargo build -p benilla --features gfx` and a live boot on at least one backend pair
  (`WOW_UNATTENDED=1 WOW_NOSOUND=1`, the account in `.probe-identity`), plus one other pair when
  the run touched backend code. Read the preflight banner first.
- Check for a local server at the start of each run: `WOW_HOST` (default `localhost`), realm port
  3724 and world port 8085 (`timeout 2 bash -c 'echo > /dev/tcp/localhost/3724'`). When it is up
  and `.probe-identity` exists, the live path is the test for anything a capture cannot reach:
  `scripts/smoke.sh` on the gfx build, login to the character screen and into the world on the
  pairs the run touched, and the in-game checks `progress.md` lists (portraits, char-select,
  live frame cost). When it is up and there is no `.probe-identity`, never log in as another
  account (a login kicks a player, and the client refuses a scripted login on an undeclared
  account): say so in the status report and ask the maintainer for a probe account.
- Say in the commit and in `progress.md` what was verified, how the result was judged, and which
  platforms and backends were built and run.

## progress.md

`progress.md` in this skill's folder is the state the next run resumes from. Keep it short: the
milestone checklist, the backend pairs verified, how to build `gfx_benilla` and the shaders, open
problems, and a "Next" section naming the exact next step. Rewrite it each run - a current
snapshot, not a log; the history is git.

## Finishing

When every milestone is done, add `GFX.md` next to `README.md` on how to run benilla with the
new gfx feature (building the gfx library and the shaders, the feature flag, `WOW_GFX_WINDOW` /
`WOW_GFX_DEVICE`, the supported pairs per platform) and how to run it without it (the unchanged
default). Mark the project done in `progress.md` and commit.
