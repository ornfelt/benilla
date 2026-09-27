# gfx DLL port - progress

Branch: `gfx-dll-backend`. Status: milestones 1 (Scaffold) and 2 (Window and input) done;
milestone 3 next.

## Milestones

- [x] 1. **Scaffold.** `crates/benilla-gfx` (empty without its `gfx` feature, all deps optional):
  `ffi.rs` (the `gfx_dll.h` surface; C-side values read as raw ints, `key`/`native_cursor`
  constants generated from the headers), `shader_def.rs` / `shader_loader.rs` (the `.gfx` format;
  four embedded families, `WOW_GFX_SHADERS=<dir>` override), `backend.rs` (`WOW_GFX_WINDOW` /
  `WOW_GFX_DEVICE`, fail-at-boot naming the choices), `context.rs` (window + device + cursor
  state, non-send), `events.rs` (thread-local queue), `runner.rs` (poll -> `window::pump` ->
  `app.update()` -> `GfxRender` -> swap), `render.rs` (`GfxRender` + `GfxRenderSystems::{Begin,
  Draw}`; Begin clears to `ClearColor`). `lib.rs::swap_in` disables `WinitPlugin` + `RenderPlugin`
  and adds `RenderMainWorldPlugin` and `GfxPlugin`; hooked in `benilla-world/src/boot.rs`.
  Features `benilla/gfx -> benilla-app/gfx -> benilla-world/gfx`, and `benilla-worldview/gfx`
  (the engine viewer; its fly camera grabs, which tests the grab without game data); both
  launchers' build.rs add `-rpath $ORIGIN` on Linux with `gfx`. `noop_device.rs`: a wgpu `noop`
  `RenderDevice`/`RenderQueue` for main-world startup code that makes wgpu handles (settled with
  the maintainer, 2026-09-27). With `gfx`, `boot.rs` raises `bevy_egui::input` to error until
  milestone 6.
- [x] 2. **Window and input.** `window.rs` is bevy_winit 0.18.1's window half. `pump` turns each
  poll's gfx events into `KeyboardInput` (`KeyCode` from gfx's `physical` key, `text` from the
  `CHARACTER` right after the press, `logical_key` as winit's; releases reuse the press's key),
  `MouseButtonInput` (4th Back, 5th Forward, then `Other(n + 5)` = winit's X button number),
  `MouseMotion` from `GFX_EVENT_RAW_MOTION`, `MouseWheel` (lines), `CursorMoved` + the `Window`'s
  physical cursor position (not while grabbed), `CursorEntered/Left`, `WindowFocused` +
  `Window.focused`, `WindowResized` + resolution, `WindowMoved`, `WindowCreated`, each also as a
  `WindowEvent` (bevy_picking reads those); repeats of an already reported size or focus are
  dropped. `Last`: `sync_windows` (title, resize, cursor warp, swap interval from `present_mode`,
  visible), `sync_cursor_options` (`Locked` -> gfx grab; `Confined` refused and reverted, like a
  grab winit refuses; hidden = a transparent cursor), `sync_cursor_icon` (`CursorIcon::Custom`
  image -> a gfx cursor from its RGBA8 pixels, cached; `System` -> gfx native shapes; removal ->
  default), `check_keyboard_focus_lost`. The scale factor is the window's
  (`gfx_dll_window_get_scale_factor`), applied at open like `create_windows`. `input.rs`: the key,
  logical key and button tables. `WOW_GFX_INPUT_TRACE=<path>` writes one line per message sent
  and per call made to the window. Not applied yet, each logged once when the app changes it:
  `WindowMode` (fullscreen, milestone 6), `window_level` (a background run opens as an ordinary
  window), `position`, `decorations`, `resizable`, focus requests; no runtime scale
  factor change (moving to another monitor), no IME. The clipboard stays on arboard, which the
  wgpu path uses too (winit never owned it).
- [ ] 3. **GPU resources.** `Assets<Mesh>` / `Assets<Image>` (BC1-3 via gfx BC formats;
  `benilla-assets/src/gpu_blp.rs`) -> gfx buffers/textures on `AssetEvent`; samplers
  (`ImageSampler`), camera/view extraction (`Camera`, `Camera3d`/`Camera2d`, `Projection`,
  `RenderLayers`, visibility), draw ordering, opaque / alpha-mask / blend passes, depth, MSAA.
- [ ] 4. **World shaders.** 17 WGSL files to `.gfxs`: `benilla-assets/src/shaders/{terrain,
  wdl, liquid, wow_model}.wgsl` (the `ExtendedMaterial<StandardMaterial, _>` extensions
  `TerrainExtension`, `WdlExt`, `LiquidExt`, `WowModelExt`), `benilla-world/src/shaders/{sky,
  sky_vertex, celestial, star, cloud, static_gx, wow_effect, ffx_glow}.wgsl` (`SkyExt`,
  `CelestialExt`, `StarExt`, `CloudExt`, static_gx pool/render, particles/render, ribbons,
  weather, ffx_glow post), plus StandardMaterial PBR/unlit as benilla uses it, fog, lighting
  (`lighting/global_light.rs`, blob shadows), instance tint, rig palette (skinning),
  mat_anim_table, straddle, zfill.
- [ ] 5. **UI.** bevy_ui nodes (`Node`, `BackgroundColor`, `ImageNode` x69, borders, 9-slice),
  text (`ui_text` shapes with cosmic-text into its own atlas: `ui_text/engine/gpu.rs`,
  `ui_text/pack.rs`), `ui_pass.rs`, `ui_gamma.rs`, `opaque2d.rs`, the `AddUiMaterial`
  (`glue/add_material.rs`), UI shaders `benilla-app/src/shaders/ui_{add,gamma,node_gamma,quad,
  slice_gamma}.wgsl`, sprites (the FrameXML quad pass), gizmos (bowstring, fishing line), glue
  screens, portraits / model frames (`ui_models`, `portrait/glue_booth.rs`: render-to-texture).
- [ ] 6. **The rest.** Render-to-texture cameras (16 `RenderTarget::Image` sites), screenshots
  (`screenshot.rs`) and the capture harness (`capture/`, `depth_probe`, `phase_probe`),
  `WOW_GPU_MS` (`perf/gpu.rs`, `perf/journal.rs` - wgpu query sets), `pipe_warm`, MSAA,
  fullscreen / window modes (`video.rs`; needs gfx calls for fullscreen, window level and
  `Monitor` entities, which `video.rs` lists resolutions from), background-run window level, the
  `dev` egui panel (`bevy_egui` -> `gfx_imgui` or equivalent).
- [ ] 7. **Backend matrix and `GFX.md`.**

## For the maintainer

- Mouselook's `CursorGrabMode::Locked` is honoured as a real gfx grab (pointer held and hidden,
  raw motion, warped back on release). winit rejects `Locked` on X11 and Windows, so there the
  wgpu path leaves the hidden pointer roaming; benilla restores its stash either way, and holding
  the pointer is what 1.12 does. Confirm or ask for winit's refusal to be copied.
