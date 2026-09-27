# gfx DLL port - progress

Branch: `gfx-dll-backend`. Status: milestones 1 (Scaffold), 2 (Window and input) and 3 (GPU
resources) done; milestone 4 (World shaders) next.

## Milestones

- [x] 1. **Scaffold.** `crates/benilla-gfx` (empty without its `gfx` feature, all deps optional):
  `ffi.rs` (the `gfx_dll.h` surface), `shader_def.rs` / `shader_loader.rs` (the `.gfx` format;
  four embedded families, `WOW_GFX_SHADERS=<dir>` override), `backend.rs` (`WOW_GFX_WINDOW` /
  `WOW_GFX_DEVICE`, fail-at-boot naming the choices), `context.rs`, `events.rs`, `runner.rs` (poll
  -> `window::pump` -> `app.update()` -> `GfxRender` -> swap). `lib.rs::swap_in` disables
  `WinitPlugin` + `RenderPlugin`, adds `RenderMainWorldPlugin` and `GfxPlugin`; hooked in
  `benilla-world/src/boot.rs`. Features `benilla/gfx -> benilla-app/gfx -> benilla-world/gfx`, and
  `benilla-worldview/gfx`. `noop_device.rs`: a wgpu `noop` `RenderDevice`/`RenderQueue` for
  main-world startup code (settled with the maintainer, 2026-09-27). With `gfx`, `boot.rs` raises
  `bevy_egui::input` to error until milestone 6.
- [x] 2. **Window and input.** `window.rs` is bevy_winit 0.18.1's window half (keys, text,
  buttons, motion, wheel, cursor, focus, size, scale factor; `Window`, `CursorOptions`,
  `CursorIcon` sync). `WOW_GFX_INPUT_TRACE=<path>` logs every message and window call. Not applied
  yet, each logged once: `WindowMode` (milestone 6), `window_level`, `position`, `decorations`,
  `resizable`, focus requests; no runtime scale-factor change, no IME.
- [x] 3. **GPU resources.** `GfxRender` sets: `Prepare` (asset events -> stores, draw list reset),
  `Collect` (per-material collectors), `Draw`, `Present`.
  - `meshes.rs`: one gfx vertex buffer per attribute stream + index buffer, uploaded on first draw;
    `AssetEvent::Modified` re-makes it `Dynamic`, later changes rewrite in place while sizes hold;
    a program's missing attribute is a per-mesh filler of its default (colour white).
  - `images.rs`: 2D and 2D-array images, every mip, layer- or mip-major data, BGRA swizzled;
    BC1-5 as blocks; `*Srgb` as `gfx_benilla`'s sRGB formats; sampler from `ImageSampler` or the
    `ImagePlugin` default. `CompressedImageFormatSupport` = BC for every device but gles3 (read
    from `WOW_GFX_DEVICE` at build; the BLP loader decodes on gles3).
  - `material.rs`: `GfxMaterialPlugin::<M>::new(describe)` draws visible `MeshMaterial3d<M>`;
    `standard` = `StandardMaterial` unlit (base colour x texture x vertex colour, `uv_transform`,
    alpha modes as bevy_pbr); `extended_base` for `ExtendedMaterial<StandardMaterial, _>`.
    `benilla-world/src/gfx.rs` registers the 8 world materials through their base.
  - `draw.rs`: active `Camera3d`s on the primary window, by `order`; `ClearColorConfig`, viewport,
    `invert_culling`; per-view `VisibleEntities` (Mesh3d class: frustum + `RenderLayers`);
    opaque, mask, then transparent back to front (view z of the origin); reverse-Z `GreaterEqual`,
    depth cleared to 0 per view; GL gets `z' = 2z - w`. One uniform ring per frame (view block 80
    B, draw block 128 B, aligned by `gfx_dll_get_uniform_buffer_size`, 256 B tail for d3d11).
  - `pipelines.rs`: pipeline + state caches (bevy_pbr blend states, CCW front, cull from the
    material). `target.rs`: `R16G16B16A16Sfloat` + `D32Sfloat` scene target (the world camera is
    `Hdr`, `Tonemapping::None`) and `present.{vs,fs}` clamping and sRGB-encoding into the window
    (V flipped on vk/d3d, where render-target rows are top-down).
  - Carried to milestone 4: lit `StandardMaterial` (drawn unlit now), fog, emissive, normal maps,
    `depth_bias` (gfx rasterizer state has none), skinning/morph. To milestone 5: 2D/UI cameras
    (skipped, logged once). To 6: image-target cameras (skipped, logged once), MSAA (`Msaa` is
    ignored: drawn without).
