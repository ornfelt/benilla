# gfx DLL port - progress

Branch: `gfx-dll-backend`. Status: milestones 1-3 done; milestone 4 (World shaders) nearly done:
every world material has its WGSL ported (models, static-gx, terrain, WDL, liquid, the sky family),
the effect lane (particles, ribbons, decals, foam, rain) draws through gfx, and gfx_benilla has a
raster depth bias. Water, dawn, dusk and night views match wgpu to one level on vk. Left in 4: the
underwater GlowWave warp, the UI camera's backdrop claim, a lit-StandardMaterial inventory.

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
  `resizable`, focus requests; no runtime scale-factor change, no IME. `cover_input_wall` skips
  `benilla-gfx/src/{window,input}.rs`: they send the channels, as bevy_winit does.
- [x] 3. **GPU resources.** `GfxRender` sets: `Pack` (main-world data into data textures),
  `Prepare` (asset and data-texture changes -> device stores, draw list reset), `Collect`
  (per-material collectors, early draws), `Draw`, `Present`.
  - `meshes.rs`: one gfx vertex buffer per attribute stream + index buffer; `VertexInput::read_as`;
    the mask of inputs a mesh has goes to the draw block's tag (`tag.y`).
  - `images.rs`: 2D and 2D-array images, every mip, a 2D array one layer per call (the device
    convention, `data.rs` alike), BGRA swizzled, BC1-5 as blocks, `*Srgb` as sRGB formats.
    A gfx texture carries its sampler: `GfxSampler` variants (`get_sampled`) upload an image again
    under a draw's own sampler unless it equals the image's; `get_sampled_like` samples with
    another image's sampler (a Bevy `#[sampler]` binding shared by a group's textures).
  - `data.rs`: a storage buffer as an RGBA32F 2D-array data texture (256x16 rows a layer).
  - `material.rs`: `GfxMaterialPlugin::<M>::new(describe)`; `GfxMaterialDesc` = program, up to 4
    texture slots (`White`/`Image`/`ImageSampled`/`ImageSampledLike`/`Data`), up to 12 parameter
    rows, alpha, cull, `GfxDrawState`.
  - `draw.rs`: active `Camera3d`s on the primary window, by `order`; per view: clear, the early
    draws (`DrawList::push_early(camera, EarlyDraw)`: an index range of a mesh with a description,
    before the opaque phase in push order, draws sharing (desc, matrix) share one ring block),
    opaque, mask, then transparent sorted by the AABB centre's view z + depth bias; reverse-Z; GL
    gets `z' = 2z - w`. `resolve` builds a draw's device state for both kinds.