- Outside a grab, glfw and sdl report motion as pointer steps over the window (neither has raw
  device events then); x11 and win32 report raw device motion with focus, as winit does.
- Upstream candidates in `gfx_benilla` that are gfx bugs rather than benilla needs: the x11 raw
  event delivered twice under a grab (the old `RAW_MOTION_SENSITIVITY 0.5` hid it), the x11
  release-as-repeat heuristic ignoring the keycode, glfw's no-op `set_mouse_position`, sdl's X1/X2
  one button late, win32's screen-coordinate `set_mouse_position`.

## Verified backend pairs

Linux (Debian 13, X11 :0, awesome WM), `gfx_benilla` Debug, no install, no account (this
checkout has no `.probe-identity`), `WOW_UNATTENDED=1 WOW_NOSOUND=1`, judged from the
`WOW_GFX_INPUT_TRACE` lines, the boot lines and exit codes. Input is sent to benilla's own window
only (`XSendEvent`: keys, motion, buttons, wheel, focus, `WM_DELETE_WINDOW`), plus a 12-20 px XTEST
pointer nudge that is undone: `tools/live_input.sh` and `tools/live_grab.sh` in this skill's folder
(`OUT=<dir> .claude/skills/gfx-dll-port/tools/live_input.sh x11 gl4` from the repo root).
- Input (`benilla`), x11/gl4, glfw/vk, sdl/gl3 (and x11/gl3, sdl/gles3 through the saved script):
  `a`, Shift+`1`, Space, Enter, a 400 ms `W` hold
  -> `KeyA`/"a", `Digit1`/"!", `Space`/" ", `Enter`/no text, one `KeyW` press and release; motion
  (100,50) physical -> `CursorMoved` (60,30) logical; left, wheel +1/-1, button 8 -> `Back`; the
  nudge -> `MouseMotion` (12,5) and (-12,-5) on x11 (raw); focus out -> `KeyboardFocusLost`;
  `WM_DELETE_WINDOW` -> exit 0 at 58.6-58.8 frames/s.
- Grab (`benilla-worldview`, right-drag), x11/gl4, glfw/gl4, sdl/vk: `gfx grab true` + hidden on
  the press, `MouseMotion` (20,0) and (-20,0) under it (exactly the nudge, once each), ungrab +
  shown on release, the pointer back at the drag start.
