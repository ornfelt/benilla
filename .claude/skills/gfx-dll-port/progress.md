# gfx DLL port - progress

Branch: `gfx-dll-backend`. Status: milestones 1-4 done; milestone 5 (UI) nearly done: the in-world
player UI and (this run) bevy_ui, the glue and loading screens, with `AddUiMaterial`, draw through
gfx and match wgpu (login and realm list: max 1 on vk and gl4). Left in 5: gizmos. The glue
screens' 3D scenes (the login portal, the character booth) are image cameras: milestone 6.

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
- [x] 4. **World shaders.**
  - [x] `WowModelExt` (`wow_model.{vs,fs}.gfxs`, `benilla-world/src/gfx/model.rs`).
  - [x] The shared light buffer (`benilla-world/src/gfx/light.rs`); a test checks every ported
    shader's `#define` row bases (`wow_model`, `static_gx`, `terrain`).
  - [x] FFXGlow (`post.rs`, `ffx_*.fs.gfxs`, `benilla-world/src/gfx/ffx.rs`), and the underwater
    GlowWave warp: `ffx_combine_wave` (its own program, as the WGSL's own pipeline), the 128x128
    LUT an `Rg8Unorm` linear/repeat image (`GfxFfxGlow::wave_lut`), the warp computed on the
    top-down UV and its v turned back on GL.
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
  - [x] Lit `StandardMaterial`. Inventory: every world material is an extension with its own
    program; the plain ones are unlit (nameplates, raid marks, `waterfx`, the menagerie lanes)
    but for `entities.rs`'s streaming fallback cube. `standard.fs.gfxs` now has bevy_pbr's lit arm
    under ambient light alone (benilla spawns no Bevy light): `ambient_light` through
    `EnvBRDFApprox`, times exposure, plus the emissive; the view block's new trailing row
    `ambient` (colour x brightness of the camera's `AmbientLight` over `GlobalAmbientLight`,
    w = `Exposure::exposure()`; `VIEW_BLOCK` 256, other programs declare the 240-byte prefix).
    Not drawn: the metallic-roughness, emissive, occlusion and normal maps, transmission, and
    `double_sided` back-face normal flip (the compiler has no front-facing input).
- [ ] 5. **UI.**
  - [x] The UI lane (`benilla-gfx/src/ui.rs`, `GfxUiLane`; `benilla-app/src/ui_pass/gfx.rs`): the
    lane `Camera2d` draws into `UiTarget` (8-bit sRGB, as the `Camera2d` main texture, no depth)
    - cleared, or grounded by the world view it claims (`FfxBackdrop::source`, drawn by gfx
    though it targets the size-carrier image), whose combine then writes premultiplied gamma
    there (`wave.z`, the `GAMMA_OUT` arm) - then its `Mesh2d` draws in `Transparent2d` order
    (world z ascending, stable), then `ui_gamma` decodes into the scene target over its viewport.
  - [x] `UiQuadMaterial` (`ui_quad.{vs,fs}.gfxs`) through `GfxMaterial2dPlugin` (a `Mesh2d`
    collector); the run tint unpacked from the draw's tag in the fragment stage.
  - [x] Text: `ui_text`'s glyph cells become `GfxTextureWrites` (sub-rect writes applied in
    Prepare through gfx_benilla's `gfx_dll_set_texture_subdata`); a data-less image
    (`Image::new_uninit`, a render target) is made zeroed.
  - [x] bevy_ui (`benilla-gfx/src/bevy_ui.rs`, `GfxBevyUiPlugin`): bevy_ui_render 0.18.1's
    extract / queue / prepare over the main world for nodes on a `GfxUiLane` camera: backgrounds,
    images (atlas, rect, flip), borders, outlines, text and `TextShadow` through `ui_node_gamma`,
    sliced / tiled images through `ui_slice_gamma`, `MaterialNode<M>` through
    `GfxUiMaterialPlugin::<M>::new(describe)` (`AddUiMaterial` -> `ui_add`, `Blend::AddAlpha`).
    Sorted by stack index + bevy's `stack_z_offsets` (stable), batched by image as bevy does,
    written into two meshes rewritten in place (custom attributes `988_2xx`), drawn as the
    lane's late draws (`DrawList::push_late`, after its `Mesh2d` draws, before the decode) through
    bevy's UI view projection (a view block of its own). Not drawn (none in benilla): box
    shadows, gradients, `ViewportNode`, text backgrounds, underline / strikethrough.
  - [ ] Gizmos (bowstring, fishing line).
- [ ] 6. **The rest.** Render-to-texture cameras (16 `RenderTarget::Image` sites: portraits,
  the paper doll and model frames, the minimap composite, a `Camera2d` on an image) and their
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
the same binary; since this run it has a lit cube and a lit metallic, emissive sphere under a
camera `AmbientLight` (6000). `OUT=<dir> .claude/skills/gfx-dll-port/tools/parity.sh x11:gl4 x11:vk ...`.

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

The full client: `tools/client_shot.sh <bin> <out.png>` runs a capture scenario
(`WOW_CAPTURE=ui-bag`, the in-world UI server-less) and imports the window when the log prints
`capture: scene aged`, the moment the wgpu build writes its own `WOW_CAPTURE_OUT`; the gfx build
has no read-back, so its capture fails and exits after that line (expected). Diff the wgpu
capture against the gfx shot with `parity_diff.py`. A leftover `import` holding the X server
blocks the next client's window: kill stray `import`s before a run. A phase-dependent effect
(the underwater warp) needs the phase pinned in both builds for an A/B: a temporary, uncommitted
pin in `ffx_glow.rs::sync_wave` served this run.

## Verified backend pairs

Linux (Debian 13, X11 :0, awesome WM, Radeon 680M/Mesa), `gfx_benilla` Debug (gfx `c8e0bec`), no
account (no `.probe-identity`), `WOW_UNATTENDED=1 WOW_NOSOUND=1`, window tiled by the WM.
- Milestone 5 bevy_ui, this run: `WOW_CAPTURE=glue-login` and `glue-realmlist`, the login
  portal scene pinned off in both builds (a temporary, uncommitted `preview.scene = None` in
  `login/mod.rs::enter_login`: the scene is an image camera gfx does not draw yet), wgpu capture
  vs gfx window. Login: x11/vk and x11/gl4 max 1 (0.000% >1; the background clear 1 level off in
  blue, 22% of UI pixels 1 level); glfw/gles3 and sdl/vk the same but for the text caret, white
  in those shots and hidden in wgpu's (blink phase, 0.016%). Realm list (tiled borders, header
  tabs, the highlighted row, the dimming overlay): x11/vk and x11/gl4 max 1, mean 0.015. First
  on lavapipe with validation (clean) and llvmpipe gl3 / gles3 (no `GL_INVALID`, max 3, 0.37% >1).
