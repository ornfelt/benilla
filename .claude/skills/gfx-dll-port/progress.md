# gfx DLL port - progress

Branch: `gfx-dll-backend`. Status: milestones 1-6 done; milestone 7 in flight: every Linux pair
verified live (this run: the portraits fixed, smoke green on the gfx build, the character screen,
the world on all 12 pairs, frame cost, 4x MSAA); Windows left. The history of each piece is in
`git log main..HEAD`; this file is the current state only.

## Milestones

## Milestones

- [x] 1. **Scaffold.** `crates/benilla-gfx` (empty without its `gfx` feature, all deps optional):
  `ffi.rs` (the `gfx_dll.h` surface), `shader_def.rs` / `shader_loader.rs` (four embedded
  families, `WOW_GFX_SHADERS=<dir>` override), `backend.rs` (`WOW_GFX_WINDOW` / `WOW_GFX_DEVICE`,
  fail-at-boot naming the choices), `context.rs`, `events.rs`, `runner.rs` (poll ->
  `window::pump` -> `app.update()` -> `GfxRender` -> swap). `lib.rs::swap_in` disables
  `WinitPlugin` + `RenderPlugin`, adds `RenderMainWorldPlugin` and `GfxPlugin`, hooked in
  `benilla-world/src/boot.rs`. Features `benilla/gfx -> benilla-app/gfx -> benilla-world/gfx`,
  and `benilla-worldview/gfx`. `noop_device.rs`: a wgpu `noop` `RenderDevice` for main-world
  startup code (settled with the maintainer, 2026-09-27).
- [x] 2. **Window and input.** `window.rs` is bevy_winit 0.18.1's window half; the window is
  created at the logical size times the display's scale factor (or bevy's override), as winit
  does (`context.rs::physical_size`), resized before its first show. `WOW_GFX_INPUT_TRACE=<path>`
  logs every message and window call. Not applied: focus requests, runtime scale-factor change,
  IME.
- [x] 3. **GPU resources.** `GfxRender` sets `Pack`, `Prepare`, `Collect`, `Draw`, `Present`;
  `meshes.rs`, `images.rs` (BC1-5, sRGB, sampled variants), `data.rs` (storage buffers as RGBA32F
  data textures), `material.rs` (`GfxMaterialPlugin::<M>::new(describe)`), `draw.rs` (cameras by
  `order`, early / opaque / mask / sorted transparent / late draws, reverse-Z).
- [x] 4. **World shaders.** `wow_model`, the light buffer, FFXGlow and the GlowWave warp,
  `static_gx`, `terrain`, `wdl`, `liquid`, the sky family, the effect lane, raster depth bias,
  lit `StandardMaterial` (ambient + emissive; no maps, no two-sided normal flip).
- [x] 5. **UI.** The UI lane (`ui.rs`, `ui_pass/gfx.rs`), `UiQuadMaterial`, text glyph writes,
  bevy_ui (`bevy_ui.rs`: nodes, images, borders, slices, text, `MaterialNode`), gizmos.
- [x] 6. **The rest.** Image cameras, booth light buffers, screenshots / `WOW_CAPTURE`, window
  modes and level, `WOW_GPU_MS`, MSAA, the dev egui panel, window position / decorations /
  resizable, the `WOW_DEPTH` / `WOW_PHASE` instruments.
- [ ] 7. **Backend matrix and `GFX.md`.**
  - [x] GL clip control (`gfx_dll_set_clip_upper_left`: GL targets top-down as vk's; the
    bottom-up fallback under `WOW_GFX_DEPTH_REMAP=1`), vk offscreen passes load their
    attachments, the window's creation size, BC on gles3, `renderScale` through the backdrop
    target, the gles3 depth probe, a refused window size reconciled, sub-rect writes reaching
    every sampler variant (details in git).
  - [x] Every capture scenario A/B'd on every Linux pair; `GFX.md` (refresh its pair table when
    Windows is built).
  - [x] Live with the `.probe-identity` account (this run): `smoke_gfx.sh` green; every Linux pair
    (x11, sdl, glfw x gl3, gl4, gles3, vk) into the world; the character screen on x11 gl4 and vk.
  - [x] In-game portraits (this run): every booth rig drew black. A booth light buffer is on the
    rig-palette mirror list only, so its tint region is never written; on wgpu its zero word is
    the identity, but gfx carries tints as floats with -1 the identity (`gfx/light.rs::tint_value`)
    and 0.0 a black tint, which multiplies the light sum. `gfx/light.rs::light_texture` now makes
    every light data texture with its tint region at -1. Covers the round portraits, the paper
    doll, inspect, the pet doll, the stable and the dressing room (the pane buffer too).
  - [x] The gfx build logs no ERROR on boot (this run): with no render sub-app by design, the first
    `Extract*Plugin`'s one-time "Render app did not exist" error and bevy_gizmos_render's warning
    are filtered (`boot.rs::LOG_FILTER`, gfx only); it failed `smoke.sh`'s zero-ERROR check.
  - [ ] Windows: build win32 / d3d11 / d3d12 and run the matrix there.

