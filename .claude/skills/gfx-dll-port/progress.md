# gfx DLL port - progress

Branch: `gfx-dll-backend`. Status: milestone 1 (Scaffold) done; milestone 2 next.

## Milestones

- [x] 1. **Scaffold.** `crates/benilla-gfx` (empty without its `gfx` feature, all deps optional):
  `ffi.rs` (the `gfx_dll.h` surface; C-side values read as raw ints), `shader_def.rs` /
  `shader_loader.rs` (the `.gfx` format; four embedded families, `WOW_GFX_SHADERS=<dir>`
  override), `backend.rs` (`WOW_GFX_WINDOW` / `WOW_GFX_DEVICE`, fail-at-boot naming the
  choices), `context.rs` (window + device, non-send), `events.rs` (thread-local queue),
  `runner.rs` (poll -> `app.update()` -> `GfxRender` schedule -> swap; WM close ->
  `WindowCloseRequested`), `render.rs` (`GfxRender` + `GfxRenderSystems::{Begin, Draw}`;
  Begin clears to `ClearColor`). `lib.rs::swap_in` disables `WinitPlugin` + `RenderPlugin` and
  adds `RenderMainWorldPlugin` (RenderPlugin's main-world half: Shader asset, Camera/View/
  Mesh/Morph/Texture/... plugins) and `GfxPlugin`. Hooked in `benilla-world/src/boot.rs`
  (`tuned_default_plugins`); features `benilla/gfx -> benilla-app/gfx -> benilla-world/gfx`.
  The launcher's build.rs adds `-rpath $ORIGIN` on Linux with `gfx`. `noop_device.rs`: the
  runner inserts a `RenderDevice`/`RenderQueue` on wgpu's `noop` backend (no GPU, no surface)
  after `finish`, because main-world startup code creates wgpu handles (`open_world_assets`'
  shared light buffer, `ui_models::setup_tiles`) that the material extensions and
  `WorldAssets` hold; they are never drawn. Decided (maintainer, 2026-09-27): keep the noop
  stand-in rather than gating those ~15 files onto gfx types; the gfx renderer reads the
  CPU-side data (e.g. packs the light blob from `WowLighting` itself). With `gfx`, `boot.rs` raises
  `bevy_egui::input` to error (it warns every frame without a winit window) until milestone 6.
- [ ] 2. **Window and input.** gfx events -> Bevy `KeyboardInput` / `ButtonInput<KeyCode>`,
  `KeyboardInput.text`/`logical_key` (text entry: `textinput` reads these), `MouseButtonInput`,
  `MouseMotion` (grabbed mouselook: `player/input.rs`, `camera`), `MouseWheel`, `CursorMoved` +
  `Window::set_physical_cursor_position`, `CursorEntered/Left`, `WindowFocused`,
  `WindowResized` + `Window.resolution`, `WindowMoved`; ECS -> gfx: title, vsync from
  `present_mode`, `CursorOptions.visible` / `grab_mode`, `CursorIcon` (custom BLP cursors,
  `cursor.rs`), clipboard (`textinput/clipboard.rs`, today arboard/smithay), background runs
  (`bgwin`: unfocused, AlwaysOnBottom), `WindowMode` (`video.rs`). Needs `gfx_benilla`: the
  DLL does not export cursor show/hide, set-cursor, set-mouse-position, clipboard, window icon,
  fullscreen, focus/raise (they exist in `window.h`, not `gfx_dll.h`).
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
  fullscreen / window modes, the `dev` egui panel (`bevy_egui` -> `gfx_imgui` or equivalent).
- [ ] 7. **Backend matrix and `GFX.md`.**

## Verified backend pairs

Linux (Debian 13, X11, :0), `gfx_benilla` Debug, no install, no account (this checkout has no
`.probe-identity`): `WOW_UNATTENDED=1 WOW_NOSOUND=1 WOW_PROBE_EXIT_AT=6`, judged by the boot line
(pair + shader family loaded), `xwininfo` (window mapped), the runner's exit line (frames/s) and
the exit code. All exit 0 at 58.7-60.2 frames/s (vsync): x11/gl4, x11/gl3, x11/gles3, x11/vk,
glfw/gl4, glfw/vk, sdl/gl4, sdl/gl3, sdl/vk. A bad `WOW_GFX_WINDOW` exits 1 naming the choices.
Not yet run: glfw/gl3, glfw/gles3, sdl/gles3; Windows (win32, d3d11, d3d12) not built.

## Build notes

