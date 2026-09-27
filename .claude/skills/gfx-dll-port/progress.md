# gfx DLL port - progress

Branch: `gfx-dll-backend`. Status: milestones 1-3 done; milestone 4 (World shaders) in flight:
the model program (`WowModelMaterial`), the shared light buffer and the FFXGlow post pass are
ported and A/B'd live; the static-gx pool, terrain and the rest are next.

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
- [x] 3. **GPU resources.** `GfxRender` sets: `Pack` (main-world data into data textures),
  `Prepare` (asset and data-texture changes -> device stores, draw list reset), `Collect`
  (per-material collectors), `Draw`, `Present`.
  - `meshes.rs`: one gfx vertex buffer per attribute stream + index buffer, uploaded on first draw;
    `AssetEvent::Modified` re-makes it `Dynamic`, later changes rewrite in place while sizes hold;
    a program's missing attribute is a per-mesh filler of its default; `VertexInput::read_as`
    reads a stream as another same-size format (a `Uint32` as `R32_SINT`); the mask of inputs a
    mesh has goes to the draw block's tag (`tag.y`), the shader defs' stand-in.
  - `images.rs`: 2D and 2D-array images, every mip, BGRA swizzled, BC1-5 as blocks, `*Srgb` as
    `gfx_benilla`'s sRGB formats; `CompressedImageFormatSupport` = BC except on gles3.
  - `data.rs`: a storage buffer as an RGBA32F 2D-array data texture (256x16 rows a layer; every
    device updates a 2D array one whole layer at a time), keyed by the Bevy `BufferId` a material
    binds; main-world `GfxDataTextures`, uploaded by dirty layer.
  - `material.rs`: `GfxMaterialPlugin::<M>::new(describe)`; `GfxMaterialDesc` = program
    (`GfxProgram`: name, inputs, parameter rows, sampler count), up to 4 texture slots
    (`White`/`Image`/`Data`), up to 12 parameter rows, alpha, cull, `GfxDrawState` (blend,
    depth-write and `Always` overrides, colour write, sort bias). `standard` = `StandardMaterial`
    unlit; `extended_base` for an unported `ExtendedMaterial`. The collector takes `MeshTag` and
    the AABB centre (bevy_pbr's transparent sort point).
  - `draw.rs`: active `Camera3d`s on the primary window, by `order`; view block (240 B:
    `clip_from_world`, `view_from_world`, `clip_from_view`, eye, viewport, `misc` = mip bias, GL
    remap flag, target height); draw block = world matrix, tag `uvec4`, the program's rows; opaque,
    mask, then transparent sorted by the AABB centre's view z + depth bias; reverse-Z; GL gets
    `z' = 2z - w`. `pipelines.rs`: bevy_pbr blend states plus `Add` (ONE,ONE), `Modulate`,
    `Modulate2x`, colour mask, `Always` depth. `target.rs`: two `R16G16B16A16Sfloat` colours
    sharing a `D32Sfloat` depth (Bevy's ping-pong), linear-filtered; `present` clamps and
    sRGB-encodes the current one into the window.
- [ ] 4. **World shaders.**
  - [x] `WowModelExt` (`wow_model.wgsl` -> `wow_model.{vs,fs}.gfxs`, `benilla-world/src/gfx/
    model.rs`): M2/WMO/clutter lighting, SH lobe, prop probes, point lights, rig palette skinning,
    body tint, mat-anim UV/tint/affine, straddle clip, merged fade/slot, env map, fog policies,
    Mod/Mod2x/additive/zfill/no-depth states as `WowModelExt::specialize`. Shader defs are runtime
    tests (input mask, `flags.x` = `WOW_WATER_CLIP`). No `front_facing` (every model material is
    double-sided exactly when it culls nothing, so the lit normal is the interpolated one).
  - [x] The shared light buffer (`benilla-world/src/gfx/light.rs`): packed from the main-world
    tables the render-world uploads read (`WowLightData`, `PropProbeExtract`,
    `RigPaletteExtract`, `InstanceTints`, `MatAnimTable`, `WaterClips`, via `cfg(gfx)` accessors),
    at row = the wgpu byte offset / 16; `u32` regions as floats (rig table: base index; tint:
    `word & 0xFFFFFF`, -1 = identity). A test checks the shader's `#define` bases.
  - [x] FFXGlow (`post.rs`, `ffx_{downsample,gauss,combine}.fs.gfxs`, `benilla-world/src/gfx/
    ffx.rs`): Box4 to 1/4 (min 8), Gauss4 H/V, combine (glow, haze, FFXDeath, dither) with the
    frame's gamma decode, into the other scene colour. Not yet: the underwater GlowWave warp (plain
    combine, warned once), the UI camera's backdrop claim.
  - [ ] static_gx (`static_gx/render.rs`, `static_gx.wgsl`): the default path for static doodads
    and WMOs, a custom render-graph node; not drawn under gfx at all yet, so a default run misses
    most of the world's models (`WOW_STATIC_GX=0` puts them back on `WowModelMaterial`).
  - [ ] `TerrainExtension` (`terrain.wgsl`), `WdlExt`, `LiquidExt`, `SkyExt` + `sky_vertex`,
    `CelestialExt`, `StarExt`, `CloudExt`: still through their unlit base (terrain draws white).
  - [ ] Particles (`particles/render.rs`), ribbons, weather (`wow_effect.wgsl`): render-world
    pipelines, not drawn. Lit `StandardMaterial`, blob shadows.
  - [ ] Raster depth bias: gfx's rasterizer state has none; bevy_pbr packs `depth_bias as i32`
    (0 for every model batch but the zfill twin's -8). Needs a `gfx_benilla` addition.
- [ ] 5. **UI.** bevy_ui nodes (`Node`, `BackgroundColor`, `ImageNode` x69, borders, 9-slice),
  text (`ui_text` shapes with cosmic-text into its own atlas: `ui_text/engine/gpu.rs`,
  `ui_text/pack.rs`), `ui_pass.rs`, `ui_gamma.rs`, `opaque2d.rs`, the `AddUiMaterial`
  (`glue/add_material.rs`), UI shaders `benilla-app/src/shaders/ui_{add,gamma,node_gamma,quad,
  slice_gamma}.wgsl`, sprites (the FrameXML quad pass), gizmos (bowstring, fishing line), glue
  screens, portraits / model frames (`ui_models`, `portrait/glue_booth.rs`: render-to-texture),
  the FFX backdrop claim.
- [ ] 6. **The rest.** Render-to-texture cameras (16 `RenderTarget::Image` sites) and their
  studio light buffers (`LightBlob` buffers written through the noop queue: mirror them into data
  textures), screenshots (`screenshot.rs`) and the capture harness (`capture/`, `depth_probe`,
  `phase_probe`; needs a gfx read-back call, which the API lacks), `WOW_GPU_MS`, `pipe_warm`,
  MSAA, fullscreen / window modes (`video.rs`), background window level, the `dev` egui panel.
- [ ] 7. **Backend matrix and `GFX.md`.**

## Parity instrument

`crates/benilla-gfx/examples/parity.rs` (`--features gfx`): one scene through `wgpu` or `gfx` in
the same binary (camera as benilla's world camera, MSAA off): nearest and linear sRGB textures,
BC1 blocks, vertex colours, culling, depth, mask, blend, additive.
`OUT=<dir> .claude/skills/gfx-dll-port/tools/parity.sh x11:gl4 x11:vk ...` runs wgpu and each
pair, captures each window with ImageMagick `import` at 1 s, diffs with `parity_diff.py` (max,
mean, share of pixels off by >1/>4/>16 levels, bbox, x8 diff image). Both windows are resizable,
so the tiling WM gives both the same slot (gfx does not apply `resizable: false`).

The live world A/B: `.claude/skills/gfx-dll-port/tools/world_ab.sh <wgpu-bin> <gfx-bin> x11:gl4
...` runs `benilla-worldview` (default Northshire overview, `WOW_CLOCK=720`, `WOW_WIN=1280x720`,
`WOW_BG=0`) through both builds (copy each out of `target/debug`: `target/ab/` holds them with
`libgfx.so`), captures at `AT` s, and `masked_diff.py <wgpu> <gfx> --erode 24` diffs only where
gfx drew and 24 px past the glow's reach of anything it did not (unported white terrain).
`validate_gfx.py crates/benilla-gfx/shaders` runs glslangValidator over every family's GL/GLES
text and the d3d11 HLSL (glslang's HLSL front end, not fxc).

## Verified backend pairs

Linux (Debian 13, X11 :0, awesome WM, Radeon/Mesa), `gfx_benilla` Debug, no account (no
`.probe-identity`), `WOW_UNATTENDED=1 WOW_NOSOUND=1`, window tiled to 928x1013.
- Milestone 4, world A/B with the install, `WOW_STATIC_GX=0`, 2026-09-27, eroded mask 25.8% of the
  frame (models only): x11/vk max 1 level, 0 pixels >1; x11/gl4 mean 0.24, 0.125% >1 (speckles
  that did not recur on vk: foliage animation between runs); x11/gles3 0.575% >1, 0.05% >4 (BLPs
  CPU-decoded there). vk run: 59 frames/s, `WORLDVIEW_CHECK ok`, exit 0.
- Parity scene after this run's block changes: gl4 mean 0.048 (as before), vk max 1.
- Milestone 3: parity scene on all twelve Linux pairs (x11, glfw, sdl) x (gl3, gl4, gles3, vk).
- Milestone 2 (input, grab, scale factor): see git `3db008e3`; not re-run.
Windows (win32, d3d11, d3d12) not built; the d3d11 HLSL of every shader passes glslang's parser.

## Build notes

- gfx library: `~/Code2/General/gfx/gfx_dll/gfx_benilla` (gfx repo `b046a3d`). Differs from
  `gfx_dll/gfx`: key events carry `physical` + `scancode`, `GFX_EVENT_RAW_MOTION`, per-window
  `scale_factor`, exported cursor / warp / icon / scale-factor calls, milestone-2 backend fixes,
  and (milestone 3) `GFX_FORMAT_R8G8B8A8_SRGB`, `BC1_RGBA_SRGB_BLOCK`, `BC2_SRGB_BLOCK`,
  `BC3_SRGB_BLOCK` appended to `enum gfx_format` (all six devices; d3d11/d3d12 not compiled), GL
  depth attachment by format (`gl_depth_attachment`), `D32_SFLOAT` = `GL_DEPTH_COMPONENT32F`.
  Build: `cd ~/Code2/General/gfx/gfx_dll/gfx_benilla && mkdir -p build && cd build && cmake ..
  -DCMAKE_BUILD_TYPE=Debug && make -j$(nproc)` -> `bin/Debug_x64/libgfx.so`. `benilla-gfx/build.rs`
  takes `gfx_benilla` over `gfx` (`GFX_DIR`, `GFX_CONFIGURATION`) and copies it beside the binary
  (and gives this crate's examples an rpath to it).
- Shaders: sources `crates/benilla-gfx/shaders/src/*.{vs,fs}.gfxs` (`blit`, `present`,
  `standard`, `wow_model`, `ffx_downsample`, `ffx_gauss`, `ffx_combine`);
  `crates/benilla-gfx/shaders/compile.sh` writes the four families. The compiler prints samplers
  in fragment stages only: a vertex stage that fetches (`wow_model.vs`) declares its sampler in
  the body per backend, at the fragment stage's slot (every device binds samplers to all
  stages). Compiler: build it first, `cd ~/Code2/General/gfx/wc_compiler_rs && cargo build
  --release` (compile.sh prefers release; the old March debug build appended a `pow(c, 2.2)`
  gamma hack to every gles3 fragment output, which the current source only does on Windows for a
  `*gles3*dark*` source dir). No gfx library or compiler change this run.
- Build: `cargo build -p benilla --features gfx`; the instrument: `cargo build -p benilla-gfx
  --features gfx --example parity` (1.5 GB debug binary).

## Gates (this run)

`cargo fmt --all -- --check` clean; `cargo clippy --workspace --all-targets -D warnings` green
with and without `--features benilla/gfx`; `cargo test -p benilla-gfx --features gfx` 25 passed;
`benilla-world --features gfx` `gfx::` tests 3 passed (the shader's region bases, tint words,
row packing); `cargo test -p benilla-world` feature-off 547 passed, 6 ignored (as before).
`scripts/check.sh` / `gates.sh`, the player build and `smoke.sh` not run: the disk has ~6 GB free
(`target/` ~70 GB) and there is no `.probe-identity` for a login. The feature-off change outside
`benilla-gfx` is `cfg(feature = "gfx")` accessors only (global_light, prop_probes, rig_palette,
instance_tint, mat_anim_table, straddle, ffx_glow) and `gfx.rs` -> `gfx/mod.rs`.

## For the maintainer

- Deferred (maintainer, 2026-09-27): mouselook's `CursorGrabMode::Locked` stays a real gfx grab.
  Don't re-ask before it matters.
- Window size is not a parity target (maintainer): the WM may tile a gfx window.
- Upstream candidates in `gfx_benilla` that are gfx bugs, not benilla needs: the x11 raw event
  twice under a grab, the x11 release-as-repeat heuristic, glfw's no-op `set_mouse_position`,
  sdl's late X1/X2, win32's screen-coordinate `set_mouse_position`, and milestone 3's GL depth
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
- A default run (static-gx on) draws almost no doodads or WMOs under gfx: static_gx is next.
- Filler streams are per mesh: a `wow_model` mesh lacking the rig and merged attributes carries
  4 filler buffers of its vertex count. An instance-step shared filler would need per-mesh input
  layouts.
- The UI tile cell clip (`anim_slots.w`) flips the fragment row on GL; untested until model tiles
  draw (milestone 6).
- Log: each `Extract*Plugin` logs "Render app did not exist" once at build; `bevy_gizmos_render`
  warns likewise.
- The capture harness is untested under gfx and gfx has no read-back (milestone 6).
- Disk: this machine's disk is full but for ~12 GB; `target/debug/incremental` (16 GB) was
  deleted again this run.

## Next

Milestone 4, static_gx: port `static_gx.wgsl` and the pool draw (`static_gx/render.rs`,
`prepare_static_gx` / `StaticGxNode`: record table, runs, kill bits) to a gfx program fed from the
main-world `GxWorld` (publish_gx_world) and a data texture for the record table; A/B with the
default `WOW_STATIC_GX` on. Then `terrain.wgsl` (reads the same light buffer data texture), then
WDL, liquid, and the sky family.