- [ ] 4. **World shaders.**
  - [x] `WowModelExt` (`wow_model.{vs,fs}.gfxs`, `benilla-world/src/gfx/model.rs`).
  - [x] The shared light buffer (`benilla-world/src/gfx/light.rs`); a test checks every ported
    shader's `#define` row bases (`wow_model`, `static_gx`, `terrain`).
  - [x] FFXGlow (`post.rs`, `ffx_*.fs.gfxs`, `benilla-world/src/gfx/ffx.rs`). Not yet: the
    underwater GlowWave warp, the UI camera's backdrop claim.
  - [x] static_gx (`static_gx.{vs,fs}.gfxs`, `benilla-world/src/static_gx/gfx.rs`): `pack`
    (`GfxRenderSystems::Pack`) assembles each visible region (per-item description by texture,
    sampler, cutout, two-sided; wrap bits read off the mesh's word at the item's first vertex) and
    keeps its record table as a data texture (kill and fog syncs as `prepare_static_gx`); `collect`
    pushes the admitted runs as early draws for the `StaticGxView` camera, WMOs then the doodad
    phase. The wgpu texture-array pool is replaced by per-run images through the sampler the pool
    pair would pick (repeat if either axis wraps, else clamp; mixed clamps in the shader).
  - [x] `TerrainExtension` (`terrain.{vs,fs}.gfxs`, `gfx/terrain.rs`): the three arrays through the
    layer array's sampler (`ImageSampledLike`).
  - [x] `WdlExt` (`wdl.{vs,fs}.gfxs`, `gfx/wdl.rs`): the WGSL's `frag_depth` clamp is per vertex
    (the compiler has no depth output); the band is one flat colour, so only which of its own
    fragments wins can differ. Measured: no visible difference at the overview.
  - [x] `LiquidExt` (`liquid.{vs,fs}.gfxs`, `gfx/liquid.rs`): all four arms; the view block's
    `misc.w` is now bevy's `globals.time` (`Time::elapsed_secs_wrapped`) for the frame flip.
  - [x] The sky family (`gfx/sky.rs`): `sky`, `celestial`, `star`, `cloud`, one program each, every
    vertex stage repeating `sky_vertex.wgsl`'s far pin (clip z 0; `-w` under the GL remap, `misc.y`).
    The dome is opaque without depth writes, as `SkyExt::specialize`.
  - [x] The effect lane (`effect.{vs,fs}.gfxs`, `gfx/effect.rs`): `EffectQuads` rebased per camera
    as `prepare_effects` does, written into one mesh padded to a power of two and rewritten in place
    (`get_mut_untracked` + `GpuMeshes::modified`), each record a `DrawList::push_sorted` draw (new:
    a system's draw in the transparent phase, sorted with the entities by anchor view z + rung).
    `WOW_PARTICLE_FLAT` / `WOW_PARTICLE_NODEPTH` ported; `EFFECT_DRAW_STATS` written (no merging).
    Not yet: a booth's own light buffer and a UI pane's clip rect (image cameras, milestone 6).
  - [x] Raster depth bias: `GfxDrawState::{raster_bias, raster_slope}` into `PipelineKey`;
    `standard_rows` packs `depth_bias as i32` as bevy_pbr does, liquid, the sky family and the
    model far-side / skybox keys zero it as their `specialize`s do, the effect lane takes each
    record's. gfx_benilla `gfx_dll_create_rasterizer_state_biased`.
  - [ ] The underwater GlowWave warp, the UI camera's backdrop claim (from the FFXGlow item).
  - [ ] Lit `StandardMaterial`: `standard.fs.gfxs` is unlit only. Inventory the lit sites
    (`model_render.rs` x3, `terrain_stream.rs` x2, `benilla-app/src/entities.rs`) before porting.
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

## GPU safety (read before any live vk or GL run)

On 2026-09-27 a terrain draw hung this machine's GPU (amdgpu `ring gfx timeout`, MODE2 reset) and
the reset killed the maintainer's X session. The cause was a gfx vk bug (below, fixed); the terrain
vertex stage read an unwritten descriptor and looped on a garbage light count. Since then:
- Every new or changed program runs first off the GPU: vk on lavapipe with the validation layer,
  `VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json VK_INSTANCE_LAYERS=VK_LAYER_KHRONOS_validation
  timeout -s KILL 90 <bin>` (a runaway shader spins a CPU thread; draw-time validation must be
  clean), and GL on llvmpipe, `LIBGL_ALWAYS_SOFTWARE=1`, with no `GL_INVALID` in the log. Only
  then the real GPU. There is no Xvfb here; both software runs open a window on :0.
- Shader loops read from a data texture are bounded (the point-light loops take
  `min(count, 256)`, the table's size): a wrong binding must not spin the GPU into a reset.
- After a real run, `sudo -n dmesg | grep -cE "ring gfx timeout|GPU reset"` must stay 0.

## Parity instrument

`crates/benilla-gfx/examples/parity.rs` (`--features gfx`): one scene through `wgpu` or `gfx` in
the same binary. `OUT=<dir> .claude/skills/gfx-dll-port/tools/parity.sh x11:gl4 x11:vk ...`.

The live world A/B: `.claude/skills/gfx-dll-port/tools/world_ab.sh <wgpu-bin> <gfx-bin> x11:vk
...` runs `benilla-worldview` (Northshire overview or `WOW_WORLDVIEW_AT`, `WOW_CLOCK` default 720,
`WOW_WIN=1280x720`, `WOW_BG=0`) through both builds, captures at `AT` s (30) and diffs.
`WOW_CAPTURE=1` freezes the liquid clock and the clouds (worldview has no capture harness, only
the freeze). Views used: the lake, `WOW_WORLDVIEW_AT=-8886,-350,90` (the stream centred, sky at the
corners; the camera sits 60 yd back along -X at a -0.5 rad pitch, so the frame top is ~6 deg above
the horizon); the high view `-8886,-350,400` (a sky band over fog). Loop specs under `bash -c`:
zsh does not word-split `$spec`, which silently ran five views as the default overview once.
Particles are not deterministic: `fx_series.sh` counts flat-magenta coverage over five shots;
a backend matches when its counts fall in wgpu's own spread (a second wgpu run as the "gfx" binary
gives the noise floor in `world_ab.sh` too). Keep the binaries in `target/ab/` (the gfx
one beside a copy of the current `libgfx.so`: it loads the library from its own directory, so a
stale copy there runs the old library). `target/ab/worldview-wgpu` is feature-off and still valid
(this run's feature-off changes are visibility only: `CelestialExt`'s two fields `pub(crate)`). `validate_gfx.py crates/benilla-gfx/shaders`
runs glslangValidator over every family's GL/GLES text and the d3d11 HLSL.

## Verified backend pairs

Linux (Debian 13, X11 :0, awesome WM, Radeon 680M/Mesa), `gfx_benilla` Debug, no account (no
`.probe-identity`), `WOW_UNATTENDED=1 WOW_NOSOUND=1`, window tiled by the WM.
- Milestone 4, this run (liquid, sky family, effect lane, depth bias), `WOW_CAPTURE=1`:
  lake noon/dawn/dusk/night and high dusk/night on x11/vk: max 1, 0.000% >1 each (final build:
  lake max 4, 0.000% >1). GL (x11/gl4, x11/gl3, glfw/gles3): high dusk 0.005% >1, 0% >16; lake
  night 0.065% >1, 0.026% >16: the foliage alpha-test speckle plus single pixels along the
  waterline and some edges, absent on vk (GL depth precision, see open problems). Effect lane
  coverage (Northshire, 5 shots): wgpu 1319..1738 px (mean 1514), gfx vk 1305..1818 (1499), gfx gl4
  1301..1738 (1467); the plain overview A/B stays at 0.020% >1, 0% >16 on vk. Parity scene with
  two coplanar depth-biased decals: vk max 1, 0% >1; gl3/gl4/gles3 0.001% (nine texel-edge pixels
  on the checker, BC and gradient quads, none at the decals); the +bias decal wins and the -bias
  one loses everywhere on every pair. Lavapipe (validation: only the known teardown leak) and
  llvmpipe gl3/gl4/gles3 (no `GL_INVALID`) for every new program first. No GPU reset.
- Milestone 4, the previous run, world A/B with the install and the default `WOW_STATIC_GX` (models,
  static-gx, terrain, WDL ported), mask = the whole frame: x11/vk mean 0.154, 0.023% >1, 0.001%
  >16; x11/gl3 mean 0.211, 0.168% >1, 0.072% >16; glfw/gles3 mean 0.429, 0.280% >1, 0.071% >16;
  x11/gl4 (before the gl3/gles3 array fix, which gl4 does not need) mean 0.227, 0.085% >16. The GL
  residue is isolated single-pixel alpha-test flips in the near cutout foliage, identical on gl4
  and gles3 and absent on vk (the texel alpha landing a ULP apart at 224/255 on Mesa GL vs RADV).
  Every run ~59 frames/s, `WORLDVIEW_CHECK ok`, exit 0; no GPU reset. Lavapipe (validation clean
  but for a device-memory leak at teardown), llvmpipe gl3/gl4/gles3: exit 0, no GL errors.
- Milestone 3: parity scene on all twelve Linux pairs (x11, glfw, sdl) x (gl3, gl4, gles3, vk).
- Milestone 2 (input, grab, scale factor): see git `3db008e3`; not re-run.
Windows (win32, d3d11, d3d12) not built; the d3d11 HLSL of every shader passes glslang's parser.

## Build notes

- gfx library: `~/Code2/General/gfx/gfx_dll/gfx_benilla` (gfx repo `cd838f2`). Differs from
  `gfx_dll/gfx`: key events carry `physical` + `scancode`, `GFX_EVENT_RAW_MOTION`, per-window
  `scale_factor`, exported cursor / warp / icon / scale-factor calls, milestone-2 backend fixes,
  (milestone 3) the sRGB formats, GL depth attachment by format, `D32_SFLOAT` as float depth, and
  (this run) vk: each sampler slot mapped to (binding, element) against its shader state's layout
  (`vk_map_sampler_slots`; the old write, slot i -> binding 1 element i-1, rolled slot 3 into
  binding 2's array) and every unwritten element of a binding filled from a descriptor of the same
  binding; gl3/gles3: a 2D array keeps its layer count at every mip level (was halved as for 3D);
  and (this run) the rasterizer depth bias, `gfx_dll_create_rasterizer_state_biased` (GL polygon
  offset, vk pipeline bias, d3d11/d3d12 `DepthBias`; the d3d ones not built here, GL drops the
  clamp).
  Build: `cd ~/Code2/General/gfx/gfx_dll/gfx_benilla && mkdir -p build && cd build && cmake ..
  -DCMAKE_BUILD_TYPE=Debug && make -j$(nproc)` -> `bin/Debug_x64/libgfx.so`. `benilla-gfx/build.rs`
  takes `gfx_benilla` over `gfx` (`GFX_DIR`, `GFX_CONFIGURATION`) and copies it beside the binary.
- gfx's vk layout keeps its binding-2-is-14 hack (the original C apps' array): a benilla program's
  sampler slots 0..3 are single samplers; slot 2 works through the fill.
- Shaders: sources `crates/benilla-gfx/shaders/src/*.{vs,fs}.gfxs` (`blit`, `present`,
  `standard`, `wow_model`, `static_gx`, `terrain`, `wdl`, `liquid`, `sky`, `celestial`, `star`,
  `cloud`, `effect`, `ffx_downsample`, `ffx_gauss`, `ffx_combine`); `crates/benilla-gfx/shaders/compile.sh [names]` writes the four families. The
  compiler prints samplers in fragment stages only: a vertex stage that fetches declares its
  sampler in the body per backend at the fragment stage's slot (vk `set = 1, binding = slot`).
  The compiler has no depth output (`wdl` clamps per vertex). Its `include` works only in the
  header, and GL needs a uniform block identical in both stages, so programs sharing a WGSL vertex
  stage (the sky family) each carry their own copy. Compiler: `cd
  ~/Code2/General/gfx/wc_compiler_rs && cargo build --release`.
- Build: `CARGO_INCREMENTAL=0 cargo build -p benilla --features gfx` (or `-p benilla-worldview`).
  Incremental caches filled the disk twice this run: build without them.

## Gates (this run)

`CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=8 scripts/check.sh` (escalates to `gates.sh`): ALL GATES
GREEN on the committed tree at the start of the run (1669 s) and again on this run's final tree
(954 s): fmt, clippy, workspace tests (30 addon-corpus tests skip: no corpus here), no-install
tests, doc links, pass-span lint, player build and tests, both enforcers. `cargo clippy
--workspace --all-targets --features benilla/gfx -- -D warnings` green; `cargo test -p benilla-gfx
--features gfx` 26 passed; `benilla-world --features gfx --lib gfx` 5 passed. No `smoke.sh`: no
`.probe-identity`. Disk: the gate chain needs ~25 GB; delete `target/debug/deps` executables
(`find target/debug/deps -maxdepth 1 -type f -executable ! -name '*.so' -delete`, plus
`target/debug/examples`) before it: they are link outputs, ~35 GB here.

## For the maintainer

- Deferred (maintainer, 2026-09-27): mouselook's `CursorGrabMode::Locked` stays a real gfx grab.
- Window size is not a parity target (maintainer): the WM may tile a gfx window.
- Upstream candidates in `gfx_benilla` that are gfx bugs, not benilla needs: the x11 raw event
  twice under a grab, the x11 release-as-repeat heuristic, glfw's no-op `set_mouse_position`,
  sdl's late X1/X2, win32's screen-coordinate `set_mouse_position`, milestone 3's GL depth
  attachment and `D32_SFLOAT` fixes, the previous run's vk slot mapping / descriptor fill and the
  gl3/gles3 array depth, and this run's rasterizer depth bias (an API addition).
- The `shaders_gles3_dark` family is compiled here without the Windows-only gamma hack.

## The install

This machine has a 1.12.1 install: `$wow_classic_dir` = `/home/jonas/Downloads/wow_classic`
(build 5875). Point benilla at it with the gitignored `WoW` link at the repo root, or
`WOW_DATA="$wow_classic_dir/Data"`. It is read-only to benilla. A login still needs a
`.probe-identity` account.

## Open problems

- A vk teardown leaks one `VkDeviceMemory` (validation `VUID-vkDestroyDevice-device-05137`).
- Filler streams are per mesh (a `wow_model` mesh lacking the rig and merged attributes carries
  4 filler buffers of its vertex count).
- static-gx pushes one early draw per run and one description per (region, texture, sampler,
  cutout, two-sided) each frame; fine at 59 frames/s in Northshire, unmeasured in a city.
- The UI tile cell clip (`anim_slots.w`) flips the fragment row on GL; untested until model tiles
  draw (milestone 6).
- Log: each `Extract*Plugin` logs "Render app did not exist" once at build.
- GL depth precision: the `2z - w` remap puts reverse-Z into GL's [-1, 1] clip range, so the
  float depth buffer keeps no reverse-Z advantage; coplanar intersections (the waterline, some
  terrain edges) flip single pixels on GL that vk holds. `glClipControl(GL_LOWER_LEFT,
  GL_ZERO_TO_ONE)` (GL 4.5 / `ARB_clip_control`; not in GLES 3) in gfx_benilla would take the
  remap out on gl4 and on gl3 where the extension exists.
- The depth-biased draws (the effect lane's decals, the model zfill twin) do not occur in the
  worldview views; the bias is proven on the parity scene only. Check a ring, blob shadow or
  footprint in the client once a login exists.
- The effect lane uploads the whole stream every frame into a mesh padded to a power of two; its
  cost in a busy scene (a city, a raid) is unmeasured.
- The capture harness is untested under gfx and gfx has no read-back (milestone 6).
- Disk: ~10-25 GB free; `target/debug/deps` collects stale builds of benilla's crates per feature
  set (1.5-3 GB each). Delete the ones older than the current round's.

## Next

Milestone 4's rest: the underwater GlowWave warp (`ffx_glow.wgsl`'s wave arm, `post.rs`), then the
lit-`StandardMaterial` inventory (grep the three sites above; port `pbr` lighting only where one is
actually lit and visible). Then milestone 5 (UI): start from `ui_pass.rs` and the
`ui_{quad,node_gamma,slice_gamma,gamma,add}.wgsl` programs, with a glue-screen A/B (the login
screen needs no account). Lavapipe and llvmpipe first for every new program (GPU safety).