- Milestone 5, the previous run, `WOW_CAPTURE=ui-bag` (Northshire, the unit frames, four bags, chat,
  minimap, action bars) through the full client, wgpu capture vs gfx window: x11/vk 0.103% >1,
  0.002% >16 (single pixels on world features, none on the UI); x11/gl4 2.714% >1, 0.029% >16:
  2-level steps over the world backdrop, 63% of them in channel values >= 128 (21% of all
  values), where one 8-bit sRGB store step spans two gamma levels: the GL driver's sRGB-encode
  rounding into the UI target against RADV's (open problem). Before the vk clear fix the world
  backdrop was black (see Build notes). Lavapipe (only the known teardown leak), llvmpipe
  gl3/gles3 (no `GL_INVALID`, no refused sub-rect write) first.
- Milestone 4 finish, this run: the underwater warp (`WOW_FORCE_SUB`, the lake view, particles
  off, phases pinned 0.25/0.6 in both builds) x11/vk max 1, 0.000% >1; x11/gl4 and glfw/gles3
  0.044% >1, 0% >16 (the GL foliage speckle); the same gfx shot unpinned differs 52.4% >1, so the
  check sees the warp. With particles on the drift motes differ as wgpu against itself (0.47%
  >1 floor, gfx 0.67%). The dry lake stays max 1 on vk; the overview 0.004% >1 against a
  reproducible wgpu shot (one wgpu overview run landed on another camera: 69% edges, wgpu
  against wgpu too). Parity scene with the lit shapes: x11/vk max 1, 0.000% >1; gl3/gl4/gles3
  0.001% (the known texel-edge pixels).