## GPU safety (read before any live vk or GL run)

On 2026-09-27 a terrain draw hung this machine's GPU (amdgpu `ring gfx timeout`) and the reset
killed the maintainer's X session (a gfx vk descriptor bug, fixed). Since then:
- Every new or changed program runs first off the GPU: vk on lavapipe with validation,
  `VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json VK_INSTANCE_LAYERS=VK_LAYER_KHRONOS_validation
  timeout -s KILL 90 <bin>` (draw-time validation must be clean), and GL on llvmpipe,
  `LIBGL_ALWAYS_SOFTWARE=1`, no `GL_INVALID`. Then the real GPU. No Xvfb: both open on :0.
- Shader loops read from a data texture are bounded (`min(count, 256)`).
- After a real run, `sudo -n dmesg | grep -cE "ring gfx timeout|GPU reset"` must stay 0.

## Instruments

- Parity scene: `crates/benilla-gfx/examples/parity.rs` (`--features gfx`, `parity wgpu|gfx`):
  `PARITY_DEPTH="<x>,<y>;…"` prints gfx depth at pixels, `PARITY_ORIGIN=x,y,z` moves the scene to
  benilla-sized coordinates, `PARITY_MSAA`, `PARITY_EGUI=1` (`--features egui`),
  `PARITY_WINDOW`. It overrides the scale factor to 1.0. `OUT=<dir> tools/parity.sh x11:gl4 ...`.
- The client: `tools/probe_ab.sh <wgpu-bin> <gfx-bin> lvp:vk soft:gl3 x11:vk ...` with `OUT`,
  `SCENARIO` (a `WOW_CAPTURE` name) and the probe variables exported; one capture per build and
  pair, each build's own `WOW_CAPTURE_OUT` PNG (the exact aged frame, so animated scenes diff
  exactly: two wgpu runs are identical), the probe lines diffed (`KEEP='zzzz'` for images only;
  `REUSE_WGPU=1`, `DIFF_ONLY=1`, `SHOW=n`). `tools/parity_diff.py a.png b.png out.png`.
- Worldview: `tools/world_ab.sh <wgpu-bin> <gfx-bin> x11:vk ...` (`WOW_WORLDVIEW_AT`,
  `WOW_CLOCK`, `WOW_WIN`); `tools/fx_series.sh` for particles (not deterministic).
- `tools/validate_gfx.py crates/benilla-gfx/shaders`: glslang over every family's GL/GLES text
  and the d3d11 HLSL. `tools/window_props.sh`: what the WM made of a window.
- Binaries: `target/ab/wgpu/benilla` (feature off) and `target/ab/gfx/benilla` beside a copy of
  the current `libgfx.so` (it loads the one in its own directory). Both builds write
  `target/debug/benilla`: build one, copy it, build the other. Kill stray `import`s before a run;
  never `pkill -f` a pattern the killing command's own line contains.
