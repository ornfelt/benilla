# gfx DLL port - progress

Branch: `gfx-dll-backend`. Status: milestones 1-6 done; milestone 7 in flight (this run: the
Linux matrix through the server-less scenarios, `GFX.md`, BC textures on gles3 from the device,
the `ui-unitframes` dock traced to benilla's resize re-layout). The history of each piece is in
`git log main..HEAD`; this file is the current state only.

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
  - [x] GL clip depth (this run): `gfx_dll_set_depth_zero_to_one` at renderer creation puts GL
    on `glClipControl(GL_LOWER_LEFT, GL_ZERO_TO_ONE)` (GL 4.5 / `ARB_clip_control` /
    `EXT_clip_control`), so every device takes Bevy's clip z as is and no matrix carries a
    remap. Where it fails (a GL without clip control; forced by `WOW_GFX_DEPTH_REMAP=1`) the view
    block's `misc.y` has bit 2 and every 3D vertex program ends in `device_clip` (`z' = 2z - w`,
    naga's GL remap). `misc.y` = 1 (the target's rows run bottom-up) + 2 (remap).
  - [x] The window's creation size, above.
  - [x] Every Linux pair (x11, sdl, glfw x gl3, gl4, gles3, vk) through `glue-login`,
    `glue-charcreate`, `ui-bag`, `ui-char`, `ui-unitframes` (this run, table below).
  - [x] BC on gles3 (this run): the device opens before the plugins finish (`runner.rs`) and
    `images::bc_supported` asks `gfx_dll_device_supports_format` for BC1-5, which becomes
    `CompressedImageFormatSupport`; gles3 had decoded BLPs on the CPU (`texpresso`), 5.7% of
    pixels 2 levels off.
  - [x] `GFX.md` (this run); refresh its pair table when Windows is built.
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

## Verified this run

Matrix (`OUT=target/ab/m-<scenario>`, wgpu vs gfx, % of pixels off by >1 / >4; every pair
identical across x11, sdl and glfw; before the gles3 BC change):

| scenario | vk | gl4 = gl3 | gles3 (after BC) |
|---|---|---|---|
| glue-login | 0.007 / 0.002 | 0.076 / 0.003 | 0.076 / 0.003 (was 3.53) |
| glue-charcreate | 0.412 / 0.013 | 0.491 / 0.013 | 0.491 / 0.013 (was 7.81) |
| ui-bag | 0.048 / 0.003 | 2.60 / 0.004 | 2.60 / 0.004 (was 8.0) |
| ui-char | 0.065 / 0.002 | 2.64 / 0.003 | 2.64 / 0.003 (was 8.3) |
| ui-unitframes | 6.44 / 6.32 | 9.17 / 6.32 | (the dock offset, below) |

`WOW_NO_BC=1` proved the gles3 gap before the fix: gl4 with it equals old gles3 exactly (0.000%),
and wgpu against itself with it moves 6.6% of pixels by 1-4 levels. After: gles3 x11 / sdl / glfw
equal gl4 in every scenario run; llvmpipe gles3 clean (no `GL_INVALID`), lavapipe vk validation
only the known teardown leak; x11 gl3, gl4, vk, sdl vk / gl4, glfw vk unchanged.

The `ui-unitframes` dock: the chat box's white rows at x=200 are 703-850 in wgpu, 751-897 in gfx,
and 703-850 in gfx with one resize after load (`WOW_RESIZE=611x608`). gfx's image is identical
whether it asked 640x700 or the tile's size. wgpu gives the same image after a 4 px late resize
(`WOW_WIN=609x606`) as after 1066x1166 -> 1019x1014. So the dock is whatever
`UIParent_ManageFramePositions` last saw, and benilla re-runs it only on a screen size change
(`benilla-ui/src/script/mod.rs` `set_screen_size`): awesome's tile arrives after winit's first
frames (a late re-layout after `MultiBarBottomLeft` shows, empty, +55) and before gfx's first
frame (none, +15). Not a gfx difference; handed to the maintainer (below).

Linux (Debian 13, X11 :0, awesome, Radeon 680M / Mesa 25.0.7), `gfx_benilla` Debug, no account
(no `.probe-identity`), `WOW_UNATTENDED=1 WOW_NOSOUND=1`. No GPU reset (dmesg count 0).

The previous run (GL clip depth, window size):
- Parity scene depth (`PARITY_DEPTH`, 4 pixels): at the origin gl3 / gl4 (llvmpipe and RADV)
  equal lavapipe / RADV vk to 9 digits (was 3e-6); at `PARITY_ORIGIN=0,0,-9000` GL within 3e-5
  relative of vk (was x1.00098); the rest is the f32 world transform at 9000 yd, compiled
  differently for GL and SPIR-V (the scene is not camera-relative as benilla's world programs
  are). The forced remap (`WOW_GFX_DEPTH_REMAP=1`) on gl4 and llvmpipe gl3: 1e-6 of vk at the
  origin, the same 3e-5 far out. gles3 reads no depth (known).
- Client `ui-bag` (Northshire), `WOW_DEPTH` at 5 pixels x 3 frames + the phase lines: x11/vk and
  sdl/vk identical to wgpu in every line; x11/gl4, x11/gl3, llvmpipe gl3 within 4e-6 relative
  (<= 0.0003 yd at 95 yd; was x1.00098, ~0.05 yd at 50 yd); forced remap on gl4 within 1.5e-5;
  lavapipe 1-2 ulp. Images: vk 0.003% >4 / 0.002% >16; gl4 and gl3 0.004% >4 / 0.002% >16 (was
  0.029% >16: fewer coplanar flips); lavapipe and llvmpipe 0.3% >4 (software rasterizers).
- Regressions: `glue-login` x11/vk max 8, 0% >4; x11/gl4, glfw/gles3 0.002% >4; lavapipe,
  llvmpipe gles3 0.02% >4. `glue-charcreate` (image camera, GL rows top-down: `misc.y` 0 now)
  x11/vk, x11/gl4, glfw/gles3 0.003% >4; software 0.107% >4. Validation: only the known teardown
  leak; no `GL_INVALID`.
- Window size: the client (asked 640x700 logical at scale 1.667) now opens 1067x1167 on x11 as
  winit's 1066x1166 before the WM tiles it; glfw steps through the asked size too; sdl clamps an
  oversized ask itself. The parity scene (override 1.0) is unchanged.
- Earlier runs (git): all twelve Linux pairs on the parity scene (milestone 3), every world
  material and the UI at max 1 on vk, GL within its sRGB rounding; MSAA, egui, window modes and
  properties, `WOW_GPU_MS`. Windows (win32, d3d11, d3d12) never built; the d3d11 HLSL of every
  shader passes glslang's parser.

## Build notes

- gfx library: `~/Code2/General/gfx/gfx_dll/gfx_benilla` (its own repo at `~/Code2/General/gfx`, `00d613d`),
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
  resolve, `get_msaa_counts`; `copy_texture`; vk depth barriers; and (this run)
  `gfx_dll_set_depth_zero_to_one` (gl3/gl4/gles3 `glClipControl` / `glClipControlEXT` once the
  version or extension says it exists; vk, d3d11, d3d12 already [0, 1]; d3d ones not built), and
  (this run) `gfx_dll_device_supports_format` (GL / GLES by the S3TC, S3TC-sRGB, `EXT_texture_sRGB`
  and RGTC extensions, vk by `vkGetPhysicalDeviceFormatProperties`, d3d11 / d3d12 true; d3d not
  built).
  Optional ops sit outside `GFX_DEVICE_OP_DEF` (d3d9 and jkg leave them NULL).
- Shaders: sources `crates/benilla-gfx/shaders/src/*.{vs,fs}.gfxs`; `crates/benilla-gfx/shaders/
  compile.sh [names]` writes the four families (compiler: `cd ~/Code2/General/gfx/wc_compiler_rs
  && cargo build --release`). HLSL reserves `point` and `line`. The compiler prints samplers in
  fragment stages only (a fetching vertex stage declares its own per backend), has no depth
  output, and `include` works only in the header (programs sharing a stage carry copies:
  `device_clip` is in every 3D vertex program).
- Build: `CARGO_INCREMENTAL=0 cargo build -p benilla --features gfx`; incremental caches filled the
  disk before. Disk: ~45 GB free; delete `target/debug/deps` files older than the round
  (`find target/debug/deps -maxdepth 1 -type f -mmin +180 -delete`) before the gates.

## Gates (this run)

See the commit: `scripts/check.sh` (escalates to `gates.sh`: `.claude/` is outside the crate
map), `cargo clippy --workspace --all-targets --features benilla/gfx -- -D warnings`,
`cargo test -p benilla-gfx --features egui`. No `smoke.sh`: no `.probe-identity`.

## For the maintainer

- Deferred (2026-09-27): mouselook's `CursorGrabMode::Locked` stays a real gfx grab.
- `ui-unitframes` (2026-09-28): the chat dock ends where the last `UIParent_ManageFramePositions`
  left it, and benilla re-runs that only on a screen size change, so a WM's late tile (winit) and
  an early one (gfx) end 40 UI units apart; on a non-tiling WM wgpu would land where gfx does.
  Measured under "Verified this run". A benilla UI question (1.12 re-docks from the bars'
  OnShow / OnHide), not a gfx one.
- Window size is not a parity target: the WM may tile a gfx window.
- Upstream candidates in `gfx_benilla` that are gfx bugs, not benilla needs: the x11 raw event
  twice under a grab, the x11 release-as-repeat heuristic, glfw's no-op `set_mouse_position`,
  sdl's late X1/X2, win32's screen-coordinate `set_mouse_position`, the GL depth attachment and
  `D32_SFLOAT` fixes, the vk slot mapping / descriptor fill, gl3/gles3 array depth, GL clears
  under the write masks, the vk depth barriers; API additions: depth bias, `read_texture`,
  `copy_texture`, `set_depth_zero_to_one`.
- The `shaders_gles3_dark` family is compiled without the Windows-only gamma hack.
- The install: `$wow_classic_dir` = `/home/jonas/Downloads/wow_classic` (build 5875), through the
  gitignored `WoW` link or `WOW_DATA`; read-only.

## Open problems

- `check_window_pinned` (`benilla-app/src/video.rs`) truncates the logical size at a fractional
  scale factor (5/3 here), so a `WOW_WIN` capture refuses sizes that are not multiples of 3, and
  under `WOW_DPI` winit still creates the window at the display's scale; both builds, not gfx.
- GL MSAA sample pattern: GL draws the scene bottom-up (the present flips), so 4x is mirrored in
  y against the image (0.39% of the parity scene, 0.93% of the overview >1). `glClipControl
  (GL_UPPER_LEFT, …)` where clip control exists would draw top-down and retire the GL row flips
  (`top_down`, `misc.y` bit 1, the readback flips); the no-clip-control path keeps them.
- GL sRGB store rounding: the UI target on GL differs by 1-2 levels over ~2.6% of a frame.
- The depth probe reads no depth on gles3 (a copy into colour through a shader would).
- A vk teardown leaks one `VkDeviceMemory` (`VUID-vkDestroyDevice-device-05137`).
- A vk render pass loads `DONT_CARE` from `UNDEFINED`: targets kept across passes rely on the
  driver keeping them (RADV, lavapipe do).
- Not A/B'd for want of a scenario or a login: the UI model tiles, the minimap composite, the
  tile cell clip, char-select, depth-biased decals in the client, image-camera MSAA, `WOW_GPU_MS`
  against the wgpu meter, `AlwaysOnBottom` (awesome keeps no BELOW for either build).
- Costs unmeasured in a busy scene: static-gx's per-run early draws, the effect lane's whole
  stream upload, bevy_ui atlas re-uploads, egui's whole-atlas re-upload.
- Filler streams are per mesh; booth data textures and FFXGlow quarter targets are never dropped.
- The effect lane's clip has no `MAX_CLIP_ROWS` cap; the UI lane's decode scissor is not applied
  on GL; the UI lane assumes `RenderScale` 1.0; a sampled image variant misses sub-rect writes;
  a `Camera2d` on the window outside the lane without overlays and bevy_ui on a non-lane camera
  are skipped; a `Screenshot` of anything but the primary window is not taken.
- sdl and glfw grow a new window to 95% of the work area (`pick_window_size`) and clamp or place
  the client, not the frame; `WindowPosition::Centered` ignores its `MonitorSelection`.
- Log: each `Extract*Plugin` logs "Render app did not exist" once.

## Next

Milestone 7 continues; what is left needs what this machine lacks. With a `.probe-identity`
account: `scripts/smoke.sh` on the gfx build, then login to the character screen and into the
world on every Linux pair, the bowstring or fishing line, `gxMultisample 4`, `WOW_LIVE_FPS` with
`WOW_GPU_MS=1` on both builds, the warm pass on world entry, `WOW_PHASE=<uniqueId>` on a WMO. On
Windows: build `gfx_benilla` (MSVC, `GFX.md`) and benilla with `--features gfx`, fix what does not
compile in the win32 / d3d11 / d3d12 code written blind, and run the capture matrix there. Without
either: the worldview A/B (`tools/world_ab.sh`) on every pair, and the GL MSAA row order
(`glClipControl(GL_UPPER_LEFT, ...)`) from Open problems. Then mark the project done.