- Milestone 4, earlier: liquid, sky family, effect lane, depth bias: lake noon/dawn/dusk/night
  and high dusk/night max 1 on x11/vk; GL 0.005-0.065% >1 (foliage alpha-test, waterline).
  Models, static-gx, terrain, WDL: x11/vk 0.023% >1; x11/gl3, glfw/gles3, x11/gl4 up to 0.28%
  >1 (the same GL foliage speckle).
- Milestone 3: parity scene on all twelve Linux pairs (x11, glfw, sdl) x (gl3, gl4, gles3, vk).
- Milestone 2 (input, grab, scale factor): see git `3db008e3`; not re-run.
Windows (win32, d3d11, d3d12) not built; the d3d11 HLSL of every shader passes glslang's parser.
No GPU reset in any run (`dmesg` count 0).
## Build notes

- gfx library: `~/Code2/General/gfx/gfx_dll/gfx_benilla` (gfx repo `c8e0bec`). Differs from
  `gfx_dll/gfx`: key events carry `physical` + `scancode`, `GFX_EVENT_RAW_MOTION`, per-window
  `scale_factor`, exported cursor / warp / icon / scale-factor calls, milestone-2 backend fixes,
  (milestone 3) the sRGB formats, GL depth attachment by format, `D32_SFLOAT` as float depth, and
  (this run) vk: each sampler slot mapped to (binding, element) against its shader state's layout
  (`vk_map_sampler_slots`; the old write, slot i -> binding 1 element i-1, rolled slot 3 into
  binding 2's array) and every unwritten element of a binding filled from a descriptor of the same
  binding; gl3/gles3: a 2D array keeps its layer count at every mip level (was halved as for 3D);
  and (this run) the rasterizer depth bias, `gfx_dll_create_rasterizer_state_biased` (GL polygon
  offset, vk pipeline bias, d3d11/d3d12 `DepthBias`; the d3d ones not built here, GL drops the
  clamp), and (this run) `gfx_dll_set_texture_subdata` (a sub-rect of one level and layer,
  uncompressed formats; gl3/gl4/gles3 `Tex(ture)SubImage`, vk a buffer-to-image region, d3d11
  `UpdateSubresource` with a box, d3d12 `CopyTextureRegion` at (x, y); d3d9/jkg refuse) and GL
  sRGB writes: gl3/gl4 enable `GL_FRAMEBUFFER_SRGB` while a framebuffer with an sRGB colour
  attachment is bound (`gl_bind_framebuffer`), so the UI target encodes as on vk/d3d/gles3.
  A vk clear lands in the bound render pass: bind a framebuffer before `gfx_dll_clear_color` on
  it (a clear of the UI target with the scene bound erased the world backdrop this run).
  Build: `cd ~/Code2/General/gfx/gfx_dll/gfx_benilla && mkdir -p build && cd build && cmake ..
  -DCMAKE_BUILD_TYPE=Debug && make -j$(nproc)` -> `bin/Debug_x64/libgfx.so`. `benilla-gfx/build.rs`
  takes `gfx_benilla` over `gfx` (`GFX_DIR`, `GFX_CONFIGURATION`) and copies it beside the binary.
- gfx's vk layout keeps its binding-2-is-14 hack (the original C apps' array): a benilla program's
  sampler slots 0..3 are single samplers; slot 2 works through the fill.
