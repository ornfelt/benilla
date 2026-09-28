# gfx DLL port - progress

Branch: `gfx-dll-backend`. Status: milestones 1-6 done; milestone 7 in flight (this run: the
world at `renderScale` other than 1, the capture sweep of every scenario, the gles3 depth probe,
a refused window size reconciled). The history of each piece is in `git log main..HEAD`; this
file is the current state only.

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
  - [x] GL clip control (this run): `gfx_dll_set_clip_upper_left` at renderer creation puts GL
    on `glClipControl(GL_UPPER_LEFT, GL_ZERO_TO_ONE)` (GL 4.5 / `ARB_clip_control` /
    `EXT_clip_control`): clip z as Bevy's, and clip y +1 at a target's first row, so every GL
    target runs top-down as vk's (`GfxRenderer::upper_left`; views `top_down`, no clip flip, no
    culling swap, `misc.y` 0, gfx rects from the top, the post triangle's V as vk's, no depth
    probe row flip). The present and screenshot paths are unchanged: the window's first row is
    its bottom, so clip y -1 lands on its top. Where clip control fails (forced by
    `WOW_GFX_DEPTH_REMAP=1`) GL draws bottom-up with the old flips, the view block's `misc.y` bit
    2 set and every 3D vertex program ending in `device_clip` (`z' = 2z - w`). `misc.y` = 1 (rows
    bottom-up) + 2 (remap). Window, UI-lane and decode viewports now go through `gfx_rect` too.
  - [x] vk offscreen passes load (this run): `LOAD_OP_LOAD` from the attachment layouts, each
    image handed over in `vk_begin_render_pass` from its recorded layout (they loaded
    `DONT_CARE` from `UNDEFINED`, relying on RADV and lavapipe keeping the content). The swapchain
    pass keeps `DONT_CARE` (the present covers it whole). `vmaDestroyAllocator` before the device
    (the teardown leak).
  - [x] The window's creation size, above.
  - [x] Every Linux pair (x11, sdl, glfw x gl3, gl4, gles3, vk) through `glue-login`,
    `glue-charcreate`, `ui-bag`, `ui-char`, `ui-unitframes`, and the worldview (table below).
  - [x] BC on gles3: the device opens before the plugins finish (`runner.rs`) and
    `images::bc_supported` asks `gfx_dll_device_supports_format` for BC1-5, which becomes
    `CompressedImageFormatSupport`; gles3 had decoded BLPs on the CPU (`texpresso`), 5.7% of
    pixels 2 levels off.
  - [x] `GFX.md`; refresh its pair table when Windows is built.
  - [x] `renderScale` (this run): a claimed world view whose target (the backdrop size-carrier,
    `camera.physical_target_size()`) is not the window's size draws into `GfxRenderer::backdrop`,
    a scene target at the render size (`Dest::Backdrop`), made, re-made and dropped with the
    size; its glow runs at that size and the combine resamples it over the whole UI lane
    (bilinear, as bevy's combine reads the other-size main texture), the combine's output height
    in `wave.w` for the dither row. At the window's size the claimed view draws into the scene
    target as before (vk images byte-identical to the last run). FFXGlow quarter pairs idle for
    120 frames are dropped (`FfxPost::begin_frame`).
  - [x] Every capture scenario A/B'd (this run, table below).
  - [x] The depth probe on gles3 (this run): `depth_pack.{vs,fs}.gfxs` writes each depth texel's
    float bits into RGBA8 (`FfxPost::pack_depth`), which gles3 reads; `probe::read_packed`.
  - [x] A window size the WM refuses (this run): a size request with no resize back in 30
    frames sets `Window` to the size the window kept, with `WindowResized`
    (`window::kept_size`); `Window` had kept the refused size and the UI laid out for it.
  - [x] Sub-rect writes reach every sampler variant of the image (`GpuImages::write`); a variant
    made after writes warns once (none is today: variants are static-gx and terrain BLPs). The UI
    decode takes the scissored rasterizer on GL.
  - [ ] With a `.probe-identity` account: every pair to the character screen and into the world.
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

## Verified this run

- `renderScale`, reproduced first: `ui-bag` with `WOW_RENDER_SCALE=0.5` had the world in the
  top-left quarter (68% of pixels off by >1 against wgpu), 2.0 a quarter of it (68%). After (% of
  pixels off by >1 / >4; world at 510x507 or 2038x2028 in a 1019x1014 window, from the views
  line): 0.5 x11 vk 0.050 / 0.009, x11 gl4 1.97 / 0.005, lavapipe 15.0 / 0.125 (its usual level),
  llvmpipe gl3 15.0 / 0.126, the bottom-up fallback (`WOW_GFX_DEPTH_REMAP=1`) gl4 2.85 / 0.009;
  2.0 x11 vk 0.041 / 0.011, x11 gl4, sdl gl3, glfw gles3 2.84 / 0.010. Scale 1: vk identical to
  the last run's image (max 0), gl4 2.577 / 0.004 as before. Live: `/console renderScale 1` then
  `2` typed into benilla's own window (`xsend`) from 0.5 on both builds (`WOW_CAPTURE_AGE=1200`):
  the backdrop made at 510x507, dropped at 1, made at 2038x2028, the 127x126 / 254x253 quarter
  pairs dropped; the final frame vk 0.041 / 0.011, gl4 2.843 / 0.010 against wgpu's; lavapipe
  with validation silent through the switches.
- The capture sweep: all 34 other scenarios (`WOW_CAPTURE=list`'s four world ones, glue-realmlist
  and every `ui-*` fixture) on lavapipe with validation (no message), then x11 vk and x11 gl4 (no
  GPU reset). Outside the chat dock (a rect mask 0,680-600,960): every scenario <= 0.02% >4 on vk
  and gl4, but three whose frames anchor to the moved dock (bags in `ui-cooldown`, the "Chat
  Options" tooltip in `ui-chat-tabhover`, the default tooltip in `ui-tooltip-world`), and
  `ui-questlog` (below). Inside the dock, ~6.1% of pixels >16 in most `ui-*` fixtures: the known
  late-WM-resize layout (wgpu logs the late `render scale ... 1019x1014` resize, gfx none).
- `ui-questlog`: 0.705% >4, only the two quest title rows (41-384 x 225-274): wgpu draws the
  difficulty colours and the selection highlight, gfx both titles grey ("trivial"). Deterministic
  per build (repeat runs 0.000%), identical on x11, sdl and glfw; not drawing but Lua state
  (`QuestLog_Update`'s colours). A typed `/script QuestLog_Update()` did not change it, but the
  line was not shown to run. Open.
- gles3 depth probe (`ui-bag`, five pixels): llvmpipe and radeonsi within 4e-6 relative of wgpu
  at every pixel (as gl4 was), no `GL_INVALID`.
- Refused size: `WOW_RESIZE=640x480` on the gfx build logs `the window kept 1019x1014 against a
  request for 1066x800`, and the frame is at its no-resize numbers (it had laid the UI out for
  1066x800 in a 1019x1014 window). winit's window got 1066x800 under the same request.
- GL worldview seam pixels, characterised: the ~100 GL-only pixels >16 are isolated foliage
  alpha-test edges over sky (leaf on one path, sky on the other); vk matches wgpu there.
- FPS journal with `WOW_NOVSYNC=1` in `water-noon`: the capture paces both builds at 16.67 ms, so
  no frame-cost comparison came of it.

Linux (Debian 13, X11 :0, awesome, Radeon 680M / Mesa 25.0.7), `gfx_benilla` unchanged this run
(`1cde22d`), no account (no `.probe-identity`, no realm on :3724), `WOW_UNATTENDED=1
WOW_NOSOUND=1`. No GPU reset (dmesg count 0).

Earlier runs (git): GL top-down through clip control; vk passes load their attachments; every
Linux pair through glue-login, glue-charcreate, ui-bag, ui-char, ui-unitframes and the worldview;
gles3 BC; GL clip depth; MSAA, egui, window modes and properties, `WOW_GPU_MS`. Windows (win32,
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

`CARGO_INCREMENTAL=0 scripts/check.sh` (escalates to `gates.sh`): all gates ok (fmt, clippy,
test 161 s, test-no-install, doc-links, pass-span-lint, player-build, player-tests 287 s,
enforcer, enforcer-no-install); the first try was stopped by Claude Code under memory pressure
after player-build and rerun. `cargo clippy --workspace --all-targets --features benilla/gfx --
-D warnings` clean; `cargo test -p benilla-gfx --features gfx` 41 passed. No `smoke.sh`: no
`.probe-identity`. Install found, addon corpus absent (corpus tests skip).

## For the maintainer

- Deferred (2026-09-27): mouselook's `CursorGrabMode::Locked` stays a real gfx grab.
- `ui-unitframes` (2026-09-28): the chat dock ends where the last `UIParent_ManageFramePositions`
  left it, and benilla re-runs that only on a screen size change, so a WM's late tile (winit) and
  an early one (gfx) end 40 UI units apart; on a non-tiling WM wgpu would land where gfx does.
  Measured under "Verified this run". A benilla UI question (1.12 re-docks from the bars'
  OnShow / OnHide), not a gfx one.
- Window size is not a parity target: the WM may tile a gfx window. Under a size request this WM
  let winit's window take 1066x800 and kept the gfx window at its tile; gfx now tells bevy the
  size it kept (it had kept the refused size).
- Upstream candidates in `gfx_benilla` that are gfx bugs, not benilla needs: the x11 raw event
  twice under a grab, the x11 release-as-repeat heuristic, glfw's no-op `set_mouse_position`,
  sdl's late X1/X2, win32's screen-coordinate `set_mouse_position`, the GL depth attachment and
  `D32_SFLOAT` fixes, the vk slot mapping / descriptor fill, gl3/gles3 array depth, GL clears
  under the write masks, the vk depth barriers, vk passes loading their attachments, the VMA
  allocator never destroyed; API additions: depth bias, `read_texture`, `copy_texture`,
  `set_depth_zero_to_one`, `set_clip_upper_left`.
- The chat frame is a near-white opaque box (255,250,255) in every `ui-*` capture, on wgpu and
  gfx alike; live benilla draws it dark and translucent (the stock look is black, alpha 0). A
  capture-fixture question, not gfx; not traced.
- The `shaders_gles3_dark` family is compiled without the Windows-only gamma hack.
- The install: `$wow_classic_dir` = `/home/jonas/Downloads/wow_classic` (build 5875), through the
  gitignored `WoW` link or `WOW_DATA`; read-only.

## Open problems

- `check_window_pinned` (`benilla-app/src/video.rs`) truncates the logical size at a fractional
  scale factor (5/3 here), so a `WOW_WIN` capture refuses sizes that are not multiples of 3, and
  under `WOW_DPI` winit still creates the window at the display's scale; both builds, not gfx.
- A GL without clip control still draws bottom-up, its 4x pattern mirrored (0.39% of the parity
  scene >1); measured through `WOW_GFX_DEPTH_REMAP=1`.
- radeonsi GL's sRGB store rounding in the UI lane (measured, above): 1-2 levels over ~2.6% of a
  frame where the lane carries the world. Driver behaviour; matching it would mean encoding in
  the shader into a UNORM target, which breaks the lane's linear blending.
- About 100 isolated GL pixels in the worldview differ by >16 (0.010% against vk's 0.001%):
  foliage alpha-test edges over sky decided the other way (radeonsi's GL compile at the
  threshold); not traced further.
- `ui-questlog`'s title colours (above): Lua state that differs per build; next, log
  `UnitLevel("player")` and the quest levels at each `QuestLog_Update` in both builds.
- The vk swapchain pass loads `DONT_CARE`: the present covers it whole, but a second pass on the
  window in one frame would lose the first.
- Not A/B'd for want of a login: char-select, depth-biased decals in the client, image-camera
  MSAA, `WOW_GPU_MS` against the wgpu meter, `AlwaysOnBottom` (awesome keeps no BELOW for either
  build).
- Costs unmeasured in a busy scene: static-gx's per-run early draws, the effect lane's whole
  stream upload, bevy_ui atlas re-uploads, egui's whole-atlas re-upload. The capture paces its
  frames, so it cannot measure them; it takes a live run with `WOW_NOVSYNC=1`.
- Filler streams are per mesh. (The booth data textures are not a leak: both booth light
  buffers are made once and kept for the session on wgpu too.)
- Latent, unreached in benilla today (no log of any in 34 scenarios): a `Camera2d` on the window
  outside the lane without overlays and bevy_ui on a non-lane camera are skipped; a `Screenshot`
  of anything but the primary window is not taken.
- Deliberate: the effect lane clips every pane; wgpu's `MAX_CLIP_ROWS` (64) draws a pane past it
  unclipped, an overflow limit rather than behaviour.
- sdl and glfw grow a new window to 95% of the work area (`pick_window_size`) and clamp or place
  the client, not the frame; `WindowPosition::Centered` ignores its `MonitorSelection`.
- Log: each `Extract*Plugin` logs "Render app did not exist" once.

## Next

Milestone 7 continues; what is left needs what this machine lacks. With a `.probe-identity`
account: `scripts/smoke.sh` on the gfx build, then login to the character screen and into the
world on every Linux pair, the bowstring or fishing line, `gxMultisample 4`, `WOW_LIVE_FPS` with
`WOW_GPU_MS=1` and `WOW_NOVSYNC=1` on both builds, the warm pass on world entry,
`WOW_PHASE=<uniqueId>` on a WMO. On Windows: build `gfx_benilla` (MSVC, `GFX.md`) and benilla with
`--features gfx`, fix what does not compile in the win32 / d3d11 / d3d12 code written blind (d3d
leaves the clip-origin op NULL: its targets run top-down already), and run the capture matrix
there. Without either: the `ui-questlog` title colours (log the levels `QuestLog_Update` sees in
both builds). Then mark the project done.
