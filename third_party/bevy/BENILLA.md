# benilla's vendored Bevy: what it is, and how to check it

Upstream: [`bevyengine/bevy`](https://github.com/bevyengine/bevy), MIT OR Apache-2.0, version
`0.18.1` (and `bevy_mikktspace` `0.17.0-dev`, the version 0.18.1 depends on), the versions the
workspace lock resolved to. Beside it, in `third_party/bevy-plugins/`, the Bevy plugins benilla
builds, at their locked versions: [`avian3d`](https://github.com/Jondolf/avian) `0.6.1` with
`avian_derive` `0.2.3`, [`bevy_heavy`](https://github.com/Jondolf/bevy_heavy) `0.4.0`,
[`bevy_transform_interpolation`](https://github.com/Jondolf/bevy_transform_interpolation) `0.4.0`
(all MIT OR Apache-2.0) and [`bevy_egui`](https://github.com/vladbat00/bevy_egui) `0.39.1` (MIT,
dev builds only). Every one is wired in through `[patch.crates-io]` in the workspace root, the way
`kira` and `lua-src` are, so every Bevy crate the build compiles, on any target, is one of these.

## Why a copy exists at all

benilla is being moved onto a Bevy of its own that implements only what benilla uses, with the
same API, so that its code and structure stay exactly as they are (Phase A of `c-port-plan.md`).
This copy is the starting point: each crate is then trimmed to what benilla reaches, every cut a
deletion or a simplification that keeps behaviour (the same system order, change ticks, command
application, message lifetimes, observer and hook timing, asset events). The recorded frame,
`crates/benilla-app/src/game_plugins/frame.txt`, pins the schedule the copy must keep producing.

## What was copied

Each crate is copied from the cargo registry as published: `Cargo.toml` (the registry's
normalized one), `src/`, `README.md`, the licence files, and the files `src/` includes from
outside it (`bevy/docs/`, `bevy_color/docs/`, `bevy_math/images/`). Left out: the registry's
bookkeeping (`.cargo-ok`, `.cargo_vcs_info.json`, `Cargo.lock`, `Cargo.toml.orig`), the
`examples/`, `benches/` and `tests/` the manifests name but nothing here builds, and the
repository files (`.github/`, `.gitignore`, `.cargo/`, lint and format configs, `CHANGELOG.md`,
`CONTRIBUTING.md` and the like). `avian_derive` publishes no licence files; it has `avian3d`'s,
the same repository's.

## What is cut

- **Whole crates:** `bevy_gltf`, `bevy_gilrs`, `bevy_anti_alias` and `bevy_android`. benilla names
  `GltfPlugin`, `GilrsPlugin` and `AntiAliasPlugin` only to disable them in `DefaultPlugins`, so
  each is an empty stand-in in `bevy_internal/src/cut.rs` at its old path (`bevy::gltf`,
  `bevy::gilrs`, `bevy::anti_alias`) and in its old slot of the group; `bevy_asset`, `bevy_winit`
  and `bevy_derive` lost the Android branches that named `bevy_android` (benilla builds for
  Linux, Windows and macOS).
- **`bevy_post_process`** (bloom, motion blur, depth of field, chromatic aberration, MSAA
  writeback): nothing in benilla or the rest of Bevy names it, and its plugin built only render-app
  halves, so it is deleted with its slot in `DefaultPlugins` and its `bevy::post_process`
  re-export; the `bevy_post_process` feature stays as a name enabling `bevy_core_pipeline`.
- **`bevy_picking`**: benilla does its own picking (the mouseover and target pick) and reads
  `bevy_ui`'s `Interaction` (`ui_focus_system`, not a picking system), and names no picking type,
  so the crate is deleted with `DefaultPickingPlugins`' slot in `DefaultPlugins`, the
  `bevy::picking` re-export and the picking prelude, and so is every picking backend and hook
  the other crates compiled under a feature: `bevy_ui`'s `picking_backend` and
  `viewport_picking` (with `ViewportNode`'s required `PointerId` and the `uuid` dependency it
  needed), `bevy_sprite`'s `picking_backend`, `bevy_input_focus`'s `click_to_focus` observer and
  `bevy_egui`'s `picking` feature (its pointer capture system, `EguiPickingOrder`,
  `EguiContextSettings::capture_pointer_input` and `BevyEguiEntityCommandsExt`) and avian3d's
  `bevy_picking` feature (`PhysicsPickingPlugin`, which benilla never adds, and its diagnostics
  row). The `bevy` features `picking`, `bevy_picking`, `mesh_picking`, `sprite_picking` and
  `ui_picking` stay as names enabling nothing.
- **`bevy_gizmos_render`**: nobody names it, and `GizmoRenderPlugin` did nothing in the main
  world but embed its WGSL (its render-app block only logged that no `RenderApp` exists), so it
  is deleted like `bevy_post_process`; the `bevy_gizmos_render` feature enables `bevy_gizmos`.
- **The render-app halves** (no `RenderApp` exists under gfx, so none of it ever ran). Every
  plugin keeps what it did in the main world (systems, sets, assets, types, required components,
  hooks), so benilla's recorded frame is unchanged, and every type benilla or another kept crate
  names stays:
  - `bevy_sprite_render`: the sprite, mesh2d, text2d and wireframe2d pipelines, extraction,
    batching and draw commands, `Mesh2dRenderPlugin` (it had no main-world half) and all WGSL. Kept:
    `SpriteRenderPlugin`'s slice systems and `Sprite`'s `SyncToRenderWorld`, `Material2d`,
    `Material2dKey`, `Mesh2dPipelineKey`, `Material2dPlugin`'s asset and specialization check,
    `ColorMaterial`, the tilemap chunk.
  - `bevy_ui_render`: `UiRenderPlugin` returned before its sub-plugins without a `RenderApp`, so it
    is empty; everything but `UiMaterial`, `UiMaterialKey`, `MaterialNode`, `UiMaterialPlugin`'s
    asset registration, `UiAntiAlias`, `BoxShadowSamples` and `stack_z_offsets` is gone.
  - `bevy_egui`: the `render` module's pipeline, pass node, render graph, extraction,
    render-world systems, paint callbacks and `egui.wgsl`, the graph edges into `Core2d`/`Core3d`
    (and the one against `bevy_ui_render`'s UI pass), `RenderComputedScaleFactor`, the `node`
    names, and the `EguiPlugin` fields only the render half read (`ui_render_order`,
    `bindless_mode_array_size`); also the three render-world systems its `build` added to the
    main app's `Render` schedule, which nothing runs. Kept: `update_egui_textures_system` (egui's
    textures as `Assets<Image>`, which benilla-gfx draws) with the helpers it calls,
    `free_egui_textures_system`, `EguiManagedTextures`, `EguiUserTextures`, and the two
    `ExtractResourcePlugin`s (with `ExtractedEguiManagedTextures`), which log once that no render
    app exists. The `render` and `bevy_ui` features stay; `bevy_ui` enables nothing.
  - `bevy_pbr`: the mesh, material, prepass, shadow, light, cluster, fog, skin and morph render
    code, GPU preprocessing, the material bind-group allocator, SSAO, SSR, volumetric fog,
    atmosphere, light probes and environment-map generation, lightmaps, clustered decals, deferred
    lighting, meshlets (a feature nothing enables), the wireframe plugin and the allocator
    diagnostic (plugins nothing adds), the blue-noise texture and all WGSL. The sub-plugins left
    with nothing in the main world (`MeshRenderPlugin`, `GpuMeshPreprocessPlugin`,
    `LightmapPlugin`, `LightProbePlugin`, `PrepassPipelinePlugin`) are gone; every
    `RenderAssetPlugin`/`ErasedRenderAssetPlugin` (the render-world asset preparation) went, as in
    `bevy_sprite_render`; an `ExtractComponentPlugin` whose extraction built a render type became
    the `SyncComponentPlugin` it added in the main world. Kept: the `Material` and
    `MaterialExtension` traits, `ExtendedMaterial`, `StandardMaterial`, `MeshMaterial3d`,
    `MaterialPlugin`'s asset, type and specialization-check systems, `MaterialPipelineKey`,
    `MaterialExtensionKey`, `MeshPipelineKey`, `MaterialPipeline` and `MaterialExtensionPipeline`
    (the `specialize` signatures name them; `MeshPipeline` inside them is an empty struct, since
    only a `RenderApp` builds one), the prepass's previous-transform systems, the deferred
    lighting-id system, the atmosphere probe system, the forward decal's material and mesh, the
    meshes and the placeholder image the plugins add to their assets, and the components users
    add (fog, SSAO, SSR, atmosphere, lightmap, `ScatteringMedium`).
  - `bevy_core_pipeline`: the 2D and 3D phase items, their extraction, sorting, depth,
    transmission and prepass textures and graph nodes, the prepass and deferred passes, the skybox,
    tonemapping, upscaling, blit and OIT resolve pipelines, the fullscreen material and shader, the
    depth-pyramid mip generation, and all WGSL. The plugins left with nothing in the main world
    (`BlitPlugin`, `UpscalingPlugin`, `CopyDeferredLightingIdPlugin`, `MipGenerationPlugin`,
    `OitResolvePlugin`) are gone, and the Skybox's `ExtractComponentPlugin` (its extraction built
    the render-world uniform) became its `SyncComponentPlugin`. Kept: the `Core2d`/`Core3d` graph
    labels (`CameraRenderGraph` and bevy_egui name them), the cameras' required components, the
    MSAA checks and the OIT depth-usage system, `Tonemapping`, `DebandDither`, `TonemappingLuts`
    with the three LUT images added to `Assets<Image>`, `Skybox`, the OIT settings, the prepass
    markers and `PreviousViewData`.
  - `bevy_render`, its plugins (the first piece; the render-resource, phase, graph and batching
    types stay for now): `RenderPlugin` keeps what it built with no device (the `Shader` asset
    and loader, its plugin set, `RenderAssetBytesPerFrame`), the same set benilla-gfx's
    `RenderMainWorldPlugin` adds, and loses the rendering sub-app, the wgpu renderer
    initialization, the extract loop and entity sync, `render_system` and the graph runner, the
    camera driver node and the WGSL shader libraries it loaded; `PipelinedRenderingPlugin` (it
    did nothing without a `RenderApp`) is empty but for `RenderExtractApp`. The camera, view,
    window, screenshot, globals, texture, mesh, storage, readback, batching, visibility-range,
    occlusion-culling and diagnostics plugins lose their render-app blocks and the render-world
    systems and resources only those added (camera extraction and sorting, window surfaces, view
    targets and uniforms, the screenshot pipeline, the globals buffer, the texture cache, the
    mesh allocator, the readback buffers, the indirect-parameter buffers, the visibility-range
    buffer, the pipeline cache's queue and shader extraction) and all WGSL; every
    `RenderAssetPlugin` went, as in `bevy_sprite_render`. Plugins nothing adds are gone
    (`UniformComponentPlugin`, `GpuComponentArrayBufferPlugin`, `ExtractInstancesPlugin`, the
    binned and sorted render-phase plugins, the three render-asset and mesh-allocator
    diagnostic plugins). Kept in order: `camera_system` in `PostStartup`/`PostUpdate`, the
    cameras' required components and render-graph warning hook, `clear_screenshots` and
    `trigger_screenshots`, `inherit_weights`, `ShaderStorageBuffer`, `SyncWorldPlugin`'s
    observers, `RenderDiagnosticsPlugin`'s `sync_diagnostics`, and every `ExtractComponentPlugin`
    (its `SyncComponentPlugin`) and `ExtractResourcePlugin` (its once-logged error).
  - `bevy_render`, its render-world types (the second piece; `render_resource` and
    `bevy_shader`'s render half are next): the render phases, draw functions and tracked pass
    (`render_phase`, with the `ShaderLabel`/`DrawFunctionLabel` derives), the batching and GPU
    preprocessing buffers (`BatchingPlugin` stays, empty), the render graph and its nodes, slots,
    edges and context (the `RenderLabel`/`RenderSubGraph` labels stay), `RenderContext`, the mesh
    allocator and `RenderMesh`, erased render assets, `RenderAssetPlugin` with render-asset
    extraction and preparation (the `RenderAsset` trait, `RenderAssets` and
    `RenderAssetBytesPerFrame` stay), the `RenderSystems` sets and the `Render` base schedule, the
    wgpu settings and `RenderCreation` (`RenderPlugin` keeps only `debug_flags`; `settings` keeps
    the wgpu re-exports the `AsBindGroup` derive names), the diagnostics recorder and its Tracy GPU
    context (`RenderDiagnosticsMutex` and `sync_diagnostics` stay), the texture cache
    (`CachedTexture` stays for `ViewTarget`), raw Vulkan init, and the Adreno/Mali driver probes.
    No main-world system went.

  The manifests drop the dependencies no kept code uses; no crate's resolved features change
  (`fixedbitset` loses `default`, which only enabled `std`, still on).
- **`bevy` and `bevy_internal`'s manifests** keep only the features the build enables or a
  manifest in it names (the workspace's list, `debug`, `trace_tracy`, `trace_chrome`, and
  avian3d's and `bevy_transform_interpolation`'s `critical-section`, `libm`, `serialize`), each
  enabling what it did minus the cut crates; `bevy`'s examples, tests, dev-dependencies, profiles
  and `dynamic_linking` are gone. The optional dependencies no kept feature reaches left with them
  (`bevy_audio`, `bevy_dev_tools`, `bevy_feathers`, `bevy_ui_widgets`, `bevy_solari`,
  `bevy_remote`, `bevy_camera_controller`; `bevy_winit` stays a crate for bevy_egui and
  benilla-gfx, not a `bevy` feature), and so did every crate only they pulled (gltf, gilrs, cpal,
  rodio and their platform crates).

## How to check

A crate no commit has touched since the copy is byte-identical to the registry crate:

```sh
R=~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f
diff -r -q $R/bevy_ecs-0.18.1 third_party/bevy/bevy_ecs
```

prints only the files left out above. `git log -- third_party/bevy/<crate>` is what changed
since, and why. The first change: the four lint warnings rustc raises on Bevy's own code, which
cargo capped while the crates came from the registry (a path crate's lints are not capped), are
fixed without a change in meaning: `bevy_reflect`'s `pub use ::inventory` names the crate its
glob import also reached, and `bevy_ui`'s three float literals passed to taffy's
`length`/`percent` spell out the `f32` the compiler already fell back to.