- gfx library: `~/Code2/General/gfx/gfx_dll/gfx_benilla` (copy of `gfx_dll/gfx`, gfx repo
  `e518b4f`). The x11/win32 teardown fix (display closed before libGLX / the Vulkan loader is
  unloaded; before it, x11/gl* segfaulted in `XCloseDisplay` on window delete) is upstream in
  `gfx_dll/gfx` (`3693aa8`) and `my_wow/c/wc_clean_new/lib/gfx` (my_wow `6aa7b9bb`), so
  `gfx_benilla` equals `gfx` again (`775685a`). Keep `wc_clean_new/lib/gfx` in step when a
  fix in `gfx_benilla` is not benilla-specific. Build: `cd ~/Code2/General/gfx/gfx_dll/gfx_benilla && mkdir -p build && cd build &&
  cmake .. -DCMAKE_BUILD_TYPE=Debug && make -j$(nproc)` -> `bin/Debug_x64/libgfx.so`
  (`GFX_ENABLE_TARGET_FRAMEBUFFER=ON`, the default; `ffi::GfxPipelineCreateInfo` carries
  `target_framebuffer` to match). `benilla-gfx/build.rs` takes `gfx_benilla` over `gfx`
  (`GFX_DIR`, `GFX_CONFIGURATION` override; Release is taken over Debug when built) and copies
  it to `target/<profile>/`.
- Shaders: sources `crates/benilla-gfx/shaders/src/*.{vs,fs}.gfxs`; `crates/benilla-gfx/shaders/
  compile.sh` writes `shaders/` (gl3,gl4,gles3), `shaders_vk/` (vk, needs `glslangValidator`),
  `shaders_d3/` (d3d11 HLSL; d3d12 reuses it), `shaders_gles3_dark/` (gles3). Compiler:
  `~/Code2/General/gfx/wc_compiler_rs/target/{release,debug}/wc_compiler_rs` or
  `GFX_SHADER_COMPILER`. Output is deterministic; the compiled files are committed and
  embedded. A test fails when a source lacks a compiled pair in any family.
- Build: `cargo build -p benilla --features gfx`; run: `target/debug/benilla` (rpath finds
  `libgfx.so` beside it).

## Gates (last run)

fmt and clippy (`--workspace --all-targets -D warnings`) green with and without
`--features benilla/gfx`. `scripts/check.sh` escalates to `gates.sh` (Cargo.lock changed); its
test gate failed twice on a full disk (ENOSPC, then `ld` bus error linking `benilla-app`
tests: the 14 integration binaries are 1.5-3 GB each). Run instead, feature off: `cargo test
-p benilla-world -p benilla-gfx` (547 passed), `cargo test -p benilla-app --lib` (3741
passed, incl. `the_client_builds_headless`); feature on: `cargo test -p benilla-gfx --features
gfx` (7 passed). Not run here: `benilla-app` integration tests, the other crates' tests (not
touched), the player build, engine boot checks, `smoke.sh` (no install, no `.probe-identity`).
Free disk before the next run (`target/` was 56 GB; `target/debug/incremental` was cleared).

## Open problems

- Window size: the WM here resizes the 640x360 window to 928x496 on map; `Window.resolution`
  does not follow until milestone 2 handles `RESIZE` (winit reports it on the wgpu path).
- Log: each `Extract*Plugin` in Bevy's render plugins logs "Render app did not exist" once at
  build (as the stock headless configuration does); `bevy_gizmos_render` warns likewise. They
  go when those plugins get gfx counterparts (milestones 3-5).
- `CompressedImageFormatSupport` is published as NONE, so BLPs decode to RGBA8 (the wgpu path
  uploads BC natively): settle in milestone 3.
- `run_mode`/capture: the capture harness is untested under gfx (milestone 6).

- sRGB: the wgpu path renders linear into an sRGB swapchain; the gfx window is
  `R8G8B8A8Unorm`. Milestone 3/4 must settle where the linear->sRGB encode happens (shader or
  framebuffer) per backend, with a numeric A/B.
- Scale factor: gfx exposes no DPI; the window opens at the `Window`'s physical size with Bevy's
  default scale factor 1. Settle in milestone 2 (HiDPI / `video::at_requested_dpi`).
- With `gfx`, `benilla-app/src/lib.rs` prints "executor: no render app — ExtractSchedule flip NOT
  applied": true (there is no render app), harmless.

## Next

Milestone 2: create `gfx_dll/gfx_benilla` (copy of `gfx_dll/gfx` without build outputs), export
cursor visibility / set-cursor / warp / clipboard / focus from `gfx_dll.h` for every window backend,
then translate the event queue (`events.rs`, drained in `runner.rs::pump`) into Bevy's input
messages and keep `Window` in sync both ways.