- Shaders: sources `crates/benilla-gfx/shaders/src/*.{vs,fs}.gfxs` (`blit`, `present`,
  `standard`, `wow_model`, `static_gx`, `terrain`, `wdl`, `liquid`, `sky`, `celestial`, `star`,
  `cloud`, `effect`, `ffx_downsample`, `ffx_gauss`, `ffx_combine`, `ffx_combine_wave`,
  `ui_quad`, `ui_gamma`, `ui_node_gamma`, `ui_slice_gamma`, `ui_add`); HLSL reserves `point`
  (a geometry-stage keyword): no varying may be named so. `WOW_GFX_SHADERS=<dir>` loads a family tree from disk, e.g. a probe
  variant compiled into the scratchpad (`compile.sh` copied beside a `src/`); `crates/benilla-gfx/shaders/compile.sh [names]` writes the four families. The
  compiler prints samplers in fragment stages only: a vertex stage that fetches declares its
  sampler in the body per backend at the fragment stage's slot (vk `set = 1, binding = slot`).
  The compiler has no depth output (`wdl` clamps per vertex). Its `include` works only in the
  header, and GL needs a uniform block identical in both stages, so programs sharing a WGSL vertex
  stage (the sky family) each carry their own copy. Compiler: `cd
  ~/Code2/General/gfx/wc_compiler_rs && cargo build --release`.
- Build: `CARGO_INCREMENTAL=0 cargo build -p benilla --features gfx` (or `-p benilla-worldview`).
  Incremental caches filled the disk twice this run: build without them.

## Gates (this run)

`CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=8 scripts/check.sh` (escalates to `gates.sh`: a file under
`.claude/` is outside the crate map): ALL GATES GREEN on this run's final tree (1261 s): fmt,
clippy, workspace tests, no-install tests, doc links, pass-span lint, player build and tests, both
enforcers. The gate printed "install or addon corpus not found" (`WoW/Data` under the repo), so
its data-gated tests skipped. `cargo clippy --workspace --all-targets --features benilla/gfx --
-D warnings` green; `cargo test -p benilla-gfx --features gfx` 33 passed. No `smoke.sh`: no
`.probe-identity`. Disk: the gate chain needs ~25 GB; delete `target/debug/deps` executables
(`find target/debug/deps -maxdepth 1 -type f -executable ! -name '*.so' -delete`, plus
`target/debug/examples`, `target/debug/incremental`) and the A/B binaries under `target/ab/`
(2.8 GB each) before it.
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
- GL sRGB store rounding: the UI target on gl4 differs from vk/wgpu by 2 levels over 2.7% of a
  frame (bright channels); measure gl3/gles3 and try `R8G8B8A8Unorm` storage with the encode in
  the shaders if it matters.
- The UI lane assumes the world renders at the window's size (`RenderScale` 1.0): the scene
  target is window-sized, the size-carrier's scale is ignored (milestone 6).
- `GpuImages` keys a sampled variant by (image, sampler): a sub-rect write lands in the image's
  own texture only, so a glyph sheet sampled through another sampler would miss its cells.
- A `Camera2d` outside the lane (the minimap composite on an image, the egui camera) is skipped,
  and bevy_ui on any camera but a lane is not drawn.
- bevy_ui's font atlases are whole-image re-uploads on each `Modified` (a new glyph); cheap on the
  glue screens, unmeasured under heavy bevy_ui text churn.
- The char-create and char-select screens are not A/B'd: their booth scene is an image camera.
- The capture harness under gfx: the shutter fails for want of read-back and exits nonzero.
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
- Disk: ~10-25 GB free; `target/debug/deps` collects stale builds of benilla's crates per feature
  set (1.5-3 GB each). Delete the ones older than the current round's.

## Next

Milestone 6's image cameras, which finish the glue screens: an active camera whose
`RenderTarget::Image` no lane claims draws into a gfx render target of that image's size and
format (colour + depth), and `GpuImages` hands that texture out for the image's id, so the
`ImageNode` showing it (the login portal via `portrait/glue_booth.rs`, the character booth, the
portraits, the paper doll, `ui_models`, the minimap composite) samples it. Their studio light
buffers (`LightBlob` through the noop queue) need mirroring into data textures, as the world's
shared light buffer is. A/B `glue-login` unpinned and `glue-charcreate` with `client_shot.sh`.
Then gizmos (bowstring, fishing line). Lavapipe and llvmpipe first for every new program.