- Scale factor on this display (1920x1080 eDP, 309x173 mm, no Xft.dpi): winit (feature-off
  worldview, `Guessed window scale factor`) 1.6666666666666667; gfx on x11, glfw and sdl
  1.6666666; benilla's `video:` line agrees. Warp and title change: probed on all three.
Not yet run this milestone: glfw/gl3, glfw/gles3, x11/gles3, x11/vk; Windows (win32, d3d11,
d3d12) not built.

## Build notes

- gfx library: `~/Code2/General/gfx/gfx_dll/gfx_benilla` (gfx repo `30395c8`), which now differs
  from `gfx_dll/gfx`: key events carry `physical` + `scancode`, `GFX_EVENT_RAW_MOTION`
  (`gfx_motion_event`, event layout unchanged: 32 bytes, union at 12), a per-window
  `scale_factor` (x11 helper `gfx_x11_scale_factor` = winit's algorithm, also used by glfw and sdl
  on x11), exported cursor / warp / icon / scale-factor calls, and the backend fixes above. Build:
  `cd ~/Code2/General/gfx/gfx_dll/gfx_benilla && mkdir -p build && cd build && cmake ..
  -DCMAKE_BUILD_TYPE=Debug && make -j$(nproc)` -> `bin/Debug_x64/libgfx.so`
  (`GFX_ENABLE_TARGET_FRAMEBUFFER=ON`, the default). `benilla-gfx/build.rs` takes `gfx_benilla`
  over `gfx` (`GFX_DIR`, `GFX_CONFIGURATION` override) and copies it beside the binary when it
  changed; a library rebuilt after the last cargo build is copied by the next one.
- Shaders: sources `crates/benilla-gfx/shaders/src/*.{vs,fs}.gfxs`; `crates/benilla-gfx/shaders/
  compile.sh` writes `shaders/` (gl3,gl4,gles3), `shaders_vk/` (needs `glslangValidator`),
  `shaders_d3/` (d3d11, d3d12), `shaders_gles3_dark/` (gles3). Compiler:
  `~/Code2/General/gfx/wc_compiler_rs/target/{release,debug}/wc_compiler_rs` or
  `GFX_SHADER_COMPILER`. Output is deterministic; the compiled files are committed and embedded.
- Build: `cargo build -p benilla --features gfx` (and `-p benilla-worldview --features
  benilla-worldview/gfx`); run `target/debug/benilla`.

## Gates (last run)

`cargo fmt --all -- --check` clean; `cargo clippy --workspace --all-targets -D warnings` green with
and without `--features benilla/gfx`; `cargo test -p benilla-gfx` with `--features gfx` (15
passed) and without (empty crate); `cargo build -p benilla-worldview` feature-off (its binary
links no gfx). `scripts/check.sh` escalates to `gates.sh` (the branch's `Cargo.lock` differs from
main), whose workspace test run does not fit this disk (the `benilla-app` integration binaries are
1.5-3 GB each); the run ended with 8.9 GB free, so clear stale `target/debug/deps` test binaries,
`incremental` and `examples` first (they were 38 GB at this run's start); this run changed no feature-off code (`benilla-gfx` is empty without
its feature, `benilla-worldview` gained only the feature and a cfg'd build-script line). Not run:
the player build, the engine boot checks, `smoke.sh` (no install, no `.probe-identity`).

## Open problems

- Log: each `Extract*Plugin` logs "Render app did not exist" once at build; `bevy_gizmos_render`
  warns likewise. They go when those plugins get gfx counterparts (milestones 3-5).
- `CompressedImageFormatSupport` is published as NONE, so BLPs decode to RGBA8 (the wgpu path
  uploads BC natively): settle in milestone 3.
- sRGB: the wgpu path renders linear into an sRGB swapchain; the gfx window is `R8G8B8A8Unorm`.
  Milestone 3/4 must settle where the linear->sRGB encode happens, with a numeric A/B.
- The capture harness is untested under gfx (milestone 6).
- Window size is not a parity target (maintainer): glfw and sdl grow a small window toward the
  work area, and the window manager may retile it.

## Next

Milestone 3: upload `Assets<Mesh>` and `Assets<Image>` to gfx buffers and textures on
`AssetEvent` (a `GfxGpuAssets` non-send store keyed by asset id; BC1-3 straight to the gfx BC
formats and publish `CompressedImageFormatSupport` accordingly), extract cameras/views from the
main world (`Camera`, `Projection`, `GlobalTransform`, `RenderLayers`, `ViewVisibility`), and draw
one unlit pass with depth into the window, proven against a wgpu capture of the same view.