- `tools/sweep.sh` (`WGPU`, `GFX`, `SCEN`; pairs as `probe_ab.sh`): every capture scenario, each
  diff also masked to outside the chat dock (`tools/rectmask.py`), validation / GL / panic lines
  counted. `tools/type_run.sh`: console lines typed into benilla's own window mid-capture
  (`xsend`; this Swedish layout's shifted `/`); both builds at one `WOW_CAPTURE_AGE`. Never run
  two windowed captures at once: the tiling WM splits the screen and the sizes stop matching.
- Live, as `.probe-identity`'s account (realm up; from the repo root): `tools/live_run.sh <bin>
  <out.png>` logs in, takes a `WOW_LIVE_SHOT` at `AT` s (default 25), exits at `EXIT`, and prints
  the ERROR / WARN / panic counts and the preflight line; extra env passes through
  (`WOW_GFX_*`, `WOW_MSAA`, `WOW_PROBE_LUA`, `WOW_LIVE_FPS`, `WOW_BOOTH_LOG`).
  `tools/charselect_shot.sh <bin> <out.png>`: the character screen, imported `SETTLE` s after the
  roster line. `tools/smoke_gfx.sh`: `scripts/smoke.sh` on the gfx build (default pair; smoke
  unsets `WOW_*`). The live game is not deterministic: diff a region, and a second wgpu run is the
  noise floor (two wgpu character screens differ by 6.8% of pixels >4, the animated model).

## Verified this run

Linux (Debian 13, X11 :0, awesome, Radeon 680M / Mesa 25.0.7), `gfx_benilla` unchanged (`1cde22d`),
vmangos on localhost, `.probe-identity`'s account (character Blue, level 60 Orc Shaman, Durotar),
`WOW_UNATTENDED=1 WOW_NOSOUND=1`; every run's preflight read first. No GPU reset (dmesg count 0).

- Portraits, reproduced at the reported spot first: x11 gl4, the player portrait a (0,0,0) disc in
  12 shots over 6 s. The booth's target read back through `gfx_dll_read_texture`: the clear colour
  (0.00034 linear) where the model is not, exactly 0 with alpha 1 where it is (the model's
  silhouette, framed right), so the draw ran and its light sum was zero. After the fix the
  portrait disc against wgpu's (x 50-110, y 33-93 at 1920x1080, % of pixels off by >1 / >4, max):
  vk 0.00 / 0.00 / 1 on x11, sdl and glfw; gl3, gl4, gles3 0.79 / 0.00 / 2 on all three windows
  (the GL sRGB rounding). Adjacent states: the character micro-button portrait back; the
  character window (`WOW_PROBE_LUA='ToggleCharacter("PaperDollFrame")'`): its portrait 0.00% >4
  (max 1-2), the stats 0.00%, the paper doll lit and coloured on vk and gl4 (7-10% >4 is the live
  idle pose: the pane renders every frame).
- `smoke_gfx.sh` (x11 gl4): SMOKE GREEN, 2 logins + the realm walk, 0 errors, 0 panics, install
  untouched (807 files). The first try failed on the one ERROR line, fixed above.
- The world on all 12 Linux pairs: in world, 0 ERROR, 0 panic, 1920x1080; x11 gl4's whole frame
  against a wgpu run 36 s apart 0.354% >4, the torch and the character's idle pose.
- Character screen (x11 gl4, vk vs wgpu): 7.2% / 8.4% of pixels >4 against 6.8% between two wgpu
  runs (the animated model and its particles); the list, logo and buttons <= 0.011% >4; the strip
  left of the model 0.017% / 0.063% >4, nothing >16 on vk.
- Frame cost (`WOW_LIVE_FPS=600 WOW_GPU_MS=1`, vsync off, Durotar, same scene counts): mean /
  p99 ms, GPU p50 ms: wgpu 18.25 / 20.23, 5.87; gfx vk 13.40 / 17.06, 6.46; gfx gl4 10.89 /
  12.89, 5.44. The gap is CPU (cpu_ms 33.0 wgpu, 15.1 vk, 13.5 gl4).
- `WOW_MSAA=4` live: the world view `4x` on gfx; whole frame >4 vk 0.93%, gl4 0.96%, wgpu vs
  wgpu 0.62%; the rest is live UI state (below), not drawing.
- `ui-questlog` (not gfx, closed): `WOW_PROBE_LUA` at 2.0 s in the capture reads the same state on
  both builds (player level 12, quests 2 and 3, green range 4), for which `GetDifficultyColor`
  is grey, as gfx draws; wgpu's frame keeps the colours of a `QuestLog_Update` that ran before
  the fixture's level landed (stock QuestLogFrame registers no `UNIT_LEVEL`). A fixture order race.
- Gates: see below.

Earlier runs (git): every capture scenario on every Linux pair; the worldview A/B; lavapipe and
llvmpipe runs; renderScale, clip control, depth, BC, MSAA, egui, window modes. Windows (win32,
d3d11, d3d12) never built; the d3d11 HLSL of every shader passes glslang.

## Build notes

- gfx library: `~/Code2/General/gfx/gfx_dll/gfx_benilla` (its own repo at `~/Code2/General/gfx`, `1cde22d`),
  `cd .../gfx_benilla && mkdir -p build && cd build && cmake .. -DCMAKE_BUILD_TYPE=Debug && make
  -j$(nproc)` -> `bin/Debug_x64/libgfx.so`. `benilla-gfx/build.rs` prefers `gfx_benilla` over
  `gfx` (`GFX_DIR`, `GFX_CONFIGURATION`) and copies it to `target/debug/` at build time only:
  after a library rebuild copy it there (and to `target/ab/gfx/`) by hand; check with `ldd`.
- What `gfx_benilla` adds over `gfx` (details in its git log): key physical / scancode, raw
  motion, scale factor, cursor / warp / icon; sRGB formats, GL depth attachment by format,
  `D32_SFLOAT`; vk sampler-slot mapping and descriptor fill; gl3/gles3 array depth; rasterizer
  depth bias; `set_texture_subdata`; GL sRGB framebuffer writes; `read_texture` (colour and
  depth); GL clears that ignore the write masks; window mode / level / position / centre /
  decorations / resizable, windows created unmapped; GPU timestamps; vk multisampling and
  resolve, `get_msaa_counts`; `copy_texture`; vk depth barriers;
  `gfx_dll_set_depth_zero_to_one` (gl3/gl4/gles3 `glClipControl` / `glClipControlEXT` once the
  version or extension says it exists; vk, d3d11, d3d12 already [0, 1]; d3d ones not built);
  `gfx_dll_device_supports_format` (GL / GLES by the S3TC, S3TC-sRGB, `EXT_texture_sRGB` and RGTC
  extensions, vk by `vkGetPhysicalDeviceFormatProperties`, d3d11 / d3d12 true; d3d not built);
  and `gfx_dll_set_clip_upper_left` (gl3/gl4/gles3 only, the same lookup with
  `GL_UPPER_LEFT`; NULL elsewhere, so false), vk offscreen render passes loading from the
  attachment layouts with the hand-over in `vk_begin_render_pass`, and `vmaDestroyAllocator` in
  `vk_dtr` (gfx repo `git log` for the hashes).
  Optional ops sit outside `GFX_DEVICE_OP_DEF` (d3d9 and jkg leave them NULL).
- Shaders: sources `crates/benilla-gfx/shaders/src/*.{vs,fs}.gfxs`; `crates/benilla-gfx/shaders/
  compile.sh [names]` writes the four families (compiler: `cd ~/Code2/General/gfx/wc_compiler_rs
  && cargo build --release`). HLSL reserves `point` and `line`. The compiler prints samplers in
  fragment stages only (a fetching vertex stage declares its own per backend), has no depth
  output, and `include` works only in the header (programs sharing a stage carry copies:
  `device_clip` is in every 3D vertex program).
- Build: `CARGO_INCREMENTAL=0 cargo build -p benilla --features gfx`; incremental caches filled the
  disk before. Old test executables in `target/debug/deps` (any executable file without an
  extension) are safe to delete for space: this run freed 48 GB that way. Disk: ~15-35 GB free; delete `target/debug/deps` files older than the round
  (`find target/debug/deps -maxdepth 1 -type f -mmin +180 -delete`) before the gates.

## Gates (this run)

`scripts/check.sh` (escalated to `gates.sh`): fmt ok, clippy ok; the run was then stopped by
Claude Code under memory pressure before the workspace tests, so test, test-no-install, the
lints, the player build and the enforcer did not run this round: rerun `CARGO_INCREMENTAL=0
scripts/check.sh` first. `cargo clippy --workspace --all-targets --features benilla/gfx -- -D
warnings` clean; `cargo test -p benilla-world --features gfx --lib gfx::light` 5 passed.
`smoke_gfx.sh` green; the feature-off `scripts/smoke.sh` not run.

## For the maintainer

- Live UI state that varies per run (seen on wgpu too, so not gfx): the yellow tutorial alert
  button above the action bar and the green equipped-item borders on two action buttons. Three
  wgpu logins showed the alert and no borders, one showed the borders and no alert; every gfx
  login (15+) the borders and no alert. A race in benilla's entry UI, perhaps paced by how fast the
  world loads (gfx enters faster); not traced.
- `ui-questlog`'s title colours: a capture-fixture order race (above), not gfx.
- Deferred (2026-09-27): mouselook's `CursorGrabMode::Locked` stays a real gfx grab.
- `ui-unitframes` (2026-09-28): the chat dock ends where the last `UIParent_ManageFramePositions`
  left it, and benilla re-runs that only on a screen size change, so a WM's late tile (winit) and
  an early one (gfx) end 40 UI units apart; on a non-tiling WM wgpu would land where gfx does.
  A benilla UI question (1.12 re-docks from the bars' OnShow / OnHide), not a gfx one.
- Window size is not a parity target: the WM may tile a gfx window. Under a size request this WM
  let winit's window take 1066x800 and kept the gfx window at its tile; gfx tells bevy the size it
  kept.
- Upstream candidates in `gfx_benilla` that are gfx bugs, not benilla needs: the x11 raw event
  twice under a grab, the x11 release-as-repeat heuristic, glfw's no-op `set_mouse_position`,
  sdl's late X1/X2, win32's screen-coordinate `set_mouse_position`, the GL depth attachment and
  `D32_SFLOAT` fixes, the vk slot mapping / descriptor fill, gl3/gles3 array depth, GL clears
  under the write masks, the vk depth barriers, vk passes loading their attachments, the VMA
  allocator never destroyed; API additions: depth bias, `read_texture`, `copy_texture`,
  `set_depth_zero_to_one`, `set_clip_upper_left`.
- The chat frame is a near-white opaque box (255,250,255) in every `ui-*` capture, on wgpu and
  gfx alike; live benilla draws it dark and translucent. A capture-fixture question, not gfx.
- The `shaders_gles3_dark` family is compiled without the Windows-only gamma hack.
- The install: `$wow_classic_dir` = `/home/jonas/Downloads/wow_classic` (build 5875), through the
  gitignored `WoW` link or `WOW_DATA`; read-only.

## Open problems

- `check_window_pinned` (`benilla-app/src/video.rs`) truncates the logical size at a fractional
  scale factor (5/3 here), so a `WOW_WIN` capture refuses sizes that are not multiples of 3, and
  under `WOW_DPI` winit still creates the window at the display's scale; both builds, not gfx.
- A GL without clip control still draws bottom-up, its 4x pattern mirrored (0.39% of the parity
  scene >1); measured through `WOW_GFX_DEPTH_REMAP=1`.
- radeonsi GL's sRGB store rounding in the UI lane: 1-2 levels over ~2.6% of a frame where the
  lane carries the world (0.79% of the portrait disc). Driver behaviour.
- About 100 isolated GL pixels in the worldview differ by >16: foliage alpha-test edges over sky
  decided the other way (radeonsi's GL compile at the threshold); not traced further.
- The vk swapchain pass loads `DONT_CARE`: the present covers it whole, but a second pass on the
  window in one frame would lose the first.
- Not A/B'd live: the bowstring / fishing line (gizmos; no unattended combat), depth-biased decals
  in the client, `WOW_PHASE=<uniqueId>` on a WMO, `AlwaysOnBottom` (awesome keeps no BELOW).
- Filler streams are per mesh.
- Latent, unreached in benilla today: a `Camera2d` on the window outside the lane without
  overlays and bevy_ui on a non-lane camera are skipped; a `Screenshot` of anything but the
  primary window is not taken.
- Deliberate: the effect lane clips every pane; wgpu's `MAX_CLIP_ROWS` (64) draws a pane past it
  unclipped, an overflow limit rather than behaviour.
- sdl and glfw grow a new window to 95% of the work area (`pick_window_size`) and clamp or place
  the client, not the frame; `WindowPosition::Centered` ignores its `MonitorSelection`.

## Next

Milestone 7's last item is Windows: build `gfx_benilla` (MSVC, `GFX.md`) and benilla with
`--features gfx`, fix what does not compile in the win32 / d3d11 / d3d12 code written blind (d3d
leaves the clip-origin op NULL: its targets run top-down already), run the capture matrix there
(`tools/probe_ab.sh` / `sweep.sh` need a Windows shell port or a by-hand run) and a live login per
pair, then refresh `GFX.md`'s pair table and mark the project done. On Linux, nothing gfx is open
that a run here can settle; the maintainer's items above are benilla UI questions.