- [ ] 4. **World shaders.** 17 WGSL files to `.gfxs`: `benilla-assets/src/shaders/{terrain,
  wdl, liquid, wow_model}.wgsl` (`TerrainExtension`, `WdlExt`, `LiquidExt`, `WowModelExt`),
  `benilla-world/src/shaders/{sky, sky_vertex, celestial, star, cloud, static_gx, wow_effect,
  ffx_glow}.wgsl` (`SkyExt`, `CelestialExt`, `StarExt`, `CloudExt`, static_gx pool/render,
  particles/render, ribbons, weather, ffx_glow post), plus StandardMaterial lit as benilla uses
  it, fog, lighting (`lighting/global_light.rs`, blob shadows), instance tint, rig palette
  (skinning), mat_anim_table, straddle, zfill, depth bias. Each gets its own `GfxProgram` and
  describe fn in place of `extended_base`.
- [ ] 5. **UI.** bevy_ui nodes (`Node`, `BackgroundColor`, `ImageNode` x69, borders, 9-slice),
  text (`ui_text` shapes with cosmic-text into its own atlas: `ui_text/engine/gpu.rs`,
  `ui_text/pack.rs`), `ui_pass.rs`, `ui_gamma.rs`, `opaque2d.rs`, the `AddUiMaterial`
  (`glue/add_material.rs`), UI shaders `benilla-app/src/shaders/ui_{add,gamma,node_gamma,quad,
  slice_gamma}.wgsl`, sprites (the FrameXML quad pass), gizmos (bowstring, fishing line), glue
  screens, portraits / model frames (`ui_models`, `portrait/glue_booth.rs`: render-to-texture).
- [ ] 6. **The rest.** Render-to-texture cameras (16 `RenderTarget::Image` sites), screenshots
  (`screenshot.rs`) and the capture harness (`capture/`, `depth_probe`, `phase_probe`; needs a
  gfx read-back call, which the API lacks), `WOW_GPU_MS` (`perf/gpu.rs`, `perf/journal.rs`),
  `pipe_warm`, MSAA (gfx takes MS textures with `levels` as the sample count, 1 on vk, see
  `wc_clean_new_rs/src/fx/render_target.rs`), fullscreen / window modes (`video.rs`), background
  window level, the `dev` egui panel (`bevy_egui` -> `gfx_imgui` or equivalent).
- [ ] 7. **Backend matrix and `GFX.md`.**

## Parity instrument

