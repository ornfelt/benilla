# gfx DLL port - progress

Branch: `gfx-dll-backend`. Status: milestones 1-5 done; 6 in flight: image cameras, `Screenshot`
(the capture harness), window modes and level, the `WOW_GPU_MS` meter, MSAA and (this run) the
`dev` egui panel and the window's position / decorations / resizable run through gfx. Left in 6:
the `depth_probe` / `phase_probe` instruments.

## Milestones

- [x] 1. **Scaffold.** `crates/benilla-gfx` (empty without its `gfx` feature, all deps optional):
  `ffi.rs` (the `gfx_dll.h` surface), `shader_def.rs` / `shader_loader.rs` (the `.gfx` format;
  four embedded families, `WOW_GFX_SHADERS=<dir>` override), `backend.rs` (`WOW_GFX_WINDOW` /
  `WOW_GFX_DEVICE`, fail-at-boot naming the choices), `context.rs`, `events.rs`, `runner.rs` (poll
  -> `window::pump` -> `app.update()` -> `GfxRender` -> swap). `lib.rs::swap_in` disables
  `WinitPlugin` + `RenderPlugin`, adds `RenderMainWorldPlugin` and `GfxPlugin`; hooked in
  `benilla-world/src/boot.rs`. Features `benilla/gfx -> benilla-app/gfx -> benilla-world/gfx`, and
  `benilla-worldview/gfx`. `noop_device.rs`: a wgpu `noop` `RenderDevice`/`RenderQueue` for
  main-world startup code (settled with the maintainer, 2026-09-27).
- [x] 2. **Window and input.** `window.rs` is bevy_winit 0.18.1's window half (keys, text,
  buttons, motion, wheel, cursor, focus, size, scale factor; `Window`, `CursorOptions`,
  `CursorIcon` sync). `WOW_GFX_INPUT_TRACE=<path>` logs every message and window call. Not applied
  yet: focus requests (logged once); no runtime scale-factor change, no IME (bevy_egui's IME system
  is off under gfx, `GfxEguiPlugin`). Mode, level, position, decorations and resizable are applied
  since milestone 6. `cover_input_wall` skips
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
- [x] 5. **UI.**
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
  - [x] Gizmos (`benilla-gfx/src/gizmos.rs`, `gizmo_line.{vs,fs}.gfxs`): bevy_gizmos_render
    0.18.1's 3D line pipeline. Each config group's `GizmoAsset` (main world, built in `Last`) is
    expanded into one screen-space quad per segment (the WGSL's `vertex_index` corner an input),
    written into one mesh rewritten in place, drawn per active `Camera3d` whose layers meet the
    group's at transparent-phase distance 0 (list, then strip), alpha blend, depth write, reverse-Z
    `Greater` (`GfxDrawState::depth_strict`, new). Solid lines only; styles, joints, retained
    `Gizmo`s and 2D views log once (benilla uses none). HLSL reserves `line` too: no member so named.
- [ ] 6. **The rest.**
  - [x] Image cameras (`draw.rs`, `target::ImageTarget`, `images::GpuImages::target`): a camera on
    `RenderTarget::Image` draws into its image's main pair (shared by every camera on the image,
    `Hdr` float or 8-bit sRGB, depth for 3D), cleared whole as a wgpu clear op, over its viewport,
    FFXGlow with quarter targets a quarter of the viewport (`FfxPost` keeps them per size), then
    bevy's `upscaling` blit (`blit` program, scissored) into the image's own texture, the image's
    first blit of the frame clearing it with the camera's `output_mode` colour. `GpuImages` hands
    that texture out for the image's id. On GL the camera draws with a clip-space Y flip (culling
    swapped) so the texture's rows run top-down like an upload's; the view block's `misc.y` is 2
    there, and `wow_model`, `ui_quad` and `effect` skip their GL fragment-row flip on it. A 2D camera
    on an image draws its `Mesh2d`s (the minimap composite). Pipelines key on the target class
    (`TargetClass`: colour format, depth).
  - [x] Booth light buffers (`benilla-world/src/gfx/light.rs`): `LightBlob::write` queues its rows
    (gfx-gated), `pack` writes them into the buffer's data texture and copies the mirrored regions
    (rig table, origins, palette; tints; mat-anim) from the shared texture into every buffer on
    `RigPaletteMirrors` / `InstanceTintMirrors` / `MatAnimMirrors` (whole on first sight). The effect
    lane reads a record's own light buffer and its UI-pane clip rect (`effect` draw block `clip`).
  - [x] Screenshots (`screenshot.rs`): a `Screenshot` of the primary window makes the present also
    draw into an RGBA8 capture texture; the next frame's Prepare reads it back
    (`gfx_dll_read_texture`, rows flipped on GL) into bevy's `CapturedScreenshots` channel, which
    gfx re-creates (its sender went to the missing render world). `WOW_CAPTURE` runs write their PNG
    and exit 0 under gfx.
  - [x] Window modes and level (`window.rs`, `context.rs`): `WindowMode` and `WindowLevel` at
    creation (before the first show) and on change, through gfx_benilla's
    `gfx_dll_window_set_mode` / `gfx_dll_window_set_level`. Every fullscreen is borderless on the
    window's own monitor (no exclusive mode, no other monitor: `MonitorSelection` is not read);
    a size request is not applied while fullscreen, as winit's is not; `video.rs`'s leave
    (mode, then resolution, one frame) lands.
  - [x] `WOW_GPU_MS` (`timer.rs`, `GfxGpuMeter`): gfx_benilla timestamps at the start of `Draw`
    and after `Present` (the wgpu meter's camera-driver bracket), a ring of 8 frame pairs read
    back 7 frames later, into `perf::gpu`'s `GpuMsShared` (cfg `gfx`). The wgpu census stays 0.
  - [x] `pipe_warm`: nothing to port. Under gfx `PipeWatch` counts 0 created / 0 settled, so the
    cover never waits on it, and the menagerie drawn behind the cover makes gfx's own pipelines
    (made on first draw) as a side effect. Not seen live: it runs only on world entry (a login).
  - [x] MSAA (`target::MsaaTarget`, `draw.rs`): a 3D camera whose `Msaa` is above 1 draws its
    phases into its target's multisampled colour and depth (`Texture2DMs`, made on first use per
    `SceneTarget`: the window's scene and every image's main pair), cleared there, then
    `Cmd::Resolve` resolves the colour into the pair's current colour (bevy's `resolve_target`),
    which FFXGlow, the blit and the present read; depth is not resolved, as bevy's is not.
    `TargetClass.samples` keys the pipelines, whose rasterizer multisamples on an MS class.
    `MsaaFormats` and the `gxMultisample` clamp come from the gfx device's counts
    (`gfx_dll_get_msaa_counts` -> `GfxMsaaCounts`, inserted by the runner before the first update;
    `view::grant_gfx_msaa` in `PreStartup`, the wgpu `finish` path's `grant`). Not done: bevy's
    `MsaaWriteback` (an MSAA camera that does not clear draws over an empty MS target; logged
    once, none in benilla), and 2D cameras stay single-sampled (none multisamples).
  - [x] The `dev` egui panel (`overlay.rs`, `egui.rs`, `egui.fs/vs.gfxs`,
    `overlay_composite.fs/vs.gfxs`): a 2D window camera with a `GfxOverlays` frame is an overlay
    view: its draws (one vertex + index stream, per draw an image, an index range and a scissor)
    go into the overlay target (8-bit sRGB, the `Camera2d` main texture), cleared with the camera's
    clear, through a port of bevy_egui 0.39.1's `egui.wgsl` (premultiplied blend); then bevy's
    `upscaling` over the frame through the camera's output blend, as `overlay_composite`: the
    scene is read as the sRGB swapchain would hold it (clamped, stored as bytes) and the blend done
    in the shader into the other ping-pong colour (a hardware blend over the float scene brightened
    everything above 1.0 under the panel's 12% see-through). `egui.rs` (feature `benilla-gfx/egui`,
    on from `benilla-app`'s `dev` when `gfx` is on: `benilla-gfx?/egui`) fills the frames from
    each context camera's `EguiRenderOutput` and `EguiManagedTextures` (already `Assets<Image>`),
    as bevy_egui's `prepare_egui_render_target_data_system`; paint callbacks and user textures are
    not drawn (benilla has neither, logged once). `boot.rs`'s gfx-only log filter is gone.
  - [x] Window position / decorations / resizable (`window.rs::apply_position`, `context.rs`):
    at creation before the first show and on change, through gfx_benilla's
    `gfx_dll_window_set_position` / `_center` / `_set_decorations` / `_set_resizable`.
    `Centered` centres on the window's own monitor (x11: the root window). benilla keeps all three
    at their defaults; measured on the parity window (`PARITY_WINDOW`, `tools/window_props.sh`).
  - [ ] `depth_probe`, `phase_probe` (render-graph nodes).
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
the same binary, with a lit cube and a lit metallic, emissive sphere under a camera
`AmbientLight` (6000); `PARITY_MSAA=2|4|8` multisamples its camera in both paths. Built with
`--features egui`, `PARITY_EGUI=1` adds the debug panel's overlay camera and a panel;
`PARITY_WINDOW=[later,]nodeco,fixed,center,at=<x>:<y>` sets the window's frame, sizing and place
(`tools/window_props.sh` samples what the WM made of it).
`OUT=<dir> .claude/skills/gfx-dll-port/tools/parity.sh x11:gl4 x11:vk ...`. Waiting on a build in
a shell loop: never `pgrep -f` / `pkill -f` a pattern that the waiting command's own line contains
(it matches itself and never ends, or kills itself).

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
stale copy there runs the old library). `target/ab/worldview-wgpu` was deleted for disk at the end of this run: rebuild both binaries
(feature off, then gfx) before an A/B. `validate_gfx.py crates/benilla-gfx/shaders`
runs glslangValidator over every family's GL/GLES text and the d3d11 HLSL.

The full client: since this run both builds write their own `WOW_CAPTURE_OUT` (the gfx build
through `screenshot.rs`), the exact aged frame under the capture's frozen clock, so an animated
scene (the login portal's embers, fire and scrolling textures) diffs exactly: two wgpu runs of
`glue-login` and `ui-unitframes` are identical (max 0). `scratchpad`-style runner: lavapipe with
validation first, then the wgpu build, then gfx on each pair, `parity_diff.py` against the wgpu
PNG (this run's loop is worth re-creating as a tool: scenario list in, one diff line per pair
out). `tools/client_shot.sh` (window import at `capture: scene aged`) is now only for a window
check; its import lands frames after the aged one, so it cannot A/B an animated scene. A leftover
`import` holding the X server blocks the next client's window: kill stray `import`s before a run.
A phase-dependent effect (the underwater warp) needs the phase pinned in both builds for an A/B:
a temporary, uncommitted pin in `ffx_glow.rs::sync_wave` served before.

## Verified backend pairs

Linux (Debian 13, X11 :0, awesome WM, Radeon 680M/Mesa), `gfx_benilla` Debug (gfx
`d98d2c0`), no account (no `.probe-identity`), `WOW_UNATTENDED=1 WOW_NOSOUND=1`, window tiled by the WM.
- This run, egui: the parity scene with `PARITY_EGUI=1` (a debug-panel-styled window: text,
  monospace, a slider, a checkbox, filled rects, a scroll area clipped by its scissor) against
  wgpu in the same binary: x11/vk and sdl/vk max 1 (0.000% >1); x11/gl4, x11/gl3, glfw/gles3
  0.001% >16, none in the panel (the known scene texel-edge pixels). Lavapipe with validation
  first (only the known teardown leak), llvmpipe gl3 / gles3 no `GL_INVALID`. The full client,
  `WOW_PANEL=1 WOW_CAPTURE=glue-login` (the panel open over the login portal), both builds' own
  PNGs: x11/vk, sdl/vk, glfw/vk 0.001% >4 (max 12, the portal's), the panel region max 1;
  x11/gl4, x11/gl3, sdl/gl4, glfw/gles3 0.002% >4. Input: one synthetic click (`xsend`) on the
  panel's "Models" header under x11/vk expands it (trace: 2 `MouseButtonInput`); wgpu not driven
  (winit reads XI2, not the core events `xsend` sends). No GPU reset.
- This run, window properties (`tools/window_props.sh`, geometry + `_MOTIF_WM_HINTS` +
  `WM_NORMAL_HINTS` at 1.2 s, or after a change at 1.5 s): native x11 (vk, gl4) matches winit for
  the default, `nodeco`, `fixed` (the WM floats a min = max window: 640x400+0+22 on both),
  `nodeco,fixed,at=300:120` (640x400+300+120 on both) and the same applied later (both stay
  tiled, frame off, min = max the tiled size). `center`: gfx centres (640x400+640+340); winit's
  window lands where the WM puts it (+0+22). sdl and glfw float, frame and position as asked, but
  at gfx's grown size (1641x1026, `pick_window_size`) and with the client, not the frame, at the
  position (see Open problems). Before this run the vk x11, sdl and glfw windows mapped inside
  `gfx_dll_create_window`, so nothing set "before the first show" reached the WM's first look.
- This run, MSAA: the parity scene at `PARITY_MSAA=4` against wgpu's 4x: x11/vk and sdl/vk max 1
  (0.000% >1); x11/gl4, x11/gl3, glfw/gles3 0.39% >1: every GL device multisamples (gl4 4x against
  its own 1x 0.93%, wgpu's 0.98%) but on edges wgpu does not match, 53% of diagonal edge pixels
  >16 against 10% of axis-aligned ones: GL draws the scene bottom-up, so the standard sample
  pattern is mirrored in y against the image (open problem). The world (`benilla-worldview`,
  the Northshire overview, `WOW_MSAA=4`, both builds): x11/vk and sdl/vk 0.017% >1, 0% >16,
  against a wgpu-vs-wgpu floor of 0.009% and gfx-vs-wgpu at 1x of 0.010%; 4x against 1x changes
  1.766% (gfx) / 1.771% (wgpu) of the frame, 0.294% / 0.295% >16. x11/gl4 0.93% >1 (the mirrored
  pattern). Both builds log `msaa: 4x accepted (this GPU offers [1, 2, 4, 8])` (so `MsaaFormats`
  matches wgpu's on RADV); lavapipe offers `[1, 4]` and runs the world at 4x clean under
  validation (only the known teardown leak); llvmpipe gl3 / gles3 `[1, 2, 4]`, no `GL_INVALID`.
  The view log line now names the count (`gfx: views [0 3D, glow, 4x ...]`). No GPU reset.
- This run, gizmos: the parity scene with gizmo lines (a bowstring-like pair, the fishing line's
  64-segment sagging strip, a translucent line, a gradient line clipped by the near plane) against
  wgpu's capture of the same binary: x11/vk and sdl/vk max 1 (0.000% >1); x11/gl4 and glfw/gles3
  0.001% >4 (the known texel-edge pixels); llvmpipe gl3 / gles3 0.005% >4, no `GL_INVALID`;
  lavapipe with validation first (only the known teardown leak). The wgpu shot shows the lines.
- This run, window modes: the full client with `WOW_BG=0` (so `gxWindow` 0, borderless) opens
  `_NET_WM_STATE_FULLSCREEN` 1920x1080+0+0 on x11/vk, sdl/vk and glfw/gl4, as the wgpu build does;
  a background run (`AlwaysOnBottom`) shows the same 1019x1014 tile and no `_NET_WM_STATE` in both
  builds (awesome keeps no BELOW for winit either, so the level is unobservable here). A live
  switch (a scratch example: fullscreen at 2 s, windowed 800x500 at 5 s, `xprop` / `xwininfo`
  sampled): windowed -> fullscreen 1920x1080 -> windowed with the state cleared, identical on wgpu,
  gfx x11/vk, sdl/vk and glfw/gl4.
- This run, `WOW_GPU_MS`: the parity scene's reading (`WOW_GPU_MS=1 parity gfx` prints it) on
  lavapipe with validation 4.9 ms (clean), llvmpipe gl3 4.0 / gles3 4.3 ms, x11/vk 1.2-1.4,
  sdl/vk 1.2, x11/gl4 0.5-0.6, x11/gl3 0.56-0.58, glfw/gles3 0.6-0.9 ms; not A/B'd against the
  wgpu meter (its one reader, `WOW_LIVE_FPS`, needs a connected world).
- This run, regression: `glue-login` x11/vk 0% >4 (max 8), x11/gl4 0.002% >4; `glue-charcreate`
  vk and gl4 0.003% >4: the previous run's numbers.
- Milestone 6 image cameras and screenshots, this run: both builds' own `WOW_CAPTURE_OUT` PNGs,
  1019x1014. `glue-login` (the portal scene live): x11/vk and sdl/vk 0.005% >1, max 8, 0% >4;
  x11/gl4 0.074% >1, 0.002% >4; glfw/gles3 0.002% >4; llvmpipe gl3 / gles3 0.022% >4.
  `glue-charcreate` (the booth: rigged character, pet-less, the scene): x11/vk and sdl/vk 0.376%
  >1, 0.003% >4; x11/gl4 (after the GL clear fix) 0.003% >4; glfw/gles3 0.004% >4; llvmpipe 0.107%
  >4. `ui-char` (paper doll, server-less): vk 0.002% >4, gl4 0.024% >4 (the GL sRGB rounding).
  `ui-unitframes`: the world and frames match, the chat frame and one small bar sit ~50 px lower
  in gfx on both devices (open problem; UI layout, not drawing). Lavapipe with validation first for
  every scenario (only the known teardown leak); no `GL_INVALID`. Before the GL clear fix gl4 drew
  the char-create character as a dark silhouette: see Build notes.
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

- gfx library: `~/Code2/General/gfx/gfx_dll/gfx_benilla` (gfx repo `8ecf4eb`). Differs from
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
  it (a clear of the UI target with the scene bound erased the world backdrop once).
  This run: `gfx_dll_read_texture` (level 0 of a 2D texture, texel-row order, after the frames
  already submitted: gl3/gl4/gles3 a scratch FBO + `glReadPixels` in `gl.c`, gles3 RGBA8 only;
  vk a copy to a host-visible buffer after `vkDeviceWaitIdle`, colour targets now with
  `TRANSFER_SRC`, `TRANSFER_SRC_OPTIMAL` added to `transition_image_layout`; d3d11 a staging
  texture; d3d12 a readback buffer on the upload list then `wait_gpu_idle`; the d3d ones not built
  here), `gfx_format_texel_size` in `objects.h`, and GL clears that ignore the write masks
  (`gl_clear_masks_open`/`close` around every gl3/gl4/gles3 `ClearBuffer*`): a GL clear honours
  `glColorMask`/`glDepthMask`, so after any pipeline without depth (or colour) writes, the present
  included, the depth clear did nothing; every GL A/B before this run ran on stale depth, which a
  static view hides.
  This run: `gfx_dll_window_set_mode` / `gfx_dll_window_set_level` (optional `gfx_window_def`
  entries outside `GFX_WINDOW_DEF`, set on the x11, sdl, glfw and win32 defs of every device;
  wayland, android and emscripten ignore them): x11 `_NET_WM_STATE` FULLSCREEN / BELOW / ABOVE
  through `gfx_x11_set_wm_state` (dlopen'd, the property before the map, client messages after;
  also the glfw and sdl backends' bottom level on x11), sdl `SDL_WINDOW_FULLSCREEN_DESKTOP` and
  `SDL_SetWindowAlwaysOnTop`, glfw `glfwSetWindowMonitor` at the monitor's current mode with
  `GLFW_AUTO_ICONIFY` off and `GLFW_FLOATING`, win32 a `WS_POPUP` over `MonitorFromWindow`'s rect
  and `SetWindowPos` (`HWND_BOTTOM` / `HWND_TOPMOST`; also sdl and glfw on win32; not built).
  And GPU timestamps (optional `gfx_device_op` entries outside `GFX_DEVICE_OP_DEF`; d3d9 and jkg
  have none): `gfx_dll_write_timestamp` / `gfx_dll_read_timestamp`, 64 slots, never waiting; GL
  3.3 `glQueryCounter` (gl3/gl4) or `GL_EXT_disjoint_timer_query` (gles3, checked by extension
  before any name is looked up), vk one query pool per frame in flight harvested after its fence
  and reset before its first render pass (a reset may not sit inside one), d3d11 a timestamp and
  a disjoint query per slot, d3d12 a query heap resolved into a readback buffer, read once the
  writing frame's fence passed (d3d ones not built).
  This run (MSAA): vk multisampled images (`GFX_TEXTURE_2D_MS`, `levels` the sample count, one
  mip; `gfx_texture.samples`), a framebuffer's sample count from its attachments, a pipeline made
  for a target framebuffer rasterizing at that count, and `vk_resolve_framebuffer` for real
  (colour only: the pass ends, `vkCmdResolveImage`, the destination left shader-readable, the
  bound framebuffer resumes; `vk_bind_framebuffer` split into `vk_end_render_pass` /
  `vk_begin_render_pass`; an MS attachment is not handed to `SHADER_READ_ONLY` at a pass end);
  GL resolves lift the scissor test and open the write masks (`gl_blit_open` / `gl_blit_close`)
  and gl3/gles3 rebind the framebuffer the device's cache names (their blit bound read/draw
  itself, so the next `gl_bind_framebuffer` of the cached one was skipped); `GL_MULTISAMPLE` only
  on desktop GL (`multisample_control`; GLES has no such enum); GL `max_msaa` also bounded by
  `GL_MAX_DEPTH_TEXTURE_SAMPLES`; `gfx_dll_get_msaa_counts` (a mask of counts; vk from its
  colour-and-depth limits, others every power of two up to `max_msaa`, d3d11/d3d12 4). d3d11 and
  d3d12 already created MS textures, keyed PSOs on samples and resolved with
  `ResolveSubresource`: unchanged, not built here.
  This run: `gfx_dll_window_set_position` / `gfx_dll_window_center` /
  `gfx_dll_window_set_decorations` / `gfx_dll_window_set_resizable` (optional `gfx_window_def`
  entries on the x11, sdl, glfw and win32 defs): x11 `WM_NORMAL_HINTS` (USPosition | PPosition,
  NorthWest gravity; a fixed size as min = max, rewritten on resize), `XMoveWindow` (now loaded),
  `_MOTIF_WM_HINTS`; sdl `SDL_SetWindowPosition` / `SDL_WINDOWPOS_CENTERED_DISPLAY` /
  `SDL_SetWindowBordered` / `SDL_SetWindowResizable`; glfw `glfwSetWindowPos` (centre from
  `window_monitor`'s mode) and the `GLFW_DECORATED` / `GLFW_RESIZABLE` attributes; win32
  `SetWindowPos` and the caption / sizing style bits around the kept client rect (`restyle`; only
  the saved windowed style while fullscreen; not built). The vk x11 window is no longer mapped in
  its create, and sdl (`SDL_WINDOW_HIDDEN`) and glfw (`GLFW_VISIBLE` false) create hidden: all
  map at `show`, as gl x11 always did.
  Build: `cd ~/Code2/General/gfx/gfx_dll/gfx_benilla && mkdir -p build && cd build && cmake ..
  -DCMAKE_BUILD_TYPE=Debug && make -j$(nproc)` -> `bin/Debug_x64/libgfx.so`. `benilla-gfx/build.rs`
  takes `gfx_benilla` over `gfx` (`GFX_DIR`, `GFX_CONFIGURATION`) and copies it to `target/debug/`
  at build time only: after a library rebuild, copy it there by hand. A binary's RUNPATH is
  `$ORIGIN/..` for an example (`target/debug/examples/parity` loads `target/debug/libgfx.so`);
  check which copy loads with `ldd`. A stale copy cost this run one wrong measurement.
- gfx's vk layout keeps its binding-2-is-14 hack (the original C apps' array): a benilla program's
  sampler slots 0..3 are single samplers; slot 2 works through the fill.
- Shaders: sources `crates/benilla-gfx/shaders/src/*.{vs,fs}.gfxs` (`blit` (the image targets'
  `upscaling`), `present`, `gizmo_line`, `egui`, `overlay_composite` (a window camera's
  `upscaling`),
  `standard`, `wow_model`, `static_gx`, `terrain`, `wdl`, `liquid`, `sky`, `celestial`, `star`,
  `cloud`, `effect`, `ffx_downsample`, `ffx_gauss`, `ffx_combine`, `ffx_combine_wave`,
  `ui_quad`, `ui_gamma`, `ui_node_gamma`, `ui_slice_gamma`, `ui_add`); HLSL reserves `point`
  and `line` (geometry-stage keywords): no varying or block member may be named so. `WOW_GFX_SHADERS=<dir>` loads a family tree from disk, e.g. a probe
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

`CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=8 scripts/check.sh` (escalates to `gates.sh`: files under
`.claude/` are outside the crate map): ALL GATES GREEN on this run's code (1543 s): fmt, clippy,
workspace tests, no-install tests, doc links, pass-span lint, player build and tests, both
enforcers; the install is not under the repo's `WoW/Data` for the gates, so the 30 data-gated
tests (no addon corpus) skipped. `cargo clippy --workspace --all-targets --features benilla/gfx
-- -D warnings` green; `cargo test -p benilla-gfx --features egui` 39 passed. No `smoke.sh`: no
`.probe-identity`. Disk: the gate chain needs ~25 GB; delete `target/debug/deps` executables
(`find target/debug/deps -maxdepth 1 -type f -executable ! -name '*.so' -delete`, plus
`target/debug/examples`) and the A/B binaries under `target/ab/` (2.8 GB each) before it.

## For the maintainer

- Deferred (maintainer, 2026-09-27): mouselook's `CursorGrabMode::Locked` stays a real gfx grab.
- Window size is not a parity target (maintainer): the WM may tile a gfx window.
- Upstream candidates in `gfx_benilla` that are gfx bugs, not benilla needs: the x11 raw event
  twice under a grab, the x11 release-as-repeat heuristic, glfw's no-op `set_mouse_position`,
  sdl's late X1/X2, win32's screen-coordinate `set_mouse_position`, milestone 3's GL depth
  attachment and `D32_SFLOAT` fixes, the vk slot mapping / descriptor fill and the gl3/gles3
  array depth, the rasterizer depth bias (an API addition), and this run's GL clears under the
  write masks (a real bug for any gfx app that clears after a no-write pipeline) and
  `gfx_dll_read_texture` (an API addition).
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
- The UI model tiles (`ui_models`, perspective panes into one atlas with viewports), the minimap
  composite (a 2D camera on an image) and the tile cell clip (`anim_slots.w`, the effect `clip`)
  draw but are not A/B'd: no server-less capture scenario shows them. Look for one (or a login).
- GL sRGB store rounding: the UI target on gl4 differs from vk/wgpu by 2 levels over 2.7% of a
  frame (bright channels); measure gl3/gles3 and try `R8G8B8A8Unorm` storage with the encode in
  the shaders if it matters.
- The UI lane assumes the world renders at the window's size (`RenderScale` 1.0): the scene
  target is window-sized, the size-carrier's scale is ignored (milestone 6).
- `GpuImages` keys a sampled variant by (image, sampler): a sub-rect write lands in the image's
  own texture only, so a glyph sheet sampled through another sampler would miss its cells.
- A `Camera2d` on the window outside the lane without a `GfxOverlays` frame is skipped (the egui
  camera is an overlay), and bevy_ui on any camera but a lane is not drawn. A `Screenshot` of
  anything but the primary window is not taken.
- egui under gfx: paint callbacks and user textures are not drawn (none in benilla), no IME; a
  partial font-atlas update re-uploads the whole atlas; the overlay's composite runs a full-window
  pass whenever the panel is open.
- sdl, glfw (and win32) grow a new window to 95% of the monitor's work area
  (`pick_window_size`, the original gfx's choice) where winit creates it at the asked size, so a
  fixed-size window there is the grown size (1641x1026 for 640x400 here); and they place the client
  area, not the frame, at a `WindowPosition::At` (the WM shifted `at=300:120` to +279+54). Window
  size is not a parity target (maintainer); raise it if a fixed window ever matters.
- `WindowPosition::Centered` ignores its `MonitorSelection` (the window's own monitor; x11 native:
  the root window, so a multi-monitor desktop centres across all of them).
- bevy_ui's font atlases are whole-image re-uploads on each `Modified` (a new glyph); cheap on the
  glue screens, unmeasured under heavy bevy_ui text churn.
- `ui-unitframes`: the chat frame and a small bar sit ~50 px lower in the gfx build (both devices;
  wgpu is identical run to run). Layout, not drawing: `UIParent_ManageFramePositions` moves the
  chat frame when a bottom bar shows. Suspect the window's size or focus event order at startup
  (`WOW_GFX_INPUT_TRACE` against winit's); `ui-bag` matched last run.
- A vk render pass loads with `DONT_CARE` from `UNDEFINED`: every target that keeps content across
  passes (the scene ping-pong, a sleeping booth's image) relies on the driver keeping it; RADV
  and lavapipe do.
- Data textures of booth light buffers that go (a UI model pool buffer) are never dropped
  (`GfxDataTextures` is keyed by buffer id, with no removal hook); FFXGlow quarter targets are
  kept per viewport size until the scene target is re-made.
- The effect lane's clip has no row cap: wgpu draws a pane past `MAX_CLIP_ROWS` (10) unclipped.
- The UI lane's decode scissor is not applied on GL (the pass's rasterizer has the scissor test
  off); only a lane viewport smaller than the window would show it.
- Char-select (a ghost, a pet) is not A/B'd: it needs a login.
- Log: each `Extract*Plugin` logs "Render app did not exist" once at build.
- GL MSAA sample pattern: GL draws the scene target bottom-up (the present flips it), so the
  standard 4x pattern is mirrored in y against the image and diagonal edges resolve differently
  from vk/wgpu (0.39% of the parity scene, 0.93% of the world overview >1). The same
  `glClipControl` change as below (drawing top-down on GL) would remove it; gles3 has no clip
  control.
- Image-camera MSAA (`Cmd::ImageView` on an image's MS target, the resolve into its main pair)
  is written and runs through the same code as the window's, but no benilla image camera
  multisamples (the booths are `Msaa::Off`), so it has not been exercised.
- GL depth precision: the `2z - w` remap puts reverse-Z into GL's [-1, 1] clip range, so the
  float depth buffer keeps no reverse-Z advantage; coplanar intersections (the waterline, some
  terrain edges) flip single pixels on GL that vk holds. `glClipControl(GL_LOWER_LEFT,
  GL_ZERO_TO_ONE)` (GL 4.5 / `ARB_clip_control`; not in GLES 3) in gfx_benilla would take the
  remap out on gl4 and on gl3 where the extension exists.
- The depth-biased draws (the effect lane's decals, the model zfill twin) do not occur in the
  worldview views; the bias is proven on the parity scene only. Check a ring, blob shadow or
  footprint in the client once a login exists. Re-run the milestone 4 GL world A/Bs on a moving
  camera now that GL clears depth.
- The effect lane uploads the whole stream every frame into a mesh padded to a power of two; its
  cost in a busy scene (a city, a raid) is unmeasured.
- `AlwaysOnBottom` cannot be seen on this WM: awesome keeps no `_NET_WM_STATE_BELOW` for the gfx
  window or winit's. Check the level on another WM (or Windows) before calling it matched.
- The gfx GPU meter reads plausibly on every device (GL ~0.5 ms, vk ~1.2 ms on the parity scene)
  but is not A/B'd against the wgpu meter; vk stamps `BOTTOM_OF_PIPE` in gfx's command buffer,
  GL at the counter's point in the stream.
- Disk: ~10-25 GB free; `target/debug/deps` collects stale builds of benilla's crates per feature
  set (1.5-3 GB each). Delete the ones older than the current round's.

## Next

Milestone 6's last item: the `depth_probe` / `phase_probe` instruments
(`crates/benilla-app/src/capture/{depth_probe,phase_probe}.rs`, render-graph nodes; read-back
exists; the depth needs a copy into a colour texture, and under MSAA a depth resolve, which vk
refuses today). Then milestone 7: every Linux window/device pair to the character screen and into
the world, and `GFX.md`. With a `.probe-identity` account: A/B the bowstring or fishing line in the client, the
client at `gxMultisample 4` (the Video dropdown's list against wgpu's), `WOW_LIVE_FPS` with
`WOW_GPU_MS=1` on both builds, and watch the warm pass on world entry. Lavapipe and llvmpipe first
for every new program.