`crates/benilla-gfx/examples/parity.rs` (`--features gfx`): one scene through `wgpu` or `gfx` in
the same binary (camera as benilla's world camera, MSAA off): nearest and linear sRGB textures,
BC1 blocks, vertex colours, culling, depth, mask, blend, additive.
`OUT=<dir> .claude/skills/gfx-dll-port/tools/parity.sh x11:gl4 x11:vk ...` runs wgpu and each
pair, captures each window with ImageMagick `import` at 1 s, diffs with `parity_diff.py` (max,
mean, share of pixels off by >1/>4/>16 levels, bbox, x8 diff image). Both windows are resizable,
so the tiling WM gives both the same slot (gfx does not apply `resizable: false`).

## Verified backend pairs

Linux (Debian 13, X11 :0, awesome WM, Radeon/Mesa), `gfx_benilla` Debug, no account (no
`.probe-identity`), `WOW_UNATTENDED=1 WOW_NOSOUND=1`. The runs so far used no install (see "The
install" below: one exists and was missed until 2026-09-27).
- Milestone 3, parity scene vs wgpu, 928x1013, all twelve Linux pairs (x11, glfw, sdl) x (gl3, gl4,
  gles3, vk), 2026-09-27: vk max 1 level, 0 pixels >1; gl3/gl4/gles3 mean 0.048, 19 pixels >1 and
  10 >4 (of 940k), each on a triangle edge or a nearest-texel boundary; no gfx error in any log.
- benilla boots on x11/gl4 and x11/vk with the renderer: 57.6 frames/s, `WM_DELETE_WINDOW` ->
  exit 0; logs "a camera on an image or other target" and "a 2D or UI camera" not drawn yet.
- Milestone 2 (input, grab, scale factor): see git `3db008e3`; not re-run.
Windows (win32, d3d11, d3d12) not built.

## Build notes

- gfx library: `~/Code2/General/gfx/gfx_dll/gfx_benilla` (gfx repo `b046a3d`). Differs from
  `gfx_dll/gfx`: key events carry `physical` + `scancode`, `GFX_EVENT_RAW_MOTION`, per-window
  `scale_factor`, exported cursor / warp / icon / scale-factor calls, milestone-2 backend fixes,
  and (this run) `GFX_FORMAT_R8G8B8A8_SRGB`, `BC1_RGBA_SRGB_BLOCK`, `BC2_SRGB_BLOCK`,
  `BC3_SRGB_BLOCK` appended to `enum gfx_format` (all six devices; d3d11/d3d12 not compiled), GL
  depth attachment by format (`gl_depth_attachment`), `D32_SFLOAT` = `GL_DEPTH_COMPONENT32F`.
  Build: `cd ~/Code2/General/gfx/gfx_dll/gfx_benilla && mkdir -p build && cd build && cmake ..
  -DCMAKE_BUILD_TYPE=Debug && make -j$(nproc)` -> `bin/Debug_x64/libgfx.so`. `benilla-gfx/build.rs`
  takes `gfx_benilla` over `gfx` (`GFX_DIR`, `GFX_CONFIGURATION`) and copies it beside the binary
  (and gives this crate's examples an rpath to it).
- Shaders: sources `crates/benilla-gfx/shaders/src/*.{vs,fs}.gfxs` (`blit`, `present`,
  `standard`); `crates/benilla-gfx/shaders/compile.sh` writes the four families. Compiler: build
  it first, `cd ~/Code2/General/gfx/wc_compiler_rs && cargo build --release` (compile.sh prefers
  release; the old March debug build appended a `pow(c, 2.2)` gamma hack to every gles3 fragment
  output, which the current source only does on Windows for a `*gles3*dark*` source dir).
- Build: `cargo build -p benilla --features gfx`; the instrument: `cargo build -p benilla-gfx
  --features gfx --example parity` (1.5 GB debug binary).

## Gates (this run)

`cargo fmt --all -- --check` clean; `cargo clippy --workspace --all-targets -D warnings` green
with and without `--features benilla/gfx`; `cargo test -p benilla-gfx` with `--features gfx` (22
passed) and without (empty); `cargo test -p benilla-world` feature-off (547 passed, 6 ignored).
`scripts/check.sh` escalates to `gates.sh` (the branch's `Cargo.lock` differs from main) and was
not run: its workspace test build does not fit this disk (9 GB free at the end, after deleting
`target/debug/incremental` and stale >200 MB test binaries). The feature-off change outside
`benilla-gfx` is `WorldPlugins::build` binding the group before returning it (same plugins) and a
cfg'd `mod gfx`. Not run: the player build, the engine boot checks, `smoke.sh` (no `.probe-identity`), and no
test ran against the install (it was not known about yet).

## For the maintainer

- Deferred (maintainer, 2026-09-27): mouselook's `CursorGrabMode::Locked` stays a real gfx grab.
  Don't re-ask before it matters.
- Window size is not a parity target (maintainer): the WM may tile a gfx window.
- Upstream candidates in `gfx_benilla` that are gfx bugs, not benilla needs: the x11 raw event
  twice under a grab, the x11 release-as-repeat heuristic, glfw's no-op `set_mouse_position`,
  sdl's late X1/X2, win32's screen-coordinate `set_mouse_position`, and this run's GL depth
  attachment and `D32_SFLOAT` fixes.
- The `shaders_gles3_dark` family (gles3 on a native window off Linux) is compiled here without
  the Windows-only gamma hack; whether that window needs it is a Windows question.

## The install

This machine has a 1.12.1 install: `$wow_classic_dir` = `/home/jonas/Downloads/wow_classic`
(`WoW.exe` "Build 5875 (Sep 19 2006)"; `Data/` holds `base`, `dbc`, `model`, `terrain`,
`texture`, `patch`, `patch-2` and the rest of the vanilla MPQs). Point benilla at it with
the gitignored `WoW` link at the repo root (`WoW -> $wow_classic_dir`, made 2026-09-27; a dev
build finds `WoW/Data` by itself), or `WOW_DATA="$wow_classic_dir/Data"`; `WOW_DATA=` (set,
empty) runs without it. It is read-only to benilla. Not the other `$wow_*_dir` installs:
`$wow_dir` (`~/Downloads/wow`) is 3.3.5, `$wow_tbc_dir` TBC, `$wow_cata_dir` Cataclysm. With it,
world scenes (worldview, the glue screens) can be A/B'd against wgpu live; `BENILLA_REQUIRE_DATA=1`
makes the gates' data-reading tests count. A login still needs a `.probe-identity` account.

## Open problems
- Log: each `Extract*Plugin` logs "Render app did not exist" once at build; `bevy_gizmos_render`
  warns likewise (milestones 4-5).
- The capture harness is untested under gfx and gfx has no read-back (milestone 6).

## Next

Milestone 4, first program: `WowModelExt` (`benilla-assets/src/shaders/wow_model.wgsl`, the M2
and WMO material), as a `wow_model.{vs,fs}.gfxs` with its own `GfxProgram` (its vertex inputs and
uniforms read from `WowModelExt`'s fields), described in `benilla-world/src/gfx.rs` in place of
`extended_base`; extend `examples/parity.rs` with a `WowModelMaterial` quad on synthetic textures
and settle it by the same diff, then against the real thing: `benilla-worldview` with
`WOW_DATA="$wow_classic_dir/Data"` through wgpu and gfx at one fixed view. Then lit
`StandardMaterial` and fog, then terrain.
