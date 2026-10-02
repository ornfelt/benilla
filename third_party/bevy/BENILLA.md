# benilla's vendored Bevy: what it is, and how to check it

Upstream: [`bevyengine/bevy`](https://github.com/bevyengine/bevy), MIT OR Apache-2.0, version
`0.18.1` (and `bevy_mikktspace` `0.17.0-dev`, the version 0.18.1 depends on, since deleted), the
versions the workspace lock resolved to. Beside it, in `third_party/bevy-plugins/`, the Bevy plugins benilla
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
- **`bevy_input_focus`**: nothing adds its plugins (`InputDispatchPlugin`, the tab and directional
  navigation plugins are not in `DefaultPlugins`) and benilla names none of its types, so the crate
  is deleted with its `bevy::input_focus` re-export, its patch line and `bevy_ui`'s
  `auto_directional_navigation` module (a system parameter over it that nothing uses); the
  `bevy_input_focus` feature stays as a name enabling nothing. Its `Reflect` types leave the
  auto-registered type registry.
- **`bevy_winit`**, to what bevy_egui names: the gfx library owns the window and nothing adds
  `WinitPlugin`, so the plugin, its runner and event loop, window and monitor creation and sync,
  the converters, the cursor plugin, `WinitSettings` and the AccessKit adapters are gone. Kept:
  `WINIT_WINDOWS` with `WinitWindows` and `get_window` (always empty; bevy_egui's IME system looks
  a window up in it), `WinitUserEvent` and `EventLoopProxyWrapper` (never inserted; bevy_egui
  takes it as an `Option`). The features `custom_cursor` (now only `bevy_window/custom_cursor`),
  `x11`, `wayland` and the default stay, and bevy_egui's `accesskit` feature (off, and its two
  systems reached into the AccessKit adapters) is gone with its `bevy_a11y` dependency.
- **`bevy_a11y` and `accesskit`**: with no AccessKit adapter (no `WinitPlugin`) nothing reads an
  `AccessibilityNode`. The crate is deleted with its `DefaultPlugins` slot (its plugin only
  initialized two resources nobody reads), its `bevy::a11y` re-export and patch line. `bevy_ui`'s
  accessibility systems (`label_changed`, `image_changed`, `button_changed`, `calc_bounds`) run in
  `PostUpdate`, which benilla runs single-threaded, where deleting a system reorders the ones that
  stay; so they are **empty stand-ins** with the same paths, added at the same point with the same
  ordering, and the recorded frame is byte-identical. `interaction_states`' six observers, which
  mirrored `InteractionDisabled`/`Checked` into the `AccessibilityNode`, are empty stand-ins too
  (still registered, so the observer entities are spawned as before).
- **`bevy_egui`'s web and Android code** (benilla builds for Linux, Windows and macOS): the
  mobile-web text agent and web clipboard (`text_agent.rs`, `web_clipboard.rs`), the
  `SubscribedEvents` listeners and every `wasm32`/`android` branch, so the clipboard is arboard's
  under `manage_clipboard` alone and `ModifierKeysState` reads macOS from the target; the manifest
  lost its wasm-only dependencies, its example list and its dev-dependencies. No system benilla's
  targets add went.
- **The web, Android and iOS code of the rest** (bevy_ecs's went with its off features, below): `bevy_log`'s
  Android, wasm and iOS layers (`android_tracing.rs`), `bevy_asset`'s wasm HTTP reader
  (`io/wasm.rs`) with every `wasm32` branch of its sources, server, processor log and web
  reader, `bevy_app`'s browser runner loop and panic hook, `bevy_tasks`' web task (a `Task` is an
  `async_task::Task` alone; the single-threaded pool stays for builds without
  `multi_threaded`), `bevy_platform`'s `web` alias with `web_time` and its `exports` module,
  `bevy_reflect`'s wasm constructor call, `bevy_render`'s wasm screenshot download, WebGL
  padding and wasm-atomics wrapper, `bevy_shader`'s and `bevy_light`'s wasm/WebGL branches
  (four cascades), the WebGPU `todo!` in the `AsBindGroup` derive's output, avian3d's
  wasm fallback to synchronous tree optimization, and the `android`/`ios` arms of
  `bevy_diagnostic`, `bevy_asset`'s open-file limit and the bindless slab limit. The `web`
  features of `bevy_app`, `bevy_platform` and `bevy_reflect` and every `webgl`/`webgpu` feature
  are gone with the target-specific dependencies; `bevy`'s `webgl2` and `android_shared_stdcxx`
  (the workspace names them) stay as names enabling nothing. Every removed branch was compiled
  out on Linux, Windows and macOS: no system, plugin or type there changed, and the Linux
  build's resolved features change only in wgpu-types' `web` and
  `fragile-send-sync-non-atomic-wasm`, which gate wasm32 code alone.
- **`bevy_input` and `bevy_window`, to what benilla and the kept crates call.** `bevy_input`:
  the gamepad module (no gamepad backend: `bevy_gilrs` is cut), with its two `PreUpdate` systems,
  nine messages, the `Axis` resource only it used and the `gamepad` feature (`bevy`'s `gamepad`
  and `bevy_gilrs` stay as names enabling nothing); `common_conditions` (no caller);
  `ButtonInput`'s `all_pressed`, `all_just_pressed`, `all_just_released`, `any_just_pressed`,
  `any_just_released`, `clear_just_pressed`, `clear_just_released`; `Touch` keeps its position
  alone and `Touches` what `bevy_ui`'s `ui_focus_system` reads (`any_just_pressed`,
  `any_just_released`, `first_pressed_position`), `touch_screen_input_system` still updates it;
  `derive_more`, `log` and `thiserror` left the manifest. `bevy_window`: `Window`'s
  maximize/minimize/drag requests with `InternalWindowState`'s four request fields (only
  `WinitPlugin` read them), `WindowResizeConstraints::check_constraints`,
  `WindowPosition::new`/`center`, `WindowResolution::set_scale_factor_and_apply_to_physical_size`,
  and `RawHandleWrapper` down to the display handle benilla reads (its constructor,
  window-handle accessors and `ThreadLockedRawWindowHandleWrapper` went; nothing under gfx
  inserts one). `WindowWrapper` stays for `bevy_winit`'s `get_window`. The frame lost the two
  gamepad systems and `PreUpdate`'s one sync point, which was ordered only between them.
- **`bevy_state` and `bevy_time`, to what benilla and the kept crates call.** `bevy_state`
  keeps plain `States` (the derive, `State`, `NextState::set`, `init_state`/`insert_state`,
  `in_state`, `OnEnter`/`OnExit`, the `StateTransition` schedule with its four sets): computed
  and sub states with `StateSet` and the `SubStates` derive, `DEPENDENCY_DEPTH`,
  `OnTransition` with its `run_transition` system, `DespawnOnEnter`/`DespawnOnExit` with their
  two `StateTransition` systems, state-scoped messages, `CommandsStatesExt`, the reflection
  type data and `register_type_*state`, `state_exists`/`state_changed`, and
  `NextState::set_if_neq`/`reset` with the `PendingIfNeq` variant are gone (`bevy_platform` and
  `variadics_please` left the manifest). The frame lost the three systems and the sync point
  after the exit despawn; the `TransitionSchedules` sets stay. `bevy_time`: `common_conditions`,
  the render-world time channel (`TimeSender`/`TimeReceiver`, `create_time_channels`,
  `TrySendError`; `time_system` reads the clock directly, as it did with no sender), and
  `Fixed::from_seconds`/`discard_overstep`, `Time::advance_to`, `Timer::almost_finish`/
  `remaining_secs`, `Virtual::from_max_delta`/`set_relative_speed`/`effective_speed_f64`
  (`crossbeam-channel` left the manifest).
- **`bevy_transform` and `bevy_camera`, to what benilla and the kept crates call.**
  `bevy_transform`: `BuildChildrenTransformExt` (`commands.rs`), the no-std serial propagation
  (`std` is always on; the parallel one stays), `StaticTransformOptimizations::from_threshold`/
  `disabled` (benilla inserts `enabled()`; the threshold logic stays for the default), and
  `Transform`'s `from_matrix`, `from_isometry`, `to_isometry`, `looking_to`, `aligned_by`,
  `align`, `rotate_axis`, `rotate_x`, `rotate_z`, `rotate_local*`, `translate_around`,
  `rotate_around` with `GlobalTransform`'s `from_isometry`/`to_isometry`. `bevy_camera`:
  `VisibilityRange` and `VisibleEntityRanges` (no entity carries a range, so the range test in
  `check_visibility` and `bevy_light`'s two shadow-caster checks never culled; their reads go),
  `check_visibility_ranges` kept as an empty stand-in in `PostUpdate` with its plugin, set and
  order; `NoCpuCulling` (nothing inserts it), `MainPassResolutionOverride`,
  `Viewport::from_viewport_and_override`, `PhysicalCameraParameters` with
  `Exposure::from_physical_camera`, `SUNLIGHT`/`OVERCAST` and their EV100 constants, `Camera`'s
  `world_to_viewport_with_depth`, `viewport_to_world_2d`, `ndc_to_world`,
  `depth_ndc_to_view_z(_2d)`, `Projection::is_perspective`, `Visibility`'s three `toggle_*`,
  and `Aabb::is_in_half_space(_identity)`, `Frustum::contains_aabb(_identity)`/
  `intersects_obb_identity` and `face_index_to_name` with the tests of those. The frame is
  unchanged: the stand-in keeps `PostUpdate`'s section identical.
- **`bevy_diagnostic`, to what benilla and the kept crates call, and `bevy_asset`'s off
  features.** `bevy_diagnostic`: the plugins nothing adds (`EntityCountDiagnosticsPlugin`,
  `FrameTimeDiagnosticsPlugin`, `LogDiagnosticsPlugin`, `SystemInformationDiagnosticsPlugin`
  with its background task and systems), and `Diagnostic`'s `with_max_history_length`,
  `history_len`, `get_max_history_length`, `clear_history`, `value`, `duration`, `values` with
  their test. `SystemInfo` stays with its `Default`, which `DiagnosticsPlugin` still runs (the
  `sysinfo_plugin` feature is on) and which logs the OS, CPU and memory once at startup.
  avian3d's `diagnostic_ui` feature (off; its panel read `FrameTimeDiagnosticsPlugin`) went with
  its module. `bevy_asset`: the code of the features the build never enables, `file_watcher`,
  `embedded_watcher`, `watch`, `http`, `https`, `web_asset_cache` and `asset_processor` (the
  file and embedded watchers, the web reader); `AssetPlugin` reads `false` where it read those
  features, as it did, and the processor itself waits for its own trim. No system went.
- **`bevy_asset`'s processor.** `processor/` (`AssetProcessor`, `Process`,
  `LoadTransformAndSave`, the transaction log and its tests), `io/processor_gated.rs`,
  `saver.rs` and `transformer.rs`, with `AssetPlugin::use_asset_processor_override` (unset, it
  read `false`), `AssetApp::register_asset_processor`/`set_default_asset_processor`, the
  processor's handle provider in `init_asset`, `AssetSource`'s gating (`gate_on_processor`, the
  ungated reader, `should_process`, `iter_processed(_mut)`), the asset hashing only the processor
  ran, and `blake3`. `AssetMode::Processed` keeps its no-processor branch (it loads from the
  processed reader). The meta format stays: `AssetMeta<L>` reads a `Process` action with `()`
  settings, as `AssetMeta<L, ()>` did, and `ProcessedInfo` stays as data. `bevy_image`'s
  `compressed_image_saver` feature (off) went with its module. No system went.
- **`bevy_asset`'s writers, watchers and hot reload.** The writer side (`AssetWriter`,
  `ErasedAssetWriter`, `AssetWriterError`, `io::Writer`, `FileAssetWriter`,
  `MemoryAssetWriter`, the source builders' `writer`/`processed_writer` slots,
  `AssetSource::writer`/`processed_writer` with their missing-writer errors, and
  `AssetServer::write_default_loader_meta_file_for_path` with `WriteDefaultMetaError`); the
  watcher side (`AssetWatcher`, `AssetSourceEvent`, the builders' `watcher`/`processed_watcher`
  slots, `AssetSource`'s event receivers, `get_default_watcher`) with hot reload
  (`AssetServer::reload`, the reload block of `handle_internal_asset_events`, and the bookkeeping
  only it read: `loader_dependents`, `living_labeled_assets`, `should_reload`, the loaded asset's
  `loader_dependencies` and `populate_hashes`, which every load passed as `false`, with
  `ProcessedInfoMinimal`, `ReadAssetBytesError::MissingAssetHash` and `AssetMetaDyn`'s
  `serialize`/`processed_info`). A source asked to watch still logs its "no `AssetWatcher`"
  warning, and a watching server still prunes its finished load tasks each frame. Also gone:
  `load_folder` (`LoadedFolder` stays registered), `load_untyped_async`, `add_async` with
  `AddAsyncError`, `DirectAssetAccessExt`, the `embedded_asset!`, `load_embedded_asset!` and
  `load_internal_asset!` macros with `GetAssetServer` and the registry's inserts (the `embedded`
  source stays registered, empty; `embedded_path!` and `load_internal_binary_asset!` stay),
  `bevy_shader`'s `load_shader_library!`, and the `not(multi_threaded)` fallbacks
  (`sync_file_asset.rs`, the detached tasks): the `multi_threaded` feature is always on here, and
  without it the crate now builds the threaded path. The tests of removed code went with it. No
  system went.
- **`bevy_asset`'s uncalled loading API.** `AssetServer`'s `load_untyped` (with the untyped
  source suffix), the `load_acquire*` family, `load_with_settings_override`, the
  `wait_for_asset*` futures with `WaitForAssetError` (and the per-asset waker list nothing
  filled), the path and id handle lookups (`get_handle(s)(_untyped)`, `get_id_handle*`,
  `get_path_id(s)`, `get_path_and_type_id_handle`, `is_managed`), the loader lookups by
  extension, path and type (with `AssetLoaders::get_by_*`, their four tests, and the
  `MissingAssetLoaderForExtension`/`ForTypeId` errors nothing raised); `NestedLoader`'s dynamic
  and unknown typing, `with_settings`, `with_reader` and `deferred` (`LoadContext::load` and the
  kept tests use the static deferred and immediate loads); `LoadContext`'s
  `labeled_asset_scope`, `get_label_handle`, `has_labeled_asset` (`begin_labeled_asset` and
  `add_loaded_labeled_asset` stay private behind `add_labeled_asset`); `LoadedAsset`'s and
  `ErasedLoadedAsset`'s `get_labeled`/`iter_labels`; `AssetEvent`'s `is_modified`/`is_removed`/
  `is_unused`; `AssetPath::take_label`/`remove_label`; `Dir::insert_meta_text`. The accessors the
  kept tests read (`load_override`, the load-state queries) stay. No system went.
- **`bevy_image`, to the formats the build decodes and the API anything calls.** The formats
  whose features the build never enables (basis-universal, DDS, EXR, zlib and zstd-C
  supercompression, the `image` crate's BMP, farbfeld, GIF, ICO, JPEG, PNM, QOI, TGA, TIFF,
  WebP) and `serialize` (`SerializedImage`), with their features and dependencies; the HDR loader
  (`hdr` was on, but nothing loads a `.hdr`: `bevy`'s `hdr` feature stays a name enabling
  nothing). `ImageFormat` keeps `Ktx2` (the tonemapping LUTs) and `Png`; a lookup of any other
  extension or `image` format still warns that its feature is not enabled and yields no format,
  as it did; `to_mime_types`, `from_mime_type` and `ImageType::MimeType` went (nothing passed a
  MIME type). Also gone: `TextureAtlasBuilder` with `TextureAtlasSources` and `rectangle-pack`,
  `TextureAtlasLayout::from_grid`, `TextureAtlas::with_index`/`with_layout`, `Image`'s
  `get_color_at(_1d/_3d)`, `set_color_at_1d/_3d`, `resize_in_place` and `pixel_bytes`
  (`set_color_at` stays: bevy_egui calls it), `ImagePlugin::default_nearest`,
  `ImageSampler::get_or_init_descriptor`, `ImageSamplerDescriptor`'s `set_*` builders and
  `as_wgpu` with the conversions to and from wgpu's sampler types,
  `CompressedImageFormats::from_features`, `TEXTURE_ASSET_INDEX`/`SAMPLER_ASSET_INDEX`,
  `TextureError::InvalidImageMimeType`, and the tests of removed code. No system went.
- **`bevy_mesh`, to the shapes something meshes and the API anything calls.** Primitive meshing
  keeps `Rectangle`, `Cuboid`, `Plane3d` and `Sphere` (benilla, its `parity` example, bevy_pbr's
  fog and decal meshes, avian3d's `collider-from-mesh` tests); the other 2D and 3D builders
  (circle through ring, capsule, cone, conical frustum, cylinder, polylines, segments,
  tetrahedron, torus, triangles) and extrusion (`Extrudable`, `ExtrusionBuilder`,
  `PerimeterSegment`) went, with `PlaneMeshBuilder::new`/`from_size`/`from_length`/`normal`,
  `SphereMeshBuilder::new`/`kind` and `RectangleMeshBuilder::new`. `Mesh` keeps what is called:
  construction, attribute and index access, `count_vertices`, `compute_normals` (with the flat and
  angle-weighted smooth paths it takes), tangent generation and `rotated_by`; gone are the
  removers, the mutable index and attribute-map access, the vertex-buffer packing and layout
  (`get_mesh_vertex_buffer_layout`, `MeshVertexBufferLayouts`, `VertexBufferLayout`'s
  constructors), `duplicate_vertices`, winding inversion, area-weighted normals, `merge`, the
  transform/translate/scale family with `Transform * Mesh`, `normalize_joint_weights`,
  `triangles`, `take_gpu_data` (the render-world extraction, so a mesh's data is never in the
  extracted state), the morph-target accessors, `MorphTargetImage`, `MorphAttributes`,
  `MorphBuildError` and the weights' constructors (nothing builds morph targets; `MorphWeights`
  and `MeshMorphWeights` keep what bevy_animation and bevy_render read), `serialize`
  (`SerializedMesh`, `MeshDeserializer`; bevy_render's `serialize` stays a name enabling
  nothing), `triangle_area_normal`, and the tests of removed code. No system went.
- **`bevy_animation`, to the API benilla calls, and `bevy_animation_macros`.** Animation events
  (nothing adds one): `AnimationEvent` with its derive crate, `AnimationEventTrigger`,
  `AnimationClip`'s `add_event*` and events map, the per-tick event window (`TriggeredEvents`)
  with `ActiveAnimation`'s `last_seek_time`/`just_completed` that only it read, and the three
  event tests. `trigger_untargeted_animation_events` is an empty stand-in keeping its `Commands`
  (the sync point before `expire_completed_transitions`), and `animate_targets` keeps its
  unused `ParallelCommands` (the sync point after it). Also gone: `gltf_curves` (only bevy_gltf
  built them) with `interpolate_with_cubic_bezier`, the morph-weight curve `WeightsCurve` (the
  `bevy_mesh` feature stays for `animate_targets`' order against `InheritWeightSystems`), the
  graph's RON loader, `save` and serialized forms (nothing loads an `.animgraph.ron`), the
  blend-node constructors, `add_edge`/`remove_edge`/`get_mut`/`nodes`, the node mask setters,
  `curves_mut`, `curves_for_target_mut`, `add_variable_curve_to_target`, the uncalled
  `ActiveAnimation`/`AnimationPlayer` controls (`rewind`, `set_seek_time`,
  `is_playback_reversed`, `is_playing_animation`, the `*_all` family, `adjust_speeds`,
  `seek_all_by`) and `AnimationTargetId`'s serde derive. `ron` stays a dependency, unused, so
  its `std` feature stays on for bevy_asset. bevy_mesh's two `weights_mut` (the
  morph curve's) went with it. No system went.
- **`bevy_scene`, to what `ScenePlugin` runs and avian3d reads.** Nothing spawns or loads a
  scene. The plugin keeps both scene assets, `SceneSpawner`, the two `SpawnScene` systems and the
  `SceneRoot`/`DynamicSceneRoot` hooks; avian3d's `SceneRoot`, `SceneInstance`,
  `SceneInstanceReady` and `instance_is_ready` stay. Gone: the RON scene format (`serde`,
  `SceneLoader`, which `ScenePlugin` no longer registers, `DynamicScene::serialize`,
  `serialize_ron`), `DynamicSceneBuilder` and `SceneFilter` with `DynamicScene::from_world`/
  `from_scene`, `Scene::from_dynamic_scene`/`clone_with`, and `SceneSpawner`'s uncalled API
  (`spawn`, `spawn_dynamic`, `spawn_sync`, `spawn_dynamic_sync`, `despawn`, `despawn_dynamic`,
  `despawn_sync`, `despawn_dynamic_sync`, `iter_instance_entities`) with the despawn queues only
  `despawn`/`despawn_dynamic` filled and `despawn_queued_scenes`, which drained them. The tests of
  removed code went; the kept tests build their scenes by hand. The `serialize` feature stays,
  without `ron` and `serde`. No system went.
- **`bevy_gizmos`, to the lines benilla draws and benilla-gfx reads.** benilla draws with
  `Gizmos::line` and `linestrip` (the parity example adds `line_gradient`); benilla-gfx reads
  `GizmoConfigStore::get_config_dyn`, `GizmoHandles`, `GizmoAsset::buffer`, the line style and
  joint enums and the retained `Gizmo` component. The plugin keeps its asset, handles, the three
  config groups and every group's context systems (RunFixedMainLoop, FixedFirst, FixedLast,
  Last). Gone: every other drawing module (arcs, arrows, circles, cross, curves, grid, rounded
  box, the 2D and 3D primitives), the other `GizmoBuffer` methods (rays, loops,
  `linestrip_gradient`, rects, `cube`, `aabb_3d`, every `_2d` form, the `GizmoBufferView`), the
  global `gizmo()` with `GlobalGizmosPlugin` and its `flush_global_gizmos` system in Last,
  `insert_gizmo_config`, `GizmoConfigStore`'s mutable and iterating accessors and
  `GizmoMeshConfig` (the renderer's). **Stand-ins** in PostUpdate: `draw_aabbs`,
  `draw_all_aabbs`, `draw_lights`, `draw_all_lights` keep their plugin, place, run conditions and
  order and an unused `Gizmos` (a deferred buffer, like a `Commands`); `ShowAabbGizmo` and
  `ShowLightGizmo`, which only they read, went (nothing adds either, and each group's `draw_all`
  stays `false`). The `bevy_light` feature enables nothing now: the optional `bevy_light`
  dependency went with the light drawing. avian3d's `debug_render` module (`PhysicsDebugPlugin`,
  which nothing adds) went with the joints' `DebugRenderConstraint` impls; its `debug-plugin`
  feature stays, enabling `bevy_gizmos` and `bevy_render`.
- **`bevy_text`, to what `bevy_ui`'s text and benilla call.** benilla lays out `bevy_ui` text
  (its tests drive `TextPipeline::update_buffer`/`update_text_layout_info` through a
  `TextUiReader`) and benilla-gfx draws `TextLayoutInfo`'s glyphs. Gone: `TextWriter` (and
  `bevy_ui`'s `TextUiWriter`) with `TextSpanAccess::write_span` and `TextSpanComponent`,
  `TextReader`'s accessors but `iter`, `TextBackgroundColor`, `Strikethrough`, `Underline` and
  their colours (only the cut UI renderer read them), `TextLayoutInfo::run_geometry` with
  `RunGeometry` and the strikeout/underline metrics `update_text_layout_info` gathered only for it
  (its `scale_factor` argument stays, unread), the `FontFeatures` builder and list conversion,
  every `FontFeatureTag` and `FontWeight` constant but `NORMAL`/`DEFAULT`, `TextColor::BLACK`,
  the uncalled `TextLayout`/`TextFont` builders, `TextBounds::new_vertical`,
  `FontAtlasSet::has_glyph`/`FontAtlas::has_glyph`, `TextPipeline::get_font_id`. No system went.
- **`bevy_sprite`, to `Mesh2d` bounds, 9-slice data and the `Text2d` frame.** Nothing spawns a
  `Sprite` or a `Text2d`; benilla-gfx builds `TextureSlicer`s by their fields and reads
  `BorderRect`/`SliceScaleMode`. Gone: `Sprite`, `SpriteImageMode`, `SpriteScalingMode`,
  `Anchor`, `TextureSlice` with the slicer's slice computation, `Text2dShadow`, the
  `Text2dReader`/`Text2dWriter` aliases, `bevy_sprite::SpriteSystems` (unused), the sprite half of
  `calculate_bounds_2d` (it keeps the `Mesh2d` half) and the tests of removed code. `Text2d`
  stays a bare component: it names `detect_text_needs_rerender::<Text2d>`, which stays whole.
  **Stand-ins** in PostUpdate: `update_text2d_layout`, `calculate_bounds_text2d` (keeps its
  `Commands`), and `bevy_sprite_render`'s `compute_slices_on_asset_event` (keeps its `Commands`)
  and `compute_slices_on_sprite_change`, with their plugins, places, sets and order;
  `ComputedTextureSlices` and `Sprite`'s `SyncToRenderWorld` requirement went with them. The
  manifest drops seven dependencies (the `bevy_text` feature no longer enables `bevy_window`);
  `bevy_sprite_render` keeps its `bevy_sprite` edge, which its `bevy_text` feature names.
- **`bevy_ui`, its API to what benilla, benilla-gfx and the kept code call** (the first half;
  its systems stay). benilla spawns `Node`s, `ImageNode`s and `Text`, reads `Interaction`,
  `ComputedNode`, `UiGlobalTransform` and `ScrollPosition`, and never uses grid layout, border
  radii, gradients, shadows or viewports. Gone: `gradients.rs` (every gradient type) with
  `UiPosition`, which only it read; `BoxShadow` and `ShadowStyle` (only the cut renderer read
  them); `ViewportNode`; `RelativeCursorPosition` (`ui_focus_system` no longer fills it: nothing
  adds one); `Val`'s string parsing (`FromStr`, `ValParseError`) and the `auto`/`px`/`percent`/
  `vw`/`vh`/`vmin`/`vmax` helpers with `ValNum`; `UiRect`'s `with_*` setters; `ComputedNode`'s
  `border_box`/`padding_box`/`content_box` and scrollbar geometry; `Overflow`'s `clip_x`,
  `clip_y`, `hidden_x`, `hidden_y`; `OverflowClipMargin`'s constructors; the grid builders no kept
  test calls (`GridTrack`'s `minmax` and viewport units, `RepeatedGridTrack`'s but `px` and
  `min_content`, `GridPlacement`'s but `auto`, `start`, `span` and the getters); `BorderRadius`'s
  per-corner and per-side constructors and setters; `ImageNode`'s builders but `new`; the
  uncalled `UiTransform`, `UiGlobalTransform` and `Val2` constructors; `layout::debug`
  (`print_ui_layout_tree`); the off `ghost_nodes` feature with its half of
  `experimental::ghost_hierarchy` (`GhostNode`, the ghost-aware `UiChildren`), and the
  non-ghost `UiChildren`'s uncalled `get_parent` and `is_ui_node`. **Stand-in** in PostUpdate:
  `update_viewport_render_target_size`, with its place, sets and ambiguities.
- **`bevy_ui`'s grid layout API.** No `Node` is a grid container: `Display::Grid`, `Node`'s
  `justify_items`, `justify_self` and seven `grid_*` fields, `JustifyItems`, `JustifySelf`,
  `GridAutoFlow`, `GridTrack`, `RepeatedGridTrack`, `GridTrackRepetition`, `GridPlacement` with
  `GridPlacementError`, the track sizing functions and their taffy conversions went, and so did
  the `derive_more` dependency. **taffy's `grid` feature stays**: each UI root is the one item of
  an implicit taffy grid viewport (`ui_surface.rs`), so the grid algorithm still lays out every
  root. `from_node` sets the root's `grid_row`/`grid_column` to the `span 1` that
  `GridPlacement`'s default converted to (taffy's own default is `auto`); the other removed
  fields converted to taffy's defaults. `ui_rounding_test` lays its two children out in a flex
  row instead of a two-column grid.
- **`bevy_sprite_render`'s tilemap chunk.** Nothing spawns a `TilemapChunk`. Gone:
  `TilemapChunk` with its insert hook, `TilemapChunkTileData`, `TileData`, `PackedTileData`,
  `make_chunk_tile_data_image`, `TilemapChunkMeshCache`. `TilemapChunkMaterial` and its
  `Material2dPlugin` stay (the frame names its asset systems). **Stand-in** in Update:
  `update_tilemap_chunk_indices`. The manifest drops `bevy_platform`, `bevy_transform` and
  `tracing`; `bytemuck` stays, unused, because dropping it turns its `must_cast` feature off for
  the whole build.
- **avian3d's off features.** The build enables `3d`, `f32`, `parry-f32`, `default-collider`,
  `collider-from-mesh`, `bevy_scene`, `debug-plugin`, `parallel` and `xpbd_joints`; the code the
  others gated is gone: the 2D half of every dimension split (`primitives2d.rs`, the 2D mass
  properties, rotations, joints and shape casts), `f64` (`math/double.rs`), `serialize` (every
  serde derive and reflected `Serialize`/`Deserialize`), `bevy_diagnostic`
  (`PhysicsDiagnosticsPlugin`, `entity_counters.rs`, `total.rs`, `write_diagnostics`),
  `validate`, `enhanced-determinism`, `simd`, and the 2D determinism test. The attributes of the
  enabled features are gone too: a `cfg` that always holds is dropped and a `cfg_attr` that always
  holds is its attribute (a doc string a `///` line); a tail block such a `cfg` gated lost its
  braces. The manifest drops the seven features with the optional `libm`, `parry3d-f64` and
  `serde`. `rustc -Zunpretty=expanded` of the crate is token-identical to the copy's apart from
  docs, `tracing`'s line numbers, the unwrapped braces and one derive order.
- **avian3d's uncalled functions.** With every `pub fn` made `pub(crate)` and only the ones
  benilla's crates (all targets) then failed to reach put back, rustc's dead-code lint named the
  functions nothing calls; those 453 are gone (the builders and constructors of joints, motors,
  mass properties, materials, layers, casters and AABBs, `SpatialQuery`'s point projection and
  shape/point intersections, `MoveAndSlide::move_and_slide` with depenetration, the contact
  graph's and contact types' queries, `TrimeshBuilder`'s API, the graph and collection helpers,
  `PhysicsPlugins::with_collision_hooks`/`with_length_unit`, the interpolation plugin's
  `interpolate_all`/`extrapolate_all` family, `ForcesItem::non_waking`), with the `Collisions`
  system parameter (no method left), `UpdatePhysicsTransformError`, five feature-id masks and two
  aliases. Kept for avian3d's own tests of kept code: the constructors and builders they call
  (`RevoluteJoint`/`PrismaticJoint::new` and four builders, the motors' and limits' `new`,
  `AngularInertia`/`ComputedAngularInertia`/`ComputedMass` `new`/`try_new`,
  `MassPropertiesBundle::from_shape`, `Restitution::new`/`with_combine_rule`, `Position::new`,
  `LayerMask::has_all`, `StableVec`/`IdPool::clear`, `project_velocity_bruteforce`); the tests of
  `TrimeshBuilder` and `ColliderConstructorHierarchy` (which import the cut `bevy::winit` and
  `bevy::gltf`) went. No system or type benilla's frame registers changed.
- **avian3d's unused types.** The same method over `pub struct`/`enum`/`trait`/`type`/`const`
  (every item made `pub(crate)`, an explicit `pub use` of one split into a `pub(crate) use`
  while the analysis ran, and put back where benilla's crates, all targets, needed it by name, by
  a trait method or by type privacy) named what nothing uses; gone: the `Forces` query data with
  `ForcesItem`/`NonWakingForcesItem`, the `RigidBodyForces` traits and their tests (a user API
  for one-time forces; the constant-force components and `ForcePlugin` stay),
  `PhysicsTransformHelper`, `PhysicsPluginsWithHooks`, `trimesh_builder.rs` (`TrimeshBuilder`,
  `Trimesh`, its error), `TimeOfImpact`/`TimeOfImpactStatus`, `SingleContact`, `UnGraph`'s
  iterators and edge references with `next_edge`, `StableUnGraph`'s `EdgeMut`, the deprecated
  aliases (`PhysicsSet`, `PhysicsStepSet`, `SolverSet`, `SubstepSolverSet`, `NarrowPhaseSet`,
  `SweptCcdSet`, `IntegrationSet`, `PhysicsTransformSet`, `OnCollisionStart`/`End`,
  `SleepingThreshold`, `TimeSleeping`, `DeactivationTime`), `Matrix2`, `FRAC_PI_2`,
  `FRAC_1_SQRT_2`, the preset constants nothing reads (`ZERO`, `MAX`, `LINEAR`, the locked-axes
  and restitution presets, `CollisionLayers::NONE`/`ALL_*`, `ProxyId::PLACEHOLDER`,
  `DEFAULT_TWIST_AXIS`), the diagnostic paths (`path_macro.rs`, `timer_paths`/`counter_paths`,
  `PhysicsDiagnosticsSystems::WriteDiagnostics`: only the cut `bevy_diagnostic` writer read
  them), and the methods of now-private types nothing calls (`PhysicsTime`'s speed setters and
  `pause`/`unpause`, four `Bvh2Ext` traversals, `distance_to_point_squared`, `scale_by`, the
  XPBD constraints' `apply_angular_correction`/`compute_torque`/`compute_force`,
  `VelocityIntegrationData::apply_*_acceleration`, `LockedAxes::apply_to_angular_inertia`,
  `ComputedAngularInertia::inverse_mut`/`inverse_tensor_mut`). Kept for avian3d's own tests:
  `Gravity::ZERO`, `MassPropertiesBundle` with `MassPropertiesExt` and the error enums of the kept
  `try_new`s. No system, set or plugin benilla's frame names changed.
- **avian3d's casters and collider constructors.** benilla spawns no `RayCaster`, `ShapeCaster`,
  `ColliderConstructor` or `ColliderConstructorHierarchy`, so the four caster systems in
  `PhysicsSchedule` and the two constructor systems in `Update` found nothing each frame. Both
  schedules run single-threaded (avian3d sets `PhysicsSchedule`'s executor itself), so the six stay
  as empty stand-ins at their places, the two in `Update` keeping their `Commands`. Gone: the
  components with `RayHits`/`ShapeHits`, their hooks and position queries, `constructor.rs` with the
  two ready events and the hierarchy config, `Collider::try_from_constructor`, the collider cache
  (`ColliderCachePlugin` stays, empty; its `PreUpdate` system went), the caster timers of
  `SpatialQueryDiagnostics` (the resource and its reset system stay), `warn_invalid_mass`'s
  `Without` filter on the two constructors. The A4af pass over the result then named what only
  these reached: twenty `Collider` constructors (`compound`, `cylinder`, `heightfield`, the
  convex-decomposition, voxel and mesh-built shapes), `SpatialQuery`'s `ray_hits`, `cast_shape`,
  `shape_hits` and their callbacks, `ColliderTree`'s `ray_traverse_all`/`sweep_traverse_all`, with
  `VhacdParameters`, `FillMode`, `TrimeshFlags` and the `IVector` alias.
- **avian3d's unread fields.** The diagnostics resources' timers and counters were only written
  (the `bevy_diagnostic` writer that read them went with the feature), so `CollisionDiagnostics`,
  `SolverDiagnostics` and `ColliderTreeDiagnostics` are fieldless like `SpatialQueryDiagnostics`,
  each with its reset system in `PhysicsDiagnosticsSystems::Reset`, and the nineteen systems that
  timed into them lose their `ResMut` and their `Instant` reads (the systems stay, in their sets).
  `PhysicsInterpolationPlugin` loses its four private `*_all` flags, which only `Default` set,
  always to `false`, with the four branches of `build` they guarded. `AabbContext` and
  `ContactManifoldContext` lose their entities, which the one `AnyCollider` (parry's `Collider`)
  never reads; their doc example reads `Time` only, and `update_moved_collider_aabbs`'s collider
  query no longer fetches the `Entity` it passed.
- **avian3d's per-body and per-joint options.** Components only a user inserts, which nothing in
  benilla (or avian3d's kept tests) inserts: the eight constant forces and accelerations,
  `GravityScale`, `LinearDamping`, `AngularDamping`, `MaxLinearSpeed`, `MaxAngularSpeed`,
  `Dominance`, `CustomVelocityIntegration`, `CustomPositionIntegration`, `SleepingDisabled`,
  `JointDisabled`, `JointCollisionDisabled` (with its hooks and its contact-removing observer),
  `JointDamping` and `JointForces`. Each read becomes its absent result: the integrator damps with
  a coefficient of `0.0` and adds gravity unscaled (`x * 1.0` is `x`), the solver body gets
  dominance `0`, a joint enters the graph with collisions enabled, and the `With`/`Without`
  filters on the markers go. The systems that only served them stay as empty stand-ins in their
  sets (avian3d runs `PhysicsSchedule` and `SubstepSchedule` single-threaded): the eight
  `apply_constant_*` systems, `clamp_velocities`, `joint_damping::<T>` and
  `writeback_joint_forces::<T>`. `wake_on_changed` loses its query of changed constant forces and
  gravity scales; the observers that re-added a joint when `JointDisabled` was removed go, and
  the one removing a joint on `Disabled` no longer also fires on `JointDisabled`.
- **avian3d's collision and material options.** The remaining components only a user inserts:
  `CollisionEventsEnabled` (with the `collision_events` module, `CollisionStart`/`CollisionEnd`
  and their message registration, the `CONTACT_EVENTS` proxy and contact-edge flags and every
  event write), `CollidingEntities`, `CollisionMargin`, `SpeculativeMargin`, `SweptCcd` (with
  `SweepMode` and the swept time-of-impact code), `NoAutoMass`, `NoAutoAngularInertia`,
  `NoAutoCenterOfMass` (with their removal hook); `Friction` and `Restitution` stay as plain
  types (the default-material resources hold them) but are no longer components, and `LockedAxes`
  went with the locking (below). Each read becomes its absent result: contacts combine the default friction
  and restitution with themselves, the collision margin sum is `0.0` (still added to the
  penetration, which turns a `-0.0` into `0.0`), every speculative margin is the configured
  default, the AABB growth is the contact tolerance alone (positive, so adding `0.0` was exact),
  no axis is locked (the integrator's locking was an identity), and the mass helper always writes
  the computed totals. `trigger_collision_events` (kept exclusive) and `solve_swept_ccd` stay as
  empty stand-ins in their sets. Kept, because something on benilla's path or a kept avian3d test
  inserts or observes them: `Sensor`, `RigidBodyDisabled`, `ActiveCollisionHooks`, `Mass`,
  `AngularInertia`, `CenterOfMass`.
- **avian3d's dead-code leftovers.** What the two option cuts above left with no reader, and what
  the uncalled-function pass then named: axis locking (`LockedAxes`, the lock bits of
  `SolverBodyFlags` and `InertiaFlags` and every branch reading them, which no body set:
  `effective_inv_mass` is the plain inverse mass, the angular inertia is never masked, forces are
  not projected, and gyroscopic motion depends only on the inertia's isotropy);
  `SolverBodyInertia::new`'s `dominance` parameter (always `0`; static and kinematic bodies still
  get `128`); `JointGraphEdge::collision_disabled` (always `false`) with the broad phase's joint
  check and its `JointGraph` parameter; the XPBD solver data's `total_*_lagrange` accessors with
  the totals only they read (the motor totals stay: warm starting reads them);
  `ContactGraph::entities_colliding_with`, `JointGraph::get_mut`/`bodies_of`/`joints_between`,
  `StableUnGraph`'s `edge_count`, `edge_endpoints`, `neighbors`, `edges` and `edges_between` with
  the `Neighbors`, `Edges`, `EdgesBetween` and `EdgeReference` iterators, `Edge::source`/`target`;
  `Friction`'s and `Restitution`'s `From<Scalar>`; the unused `ClosestPoints` and
  `PointProjection`.
- **Filling the type registry.** Nothing on benilla's path reads `AppTypeRegistry` (its readers
  are scene spawning, entity cloning, `World`'s and `EntityCommands`' reflect accessors and
  `ReflectAsset`/`ReflectHandle`, none of which runs), so what only filled it goes: automatic
  registration (`bevy_reflect`'s `auto_register`, `auto_register_inventory` and
  `auto_register_static` features with the `inventory` dependency, the derive's per-type
  `inventory::submit!` and static export with its `uuid` dependency, `#[reflect(no_auto_register)]`,
  `load_type_registrations!`, `TypeRegistry::register_derived_types`,
  `AppTypeRegistry::new_with_derived_types` and the hot-patch system that re-ran it; `bevy_app`'s
  and `bevy_ecs`'s `reflect_auto_register` features; `bevy`'s and `bevy_internal`'s
  `reflect_auto_register` stay as empty names, since the workspace manifest names it), and every
  plugin's `register_type`, `register_type_data` and `register_asset_reflect` call (`bevy_time`,
  `bevy_asset`'s `init_asset`, `bevy_mesh`, `bevy_animation`, `bevy_pbr`, `bevy_render`'s storage,
  `bevy_sprite_render`, `bevy_ui_render`, avian3d, bevy_egui, `bevy_transform_interpolation`).
  `App::default` inserts the registry as stock does without the feature: the primitive types and `String`. The
  registration API stays; the derives stay for now.
- **The plugins' `Reflect` derives** (avian3d, `bevy_transform_interpolation`, `bevy_heavy`,
  bevy_egui). With the registry unfilled, a derive is read only through a bound, and none of
  these types meets one: every `derive(Reflect)` and `#[reflect(..)]` goes (avian3d 114 and 103,
  `bevy_transform_interpolation` 19 and 19, bevy_egui 3). `AncestorMarkerPlugin<C>` loses its
  `C: TypePath` bound, which only its marker's derive needed, and `SolverBodyInertia` its
  `InertiaFlags`, which only reflection read. `bevy_heavy`'s off features go the avian3d way
  (`bevy_reflect`, which avian3d's defaults turned on, `serialize`, `libm`, `nostd-libm`; `approx`
  stays for its tests), as does `bevy_transform_interpolation`'s `serialize` with its `serde`
  dependency; avian3d stops asking `glam_matrix_extras` for `bevy_reflect` and bevy_egui drops its
  `bevy_reflect` dependency.
- **The Bevy crates' `Reflect` derives, first part** (`bevy_ui`, `bevy_text`, `bevy_light`,
  `bevy_pbr`, `bevy_core_pipeline`, `bevy_sprite`, `bevy_sprite_render`, `bevy_ui_render`,
  `bevy_gizmos`, `bevy_scene`, `bevy_camera`, `bevy_render`, `bevy_mesh`, `bevy_image`,
  `bevy_animation`, `bevy_asset`, `bevy_window`): 289 derives and 359 `#[reflect(..)]` attributes go.
  What a bound reads keeps its derive: the gizmo config groups (`GizmoConfigGroup: Reflect`) with
  `LightGizmoColor`, `AnimatableCurve` and `AnimatableKeyframeCurve` (the animation curve bounds),
  and the types of the kept scene tests. An asset that lost its derive gets `#[derive(TypePath)]`
  (`StandardMaterial`, `ColorMaterial`, `Mesh`, `AnimationClip`, `AnimationGraph`,
  `ShaderStorageBuffer`). `bevy_image`'s and `bevy_window`'s own `bevy_reflect` features go by cfg
  resolution, with `bevy_window`'s dependency and `bevy_internal`'s request for the feature;
  `bevy_image`'s took the last two `register_asset_reflect` calls (`Image`, `TextureAtlasLayout`)
  with it. With nothing calling it, `register_asset_reflect` goes, and so do `ReflectAsset` and
  `ReflectHandle` (`reflect.rs`) and `Handle`'s reflection test, `ActiveAnimation`'s
  `DynamicMap` test, and `bevy_ui`'s reflected `Serialize`/`Deserialize` under its off `serialize`
  feature. Two items only reflection kept alive go: `MeshExtractableData::ExtractedToRenderWorld`
  (only `FromReflect` could build it since the render world's extraction went) with
  `MeshAccessError::ExtractedToRenderWorld`, and `MorphWeights::first_mesh` (always `None`).
  `bevy_ui`, `bevy_light`, `bevy_core_pipeline`, `bevy_sprite`, `bevy_ui_render` and
  `bevy_camera` drop their `bevy_reflect` dependency, and `bevy_reflect` loses its `wgpu-types`
  feature, which only `bevy_camera` asked for.
- **The Bevy crates' `Reflect` derives, second part.** `bevy_input`'s, `bevy_time`'s,
  `bevy_state`'s and `bevy_color`'s own `bevy_reflect` features go by cfg resolution, with their
  `bevy_reflect` dependencies and `bevy_internal`'s requests for them; `bevy_winit` drops its one
  derive and its dependency. The colours' reflection kept two things alive: `bevy_animation`'s
  `Animatable` impls for `LinearRgba`, `Laba`, `Oklaba`, `Srgba` and `Xyza` (`Animatable: Reflect`;
  benilla animates only `Vec3` and `Quat`) go, with `bevy_animation`'s `bevy_color` dependency, and
  the gizmo config groups, which were stored as `Box<dyn Reflect>` only to be downcast through
  `as_any`: `GizmoConfigStore` now stores `Box<dyn Any + Send + Sync>` and downcasts the same way,
  `GizmoConfigGroup` (and its derive's bound) asks for `TypePath + Default + Send + Sync`, and the
  four groups derive `TypePath` in place of `Reflect` (`T::type_path()` still names a missing group
  in the panic); `LightGizmoColor` loses its derive. `bevy_math`'s, `bevy_app`'s and
  `bevy_transform`'s features stay (`Transform`'s derive, whose `type_info()` `AnimatedField`
  reads, needs `Vec3`'s and `Quat`'s reflection, `AnimatableKeyframeCurve`'s needs `UnevenCore`'s,
  and `bevy_app`'s inserts `AppTypeRegistry`), but every other derive in them goes as the off
  feature would remove it: `bevy_math`'s primitives, directions, rects, bounding volumes, rays,
  isometries, splines, easing and curve types, with the curve adaptors' hand-written `TypePath`
  impls and the tests that only proved they compile, `GlobalTransform`,
  `StaticTransformOptimizations`, and `bevy_app`'s `Propagate`, `PropagateOver`, `PropagateStop`
  and `Inherited`.
- **`bevy_reflect`'s off features and unreached impls.** The features the build never enabled go
  by cfg resolution with their code: `functions` (the `func` module, the derive's `func` impls,
  `bevy_app`'s and `bevy_ecs`'s `reflect_functions` with `register_function*` and
  `AppFunctionRegistry`), `reflect_documentation` (the derive's doc capture and `TypeInfo`'s
  docs), `hashbrown`, `critical-section` (`bevy_internal`'s request goes) and `wgpu-types`, with
  the `hashbrown` and `wgpu-types` dependencies and the `reflect_docs` example entry. The on
  features no kept derive reaches go with their requests and dependencies: `petgraph`
  (`bevy_animation` asked), `smol_str` (`bevy_text`), `uuid` (`bevy_asset`), which turns off
  `petgraph`'s `serde-1` and `smol_str`'s `serde` for the build (serde impls nothing names).
  `smallvec` and `indexmap` leave `bevy_reflect`'s defaults and `bevy_internal`'s request but stay
  on: `bevy_ecs`'s own derives (`DefaultQueryFilters`, `EntityIndexSet`) still need them. (Those two derives went with
  bevy_ecs's reflection, below.)
- **`bevy_reflect`'s serializers and path access**, which nothing outside the crate calls
  (`bevy_scene`'s RON format, their one user, went earlier): the reflection (de)serializers
  (`serde::ser` and `serde::de` with `ReflectSerializer`, `TypedReflectSerializer`,
  `ReflectDeserializer`, `TypedReflectDeserializer`, their processors, the
  `SerializeWithRegistry`/`DeserializeWithRegistry` traits and type data, and the
  `debug_stack` type stack only their errors printed; the feature stays, enabling only `std`),
  and the `path` module (`GetPath`, `ParsedPath`, `ReflectPath` and the access errors, with
  their prelude entries). Kept: the `ReflectSerialize`/`ReflectDeserialize` type data with
  `Serializable` and `SerializationData`, which the derive's and the opaque impls' output name.
  The tests of the removed code and the dev-dependencies only they used (`ron`, `bincode`,
  `rmp-serde`, `serde_json`, `serde`'s `derive`) go with it.
- **`bevy_reflect`'s remote reflection, custom attributes and unreached foreign impls**, which
  nothing outside the crate uses: `ReflectRemote` with the derive's `#[reflect_remote]` macro,
  its `remote = ..` field attribute, the compile-time assertions only remote fields produced and
  every remote-wrapper branch of the derive's output; the `#[reflect(@..)]` custom attributes
  (`attributes.rs` with `CustomAttributes` and the `custom_attributes`/`get_attribute`/
  `has_attribute` methods on the struct, tuple-struct, enum, variant and field infos; a variant's
  `#[reflect(..)]` attributes are still parsed, so bad input is still rejected); and the reflection
  impls of `TypeId`, `BuildHasherDefault`, `SocketAddr`, the ranges, `Result`, the atomics,
  `Duration`, `BinaryHeap`, `BTreeMap`/`BTreeSet`, `VecDeque`, bevy_platform's `Arc` and
  `Instant`, foldhash's hashers (with the `foldhash` dependency), `OsString`, `Path`/`PathBuf`
  and std's `HashMap`/`HashSet`/`RandomState`, which no kept derive or call reaches (the
  registry's defaults register only the primitives and `String`). Two kept tests move from std's
  `HashSet` and `RandomState` to bevy_platform's `HashSet` and `FixedHasher`. Every kept derive
  expands to the same tokens.
- **`bevy_reflect`'s uncalled methods**, which no other crate, derive output or kept test of
  bevy_reflect calls: `TypeRegistry`'s `register_by_val`, `add_registration`,
  `overwrite_registration`, the short-path lookups with their ambiguity bookkeeping
  (`short_path_to_id`, `ambiguous_names`, `get_with_short_type_path(_mut)`, `is_ambiguous`),
  `get_with_type_path_mut`, `get_type_data_mut`, `get_type_info`, `iter_mut`, `iter_with_data`;
  `TypeRegistration`'s `data_by_id`, `data_mut(_by_id)`, `contains(_by_id)`, `len`, `is_empty`;
  `ReflectFromPtr::from_ptr(_mut)`; the infos' `field_names`/`variant_names` (with the fields
  holding them), `iter`, `field_len`, `index_of`, `variant_path`, `contains_variant`,
  `UnnamedField::index` (the field; `new` still takes the index), `SetInfo::value_ty` (the
  field), `Type`'s `short_path`/`ident`/`crate_name`/`module_path`, `Generics::with`; the
  dynamic types' `DynamicArray::new`, `DynamicEnum::set_variant_with_index`/`variant`/
  `variant_mut`, `DynamicList`'s and `DynamicTuple`'s `set_represented_type`,
  `TupleFieldIter::new`, `dyn PartialReflect::represents`; the kind casts nobody makes
  (`ReflectRef::as_opaque`, `ReflectMut`'s casts but `as_map`/`as_set`, `ReflectOwned`'s but
  `into_struct`/`into_enum`, `VariantInfo::as_struct_variant`/`as_unit_variant`). Kept: what
  the derive's output can name (`SerializationData`/`SkippedField` for `skip_serializing`
  fields, `StructVariantInfo::new`, the `set_represented_type`s it calls),
  `ReflectSerialize`/`ReflectDeserialize` whole, the methods the info macros generate, and what
  bevy_reflect's own tests call. Every kept derive expands to the same tokens.
- **`bevy_reflect`'s serde type data and its `smallvec` and `indexmap` impls.** Since the
  serializers left, nothing reads `ReflectSerialize` or `ReflectDeserialize`: both go with
  `Serializable`, their prelude entries and the `Serialize`/`Deserialize` registrations of the
  opaque and glam impls (the registry's defaults no longer hold them for the primitives and
  `String`). `SerializationData` and `SkippedField` go with the derive's
  `#[reflect(skip_serializing)]` field attribute, the only thing that built them (no input in the
  build uses it; a use no longer compiles). The `serde` module goes whole, with `pub use
  erased_serde` and the `serde` and `erased-serde` dependencies (serde stays in the build through
  other crates, its resolved features unchanged). The `smallvec` and `indexmap` features go with
  their impls (`SmallVec` as a list, `IndexMap`/`IndexSet` as a map and a set) and `bevy_ecs`'s
  request for them: the bevy_ecs derives that needed them went with its reflection, and no kept
  derive reflects either type. Every kept derive expands to the same tokens.
- **`bevy_reflect`'s `Map` and `Set` kinds.** Nothing in the build reflects a map or a set: the
  only implementors were bevy_platform's `HashMap`/`HashSet` (with their `TypePath` impls and
  those of the three hashers), and no kept derive has a field of either (the workspace checks
  without them). The impls go with their two macros, and then the kinds whole: `Map`/`Set`,
  `DynamicMap`/`DynamicSet`, `MapInfo`/`SetInfo`, their iterators and the `map_*`/`set_*`
  helpers (`map.rs`, `set.rs`), the `Map`/`Set` variants of `ReflectKind`, `ReflectRef`,
  `ReflectMut`, `ReflectOwned` and `TypeInfo` with every match arm on them (only bevy_reflect
  matches on them; bevy_animation matches `Struct` and `TupleStruct`), `ReflectRef::as_map`/
  `as_set`, `ReflectMut`'s two casts (all it had) and `TypeInfo::as_map`. bevy_reflect's own
  tests lose their map parts and keep their other assertions (`reflect_complex_patch` keeps its
  tuple-of-one `g` field); `reflect_map*` and `kind.rs`' `should_cast_mut` go whole. The default
  registry never held a map, so no registry changes.
- **`bevy_reflect`'s test-only methods and the type info nothing reads.** Every `pub fn` of
  bevy_reflect was marked `#[deprecated]` on HEAD and four builds checked (the workspace with
  `--all-targets`, `benilla` with `trace_chrome`, and the tests of 32 vendored crates with and
  without it); 70 methods were reached only from bevy_reflect's own tests, and two from nowhere.
  The lint does not fire inside derive output in other crates, so a removed method the derive
  emits fails to compile instead (none re-resolves: the derive names `Any::type_id` fully
  qualified, `DynamicEnum` has no `From` impl, and no trait gives the infos an `is`, `type_path`
  or `variant`); the four `impl_reflect!` emits (`TupleStructInfo::new`, `StructVariantInfo::new`,
  `TupleStructFieldIter::new`, `DynamicTupleStruct::set_represented_type`) stay or went with
  their type. Gone: the info macro's `type_id`, `type_path`, `type_path_table` and `is` (every
  info keeps `ty`), the infos' field and variant accessors (`field`, `field_at`, `variant*`,
  `item_*`, `capacity`, `get_named`, `is_const`, the param infos' `default` and `with_default`
  with the derive's emission of generic defaults), `TypeInfo::is`/`type_path_table`/`as_opaque`,
  `TypePathTable`'s four path parts, `ReflectOwned::into_struct`/`into_enum`,
  `ReflectFromPtr::as_reflect`/`type_id`, `TypeRegistry::contains`/`iter`/`register_type_data`/
  `get_with_type_path`, `TypeRegistration::iter`/`iter_mut`, and the dynamic types' typed
  conveniences (`DynamicStruct`/`DynamicTuple`/`DynamicTupleStruct::insert`, `DynamicList::push`,
  `DynamicEnum::new`/`from`, `DynamicArray::set_represented_type`). What only those read goes
  too: `EnumInfo`'s variants with `VariantInfo`, the three variant infos and `VariantInfoError`
  (the derive no longer builds them: `EnumInfo::new` takes no variants), `ArrayInfo`'s and
  `ListInfo`'s item info and capacity, `TupleInfo`'s and `StructInfo`'s field lists (`StructInfo`
  keeps its name index, which `index_of` reads), the field infos' `type_info`, and
  `ReflectFromPtr`'s `type_id` and `from_ptr`. The tests that exercised the removed accessors go
  (`reflect_type_info`, `should_get_enum_type_info`, `option_should_impl_typed`, ...); the ones
  that built patches with the typed conveniences use the boxed forms they forwarded to
  (`insert_boxed(name, Box::new(v))`, `push_box`, `DynamicEnum::new_with_index(0, ..)`,
  `from_ref`) and keep their assertions. Nothing on benilla's path read any of it; a type info's
  `Debug` output is shorter.
- **`bevy_ecs`'s and `bevy_app`'s off features and platform code.** The features nothing in the
  build can enable go with their code: `bevy_debug_stepping` (the `Stepping` resource and module,
  `bevy_app`'s `Stepping::begin_frame` system, the executors' skip list, so
  `SystemExecutor::run` loses its always-`None` `skip_systems` parameter, and `Schedule::executable`,
  which only stepping read), `hotpatching` (`HotPatched`, `HotPatchChanges`,
  `System::refresh_hotpatch` and every override, the function systems' jump-table pointer,
  `bevy_app`'s `hotpatch.rs` with `dioxus-devtools`, `subsecond` and the optional
  `crossbeam-channel`), `track_location` (`MaybeLocation` is always the empty form: its methods
  return `None` and drop their closures uncalled, as they did) and `detailed_trace` (three
  message trace lines). The features the workspace always enables are resolved: `std`,
  `multi_threaded` and `async_executor` (the `no_std` fallbacks, the single-threaded `par_iter`
  folds and message iteration, `ExecutorKind::SingleThreaded` as a default; `MultiThreaded` stays
  the default), and so are the `wasm32` branches and the no-atomics `Box` and `concurrent-queue`
  target dependency. The features stay as names (`std`, `multi_threaded`, `async_executor` enable
  what they did); `trace` stays gated (benilla's `tracy` and `trace_chrome` reach it), as do
  `backtrace`, `debug`, `serialize` and `bevy_reflect`. Resolving the `cfg`s leaves both crates
  expanding to the same tokens, with and without `trace`; the stepping module, its parameter and
  accessor went after that proof.
- **`bevy_ecs`'s uncalled functions, first part: the entity collections, the commands and the
  hierarchy.** Found with every `pub fn` of `bevy_ecs` and `bevy_app` made crate-private and put
  back where the workspace (with and without `trace`) or the tests of `bevy_ecs`, `bevy_app` and
  every other vendored crate whose tests compile still reach it; a function goes only when
  rustc's dead-code lint names it in every one of those builds. Gone: the slice- and `Vec`-like
  API of `UniqueEntityEquivalentSlice`, `UniqueEntityEquivalentVec` and
  `UniqueEntityEquivalentArray` that nothing calls (the splitting, chunking, sorting, raw-pointer,
  `Arc`/`Rc`/`Box` and capacity methods, `get`/`get_mut`, `push`/`pop`/`insert`/`remove`,
  `drain`/`splice` and the iterators' `as_slice`s), `EntityIndexMap`'s and `EntityIndexSet`'s
  constructors, range and slice accessors and their slices' and iterators' unwrapping methods;
  `Commands`' batch spawns and inserts, `register_system`/`unregister_system`, `run_schedule`,
  `trigger_with`, `add_observer`, `get_spawned_entity`, and `EntityCommands`' by-id inserts,
  `try_insert_if`, `remove_if`, `retain`, `log_components`, the cloning and moving commands,
  `trigger` and `commands_mut`, with the free command functions only they built; the hierarchy
  and relationship commands nothing issues (`insert_children`, `replace_children`, the `detach_*`
  and Bevy's deprecated `clear_children`/`remove_child(ren)`, `insert_related`,
  `replace_related`, `despawn_children`, `insert_recursive`/`remove_recursive`), `Children`'s
  sorts and `swap`, the spawners' accessors, and the relationship queries (`related`,
  `root_ancestor`, `iter_leaves`, `iter_siblings`, `iter_descendants_depth_first`). Kept:
  `UniqueEntityEquivalentVec::len`, `UniqueEntityEquivalentSlice::chunks_exact` and what it
  builds, which only the parallel query iterators call (a later part). On HEAD's copy every
  removed function marked `#[deprecated]` is used only from other removed functions, so no call
  falls through to another method of the same name. Doc sentences that pointed at a removed
  function went with it (the six that named `Commands::register_system` now name
  `World::register_system`), and the iterator types whose constructors went stay, unconstructed.
- **`bevy_ecs`'s uncalled functions, second part: the world, the queries and the rest of the
  entity collections**, found the same way (the analysis re-run on this state). Gone: the
  `World` methods nothing calls (`clear_all`, `try_despawn`, `despawn_no_free`, `try_query`,
  `try_query_filtered`, the non-send and by-id resource accessors and checks,
  `get_resource_change_ticks`, `resource_ref`, `resource_id`, `entity_count`, `entities_mut`,
  `entities_allocator`, `observers`, `removed_components`, `register_dynamic_bundle`,
  `register_component_hooks_by_id`, the required-components lookups,
  `write_message_default`), and `DeferredWorld`'s (`query`, the by-id and non-send accessors, the
  message writers); the entity accessors' unused API on `EntityWorldMut`, `EntityMut`,
  `EntityRef`, `FilteredEntityRef`/`FilteredEntityMut`, `EntityRefExcept`/`EntityMutExcept` and
  `UnsafeEntityCell` (the by-id getters and change ticks, `contains_id`, `get_ref`,
  `get_components(_mut)(_unchecked)`, the `into_*`/`as_*` conversions, `location`,
  `spawn_tick`/`spawned_by`, the cloning spawns, `EntityWorldMut`'s resource accessors and
  `into_world_mut`), `ComponentEntry`'s and `OccupiedComponentEntry`'s accessors,
  `FilteredResources(Mut)` and their builders' accessors; `QueryState`'s unused iteration,
  lookup and construction methods (`try_new`, `is_empty`, `contains`, the unchecked and
  `_manual` variants, the unique-many lookups, `par_iter_mut`, the matched-archetype lists),
  `QueryBuilder`'s `and`, `optional` and `mut_id`, `Access`'s `clear`, `archetypal` and
  `remove_component_write`; `Query::par_iter_many(_unique)(_mut)` with the two iterators they
  built (`QueryParManyIter`, `QueryParManyUniqueIter`), and the parallel helpers part one kept
  for them (`UniqueEntityEquivalentVec::len`, `UniqueEntityEquivalentSlice::chunks_exact`,
  `from_slice_iterator_unchecked`, the chunk iterator's `remainder`); `UnsafeFilteredEntityMut`
  whole (its two functions were its only use); `Entities`' spawn checks and `is_empty`,
  `CommandQueue::is_empty`, the entity maps' and sets' `keys`, `into_keys`, `drain`,
  `extract_if` and `into_inner`s, `EntityCloner`'s `linked_cloning`/`spawn_clone`,
  `EntityClonerBuilder::with_default_clone_fn`, `SceneEntityMapper`'s map getters: 224
  functions, with 19 inherent impls left empty. Kept: `EntityHashSet::is_empty` (benilla calls it;
  without it the call would fall through `Deref` to the inner set's), and `SpawnDetails`'
  `is_spawned` and `spawned_by` (the query type stays for now, and they are its fields' only
  readers). The deprecation proof is part one's. Doc sentences that pointed at a removed function
  went with it; links to `ComponentId` and `MutUntyped`, whose imports went, are written as paths.
- **`bevy_ecs`'s and `bevy_app`'s uncalled functions, third part: the rest**, found the same way
  (the analysis re-run: 232 dead functions were left, three of them kept by the second part).
  Gone: `App`'s and `SubApp`'s unused builders and accessors (`sub_app(_mut)`, `get_sub_app`,
  `sub_apps(_mut)`, `remove_sub_app`, `update_sub_app_by_label`, `init_schedule`,
  `configure_schedules`, `register_system`, `register_type_data`, `remove_systems_in_set`,
  `allow_ambiguous_component`, `register_disabling_component`, `get_error_handler`,
  `SubApp::add_plugins`, `take_extract`), `PluginGroupBuilder`'s `enable`/`try_add`,
  `FixedMainScheduleOrder`'s inserts, `ScheduleRunnerPlugin`'s `run_once`/`run_loop`,
  `TaskPoolOptions::with_num_threads`, `AppExit::is_success`; `Query`'s unused methods
  (`iter_combinations_mut`, the `_unsafe` iterators, `get_many_unique(_mut)(_inner)`, the lens
  conversions `as_query_lens`/`into_query_lens`, `transmute_lens_inner`, `join_inner`),
  `QueryLens::query_inner`; `Schedules`' and `Schedule`'s unused methods (with
  `set_apply_final_deferred` down to the `SystemExecutor` trait and both executors' impls, which
  nothing called), the DAG helpers' (`transitive_closure`/`reduction`, `is_toposorted`,
  `reserve_nodes`); the run conditions nothing adds (`resource_equals`,
  `resource_exists_and_equals`, `condition_changed(_to)`, `any_component_removed`); `On`'s event
  accessors, `Observer`'s and `ObserverDescriptor`'s unused builders and getters (and
  `AnyNamedSystem::system_name`, leaving the trait a marker); `MessageReader`'s and
  `MessageMutator`'s `len`, `par_read`, `read_with_id` and `clear`, `MessageWriter::write_default`,
  `RemovedComponents`' readers; the component registry's unused queueing and lookup methods;
  `SystemState`'s and `SystemMeta`'s accessors, `ParamBuilder`'s constructors, the function
  systems' `with_name`; `Ref::new`, `Ref::map`, `set_ticks`, `Tick::set`; the reflect type data's
  constructors and wrapper methods (`ReflectComponent`, `ReflectResource`, `ReflectBundle`,
  `ReflectEvent`, `ReflectFromWorld`; their function-pointer fields stay); the `error`, `info`,
  `debug` and `trace` error handlers (`panic`, `warn` and `ignore` stay); storage, archetype and
  batching getters and constructors nothing calls, and `SparseSet::values` with the sparse-set
  test's check of it: 225 functions (three of them the impls of two removed trait methods), 16
  inherent impls left empty. Kept: `DynSystemParam::downcast` and `downcast_mut_inner`, which
  bevy_ecs's type-inference test calls, and the readers of `On`'s `observer`, `ReflectEvent`'s function table, `SystemChangeTick`'s
  `last_run` and `RemovedSystem`'s fields stay with their types. Doc sentences that pointed at a
  removed function went with it or lost the link.
- **`bevy_ecs`'s and `bevy_app`'s unused types**, found like the functions: every `pub` struct,
  enum, trait, type alias, const and static of both crates made crate-private, put back where the
  workspace (also with `trace_chrome`) or the tests of bevy_ecs, bevy_app and every other vendored
  crate whose tests compile still name it, and where the derive macros' or exported macros'
  output names it; then only what rustc's dead-code lint names in every one of those builds. Gone:
  the entity maps' and sets' `Keys`, `IntoKeys`, `Drain` and `ExtractIf` iterators,
  `FromEntitySetIterator` with `EntitySetIterator::collect_set` (the trait is now a marker), the
  `UniqueEntityArray`/`UniqueEntitySlice`/`UniqueEntityVec` aliases, the unique slice's chunk,
  window and split iterators (`UniqueEntityEquivalentSliceIter(Mut)` and their 21 aliases), its
  `IterMut` alias and the unique vec's `Drain`/`Splice` aliases; `SpawnDetails` with its fetch (bevy_ecs's
  `system_state_spawned` test went with it; the readonly test keeps `Spawned` without it);
  `DescendantDepthFirstIter`; `RemovedIterWithId`, `BoxedReadOnlySystem`, `SCommands`;
  `OptionBuilder`, `ResultBuilder`, `IfBuilder`; `ResourceAccessLevel` with
  `EcsAccessType::Resource` and its arms in `is_compatible` and the conflict message (nothing built
  it but a test `QueryData` reading a resource, which went with it);
  `OrderedRelationshipSourceCollection`'s ten unused methods (`insert`, `remove_at`, the stable and
  sorted variants, `sort`, the push/pop pairs; `place` and `place_most_recent` stay, and their
  `self.insert` calls were the inherent `Vec`/`SmallVec` ones); `ScheduleError::ScheduleNotFound`;
  `ScheduleCleanupPolicy`'s `RemoveSystemsOnly` and `RemoveSystemsOnlyAllowBreakages` with their
  arms; `RunMode::Once` with its runner arm; and `RemovedSystem`, so `unregister_system(_cached)`
  return `Result<(), _>` and drop the removed system themselves (the missing-system error stays).
  Kept: `TriggerContext`'s unread `event_key` (the observers' safety contracts are written in its
  terms), `ComponentRelationshipAccessor`'s unread fields (the `Component` derive writes them),
  `DynSystemParam`'s downcasts (a test calls them) and the reflect type data's unread function
  pointers (bevy_ecs's reflection goes in its own step).
- **`bevy_ecs`'s reflection**, as far as `bevy_scene` and the kept tests allow (nothing outside
  them reads it: benilla names only `TypePath`, and no registry is filled since automatic
  registration went). Gone: `ReflectCommandExt` with `EntityWorldMut`'s `insert_reflect`/
  `remove_reflect` family, `ReflectBundle`, `ReflectEvent`, `World::get_reflect(_mut)` with
  `GetComponentReflectError` (their modules whole, with their tests); the entity cloner's reflect
  path (`ComponentCloneBehavior::reflect`, `component_clone_via_reflect`, `SourceComponent::
  read_reflect`, `ComponentCloneCtx`'s registry with `type_registry` and
  `write_target_component_reflect`, and the cloner's registry lookup), so the default clone
  handler is stock Bevy's without the feature, `component_clone_ignore` (only tests clone
  entities; the move tests replace the handler, and every other cloned test component is
  `Clone`); the relationship clone specializations' two `Reflect` impls (their traits stay, empty,
  because the `Component` derive names them; `ChildOf` takes the `Clone` impl and `Children` the
  hierarchy one, as before); `ReflectComponentFns` down to `apply_or_insert_mapped`, `reflect` and
  `register_component` and `ReflectResourceFns` to `apply_or_insert` and `copy` (what
  `bevy_scene` calls), with their wrapper methods; the `Reflect` derives (and `reflect(..)` type
  data) of `Name`, `Disabled`, `DefaultQueryFilters`, `Children`, `EntityIndex`,
  `EntityGeneration`, `EntityHash`, the entity maps and sets, `ComponentId`, `Tick`,
  `ComponentTicks`, `MaybeLocation`, `MessageId`, `MessageInstance`, `Messages`,
  `MessageSequence`, the five lifecycle events, `RemovedComponentEntity` and `ObservedBy`;
  bevy_ecs's `filtered_resource_reflect` test and the `EntityHashMap: Reflect` assertion. Kept:
  `AppTypeRegistry`, `ReflectComponent`, `ReflectResource`, `ReflectMapEntities`,
  `ReflectFromWorld` and `from_reflect_with_fallback` (`bevy_scene`'s spawn path), and the
  `Reflect` derives of `Entity` and `ChildOf` (`bevy_scene`'s kept tests reflect them). Types
  only: no plugin's build changed (only tests call `register_type`).
- **`bevy_ecs`'s `serialize` feature and the entity cloner.** Only `bevy_scene`'s `serialize` asked
  for bevy_ecs's `serialize`, and since its RON format went that feature gates nothing but
  `ScenePlugin`'s build: it stays, asking for nothing (`uuid`'s `serde` and `bevy_platform`'s
  `serialize` stay on, other crates ask). bevy_ecs loses the feature with the serde impls of
  `Entity`, `Name`, `ChildOf` and the entity maps and sets, their `reflect(Serialize,
  Deserialize)` type data, `Name`'s serde test and the `serde`/`serde_test` dependencies; the
  off `serialize` features of `bevy_input`, `bevy_window`, `bevy_diagnostic` and `bevy_internal`
  stop naming it. No crate's resolved features change. The entity cloner had no caller outside
  tests: `entity/clone_entities.rs` (`EntityCloner`, its builder and filters, `SourceComponent`,
  `ComponentCloneCtx`) and `observer/entity_cloning.rs` are deleted, with `EntityWorldMut`'s
  `clone_with_opt_out`/`clone_with_opt_in`/`clone_components`/`move_components`, the
  `remove_by_ids_with_caller` only the move path called, `clone_relationship_target`, the
  `bumpalo` dependency and their tests (bevy_camera's `test_add_visibility_class_hook` cloned
  too). `ComponentCloneBehavior` keeps `Default` and `Ignore` (`Custom`, `ComponentCloneFn`,
  `clone::<C>()`, `global_default_fn` and the clone handlers go): its one reader left is
  `bevy_scene`'s scene writing, which skips `Ignore` components, and every component answers
  that as before. The `Component` derive emits `Default` where the default specialization chose
  between `Default` and the `Clone` handler; the relationship specialization stays (a
  non-`Clone` relationship is still `Ignore`; `Clone` relationships and targets and `Children`
  get `Default` where they got their handlers), without the two empty `ViaReflect` traits.
- **`bevy_ecs`'s dynamic builders.** Nothing outside bevy_ecs's own tests builds a system or a
  query at run time, so `system/builder.rs` (`SystemParamBuilder`, `ParamBuilder`,
  `QueryParamBuilder`, `ParamSetBuilder`, `DynParamBuilder`, `LocalBuilder` and the
  `FilteredResources` builders, with their tests), `query/builder.rs` (`QueryBuilder` with its
  tests), `world/filtered_resource.rs` (`FilteredResources(Mut)` and their builders) and
  `DynSystemParam` with its state are deleted, with the `SystemParam` impls of those params and
  of `Vec<P>` and `ParamSet<Vec<P>>` (empty except when a builder filled them),
  `SystemState::build_system`/`build_system_with_input`/`build_any_system`/`from_builder`,
  `QueryState::from_builder` and its `From<QueryBuilder>`, the prelude's three names, and the
  `SystemParam` derive's `#[system_param(builder)]` option (the derive now rejects every
  struct-level option, as it did every other one). `FunctionSystem::new` loses its state
  argument: its one caller left passed `None`. No system, schedule or run-time path changed.
- **`bevy_ecs`'s query transmutes, lenses, joins and sorts.** Nothing outside bevy_ecs's own tests
  transmutes, joins or sorts a query (each was marked `#[deprecated]` and the workspace checked
  with `--all-targets`: no warning outside bevy_ecs), so `QueryState::transmute`/
  `transmute_filtered`/`join`/`join_filtered`, `Query::transmute_lens*`/`join*` with `QueryLens`
  and its two `From` impls, the seven `sort*` methods of `QueryIter` and of `QueryManyIter` with
  `QuerySortedIter`, `QuerySortedManyIter` and `NeutralOrd` (and `QueryManyIter`'s `world` field,
  read only by its sorts), and their tests are deleted. `FilteredEntityMut` goes whole, with its
  conversions, its tests and `TryFromFilteredError::MissingWriteAllAccess`; `FilteredEntityRef`
  stays for `ReflectComponent::reflect` (bevy_scene passes an `EntityRef`) but is no longer
  `QueryData`, and `QueryData::provide_extra_access`, which only the transmutes called (every
  other impl was the empty default or forwarded to its fields), leaves the trait, the tuple impls
  and the `QueryData` derive. `Observer::with_entities`/`watch_entities` (test-only; `with_entity`
  serves `EntityWorldMut::observe`) and the `Access`/`FilteredAccess` methods left without a
  caller (`has_any_write`, `has_write_all*`, `clear_writes`, `remove_conflicting_access`,
  `is_subset*`, `resource_reads*`/`resource_writes`, `add_unfiltered_*_all_resources`,
  `new_write_all`, `invertible_difference_with`) go too. No system, schedule or run-time path
  changed.
- **`bevy_ecs`'s whole-entity queries and the functions the last cuts left uncalled.** The
  function analysis once more (every `pub fn` of bevy_ecs and bevy_app crate-private, put back
  where a build names it, then rustc's dead-code lint in all five builds, and a `#[deprecated]`
  proof on what it found): `Ref::into_inner`, `Components::valid_resource_id`, both
  `register_component_with_descriptor`s (`ComponentsRegistrator`'s and `World`'s),
  `RequiredComponents::iter_ids`, `Entities::resolve_from_index` and `contains`, the unique
  slice's two `cast_slice_of_*` helpers, `Name::set` and `mutate`,
  `Schedules::configure_schedules`, `EntityMut::get_mut_assume_mutable`,
  `EntityWorldMut::into_mutable` and `into_mut_by_id`, and `World::bundles`, `spawn_empty_at`,
  `register_bundle` and `get_mut_by_id`. `EntityHashSet::is_empty` stays (benilla-app calls it;
  without it the call would resolve to the `HashSet` behind the `Deref`). `FilteredEntityRef`
  keeps `new`, `get` and `From<EntityRef>`, what `ReflectComponent::reflect` uses: its other
  accessors with the two tests that called them, its other six `From` impls, both `TryFrom`s
  with `TryFromFilteredError`, and its comparison, `Hash`, `ContainsEntity` and
  `EntityEquivalent` impls go. Nothing outside bevy_ecs's tests queries `EntityRef` or
  `EntityMut` (none of benilla's or the other vendored crates names either type), so their
  `WorldQuery`/`QueryData` impls, their `EntitySetIterator` impls on `QueryIter` and the 38
  tests and two test queries built on them go, with `Query`'s "Whole Entity Access" doc section.
  `EntityRefExcept` and `EntityMutExcept` stay (bevy_animation's `AnimationEntityMut`) without
  their comparison, `Hash`, `ContainsEntity`, `EntityEquivalent` and `EntitySetIterator` impls
  and without their `access` field, which nothing read once the `FilteredEntityRef` conversion
  went (their `get`s check the excluded bundle themselves). The `Access`/`FilteredAccess` and
  `UnsafeEntityCell` methods this left uncalled go too (`has_read_all*`,
  `FilteredAccess::read_all_components`/`write_all_components`, `get_change_ticks(_by_id)` with
  the private `get_ticks`). No system, schedule or run-time path changed.
- **The off `serialize` features of `bevy_input`, `bevy_window`, `bevy_diagnostic`, `bevy_time`,
  `bevy_transform` and `bevy_ui`, and of `bevy` and `bevy_internal`.** No manifest in the build
  names `bevy/serialize` any more, so none of them was on: their serde derives and
  `reflect(Serialize, Deserialize)` data, `FrameCount`'s hand-written serde impls with its test,
  the features and the `serde`/`serde_test` dependencies they enabled go, and so does
  `bevy_diagnostic`'s off `dynamic_linking` (one `not(..)` in the `sysinfo` gate). No crate's
  resolved features change.
- **`bevy_tasks`, to the multi-threaded pool and the API anything calls.** The build enables
  `multi_threaded`, `async_executor` and `futures-lite`, never `async-io`, so the single-threaded
  pool (`single_threaded_task_pool.rs`), the `no_std` executor (`edge_executor.rs`), the
  `async-io` and busy-wait `block_on`s, the `cfg` alias module and the two `Arc` coercions for
  targets without pointer atomics are deleted (`block_on` is `futures_lite`'s, as it was), with
  the `async-io`, `atomic-waker`, `crossbeam-queue` and `heapless` dependencies and the README's
  `no_std` section. Nothing calls `ParallelIterator` (no type implements it outside its own
  adapters) or the `futures` module (`now_or_never`, `check_ready`): `iter/` and `futures.rs` go.
  `ParallelSliceMut::par_splat_map_mut` with its test, `TaskPoolBuilder::stack_size` (never set,
  so every pool thread already took the system default), the pools' `try_get`, `Task::cancel` and
  `ThreadExecutorTicker::try_tick` had no caller (marked `#[deprecated]`, the workspace checked
  with `--all-targets`: no warning outside bevy_tasks). No system, schedule or run-time path
  changed.
- **`bevy_mikktspace`, and with it `Mesh::generate_tangents`/`with_generated_tangents` and
  `GenerateTangentsError`.** Its one caller was `ForwardDecalPlugin::build` (added by
  `PbrPlugin`), which generates the tangents of its 1×1 decal quad at startup. Measured once at
  the copy (a scratch test): mikktspace gave `[1.0, 0.0, 0.0, 1.0]` (bits `3f800000 0 0
  3f800000`) at each of the four vertices, so the plugin now inserts that attribute directly and
  `ForwardDecalMesh`'s asset is the same, bit for bit. The `bevy_mikktspace` features of `bevy`,
  `bevy_internal`, `bevy_mesh` and avian3d's `collider-from-mesh` request go.
- **`bevy_platform`, to `std`.** The build enables `std`, so the `no_std` fallbacks (the
  spin-lock `sync` types, the `Instant` and `sleep` fallbacks), the `portable-atomic` paths for
  targets without native atomics, the off `rayon` (parallel-iterator impls on `HashMap`/`HashSet`)
  and `critical-section` features go, with the `spin`, `critical-section` and `portable-atomic`
  dependencies and the README's `no_std` sections. `sync`, `time` and `thread` re-export `std`'s
  and `core`'s items under the same names. The `cfg` alias module (`switch!`, `define_alias!`, the
  `std`/`alloc`/`arc`/`panic_*`/`critical_section` aliases) goes with `bevy_utils`'s `cfg` module,
  its only other user: every block they gated was active and is now plain code. The off
  `critical-section` feature also leaves the manifests that only forwarded it (`bevy`,
  `bevy_internal`, `bevy_app`, `bevy_color`, `bevy_diagnostic`, `bevy_ecs`, `bevy_input`,
  `bevy_state`, `bevy_time`, `bevy_transform`, `bevy_transform_interpolation`). No crate's
  resolved features change on the desktop targets.
- **`bevy_platform`'s forwarding API.** `HashMap` and `HashSet` are newtypes over hashbrown's
  maps that `Deref` to them, with an inherent method forwarding to each hashbrown method of the
  same name. The forwards go, apart from the three constructors something calls (`HashMap::new`,
  and `with_hasher`/`with_capacity_and_hasher` on both types), which set the `FixedHasher`. Every
  call now reaches hashbrown's method through `Deref`, the method the forward called. Proved for
  each of the two cuts: every newtype method marked `#[deprecated]` before the cut gave the call
  sites (in the workspace with `--all-targets`, the `trace_chrome` builds and the tests of the
  vendored crates); after the cut, the same builds against a scratch hashbrown with every method
  marked `#[deprecated]` show each site of a removed method warning on hashbrown's method of the
  same name on the same type (275 of 275 for the first cut; 686 of 686 for the second, which took
  the nine forwards bevy_reflect's `Map`/`Set` impls needed, `get`, `get_mut`, `len`, `iter`,
  `drain`, `retain`, `insert`, `remove`, and `contains` on the set, once those impls went).
  bevy_scene's debounce loop copies the `u32` age out of its map before `insert`, since a mutable
  call through `DerefMut` cannot take the two-phase borrow the inherent `insert` did; bevy_ecs's
  `EntityHashSet::iter` doc names hashbrown's `iter` without a link. Also gone, none of them used: the
  `HashSet` assignment operators (bevy_ecs's `EntityHashSet` ones now reach hashbrown's through
  `Deref`), the array and `HashMap<T, ()>` `From` impls, the unused hashbrown re-exports (the raw
  entry builders, `EntryRef`, `ExtractIf`, `OccupiedError`, the key and value iterators, most of
  `hash_table`'s), the `sync` and `atomic` re-exports nothing names (`Barrier`, `Once`, `Weak`,
  the 8- and 16-bit and pointer-sized atomics, ...), `FixedState`'s re-export, and
  `SyncCell::to_inner`/`read`/`from_mut` and `SyncUnsafeCell::into_inner`/`get_mut`/`raw_get`
  with its `Default` and `From` impls (no caller; neither type has a `Deref`).
- **The leaf crates' uncalled API and the derives' unused options.** Every `pub fn` of
  `bevy_ptr`, `bevy_utils` and `bevy_macro_utils` was marked `#[deprecated]` on HEAD and the
  workspace (`--all-targets`) and the `trace_chrome` build checked; what no site called goes:
  `bevy_ptr`'s `ConstNonNull::new`/`new_unchecked`, every pointer's `to_unaligned` and
  `byte_offset`, `MovingPtr::new`, `assign_to` and `write_to` (with `IsAligned::copy_nonoverlapping`,
  which only `write_to` called), `PtrMut::as_ref`, `OwningPtr::cast`/`as_ref`/`as_mut` and the
  already-deprecated `ThinSlicePtr::get` (the macros' doc examples read the fields with `read`
  instead of `assign_to`); `bevy_utils`' `TypeIdMapExt`, `Parallel::clear`/`drain` and
  `DebugName::as_string` (its one caller, `bevy_ecs`'s `SystemName` tests under `trace` and
  `debug`, has not compiled since A4ax took `SystemName::name`, and goes with it); and
  `bevy_macro_utils`' `require_named` and `FQBox`. `bevy_utils`' `debug` and `parallel` features
  are always on, so the code they gated is unconditional and the `not(feature = "debug")`
  placeholder name is gone (the features stay, for what they enable). Unused macros go:
  `bevy_derive`'s `#[bevy_main]` (and its prelude re-export), `bevy_reflect_derive`'s
  `#[reflect_trait]` (and its prelude re-export) and `impl_from_reflect_opaque!`, and the
  `AsBindGroup` derive's `storage` and `storage_texture` field attributes, which no derive in the
  build uses (no longer declared, so a use would not compile; every other input expands as
  before). `bevy_log`'s off `trace_tracy_memory` feature (nothing forwards it) goes with its
  `tracy-client` dependency and global allocator.
- **The derives' unused options, second part, and the relationship accessor.** Options no input
  in the build uses, removed from the parsers so a use no longer compiles: `Component`'s
  `#[component(on_despawn = ..)]` (bevy_ecs's `spawned_by_set_before_flush` test was its one
  user and goes; a `linked_spawn` target still gets its despawn hook) and
  `#[component(map_entities)]` (docs only), `Event`'s `#[event(trigger = ..)]` (every `Event` is
  global-triggered), and `AsBindGroup`'s texture `multisampled` and `filterable`, the `1d`, `3d`,
  `cube` and `cube_array` dimensions, the `depth` and `s_int` sample types and the sampler's
  `sampler_type` (every input expands to the same tokens: `multisampled: false`, a filterable
  float or `u_int` texture, a filtering sampler); `bevy_macro_utils`' `get_lit_bool` with them.
  The relationship accessor goes whole: `RelationshipAccessor`, `ComponentRelationshipAccessor`,
  `Component::relationship_accessor` and the derive's implementation of it, the
  `ComponentDescriptor` field with `ComponentInfo::relationship_accessor` and
  `ComponentDescriptor::new_with_layout`'s last parameter. Only its own test
  (`dynamically_traverse_hierarchy`) read it; a component descriptor's `Debug` output is shorter.
- **The derives' unused options, third part, and event propagation.** Options only the derive
  crates' own tests used, removed from the parsers so a use no longer compiles, with those tests:
  `EntityEvent`'s `#[entity_event(propagate)]`, `propagate = ..` and `auto_propagate` (no event
  in the build propagates) with what only they reached, `PropagateEntityTrigger`,
  `SetEntityEventTarget`, the `traversal` module (`Traversal` and its relationship impl) and
  `On::propagate`/`original_event_target`; `Bundle`'s `#[bundle(ignore)]` and
  `#[bundle(ignore_from_components)]` (every derived bundle keeps all its fields and its
  `BundleFromComponents` impl); `Reflect`'s `#[reflect(where ..)]`, `no_field_bounds`,
  `type_path = false`, the custom trait functions (`Clone(f)`, `Debug(f)`, `PartialEq(f)`,
  `Hash(f)`; the plain idents stay) and the field options `clone`, `clone = ".."`, `default` and
  `default = ".."`. `#[reflect(opaque)]` stays (`Entity` uses it, behind `cfg_attr`), and so does
  `from_reflect = false` (bevy_animation). Every crate in the build expands to the same tokens as
  before, checked with `-Zunpretty=expanded` under the build's own features.
- **The derives' unused options, fourth part, and `bevy_reflect`'s generics info.** Options only
  docs and the derive crates' own tests used, removed from the parsers so a use no longer
  compiles: a component hook given as a call yielding a closure (`on_add = f(..)`) or with its
  path elided (`#[component(on_add)]`; every hook in the build names a path), `#[require(..)]`'s
  named-field value (`C { .. }`) and constructor call (`F::new(..)`) forms (tuple values, enum
  variants, associated consts and `= expr` stay), `EntityEvent`'s `#[event_target]` (every
  entity event targets its `entity` field or its only field), and the `FromWorld` derive whole.
  `#[entities]` on an enum's fields is now a compile error (only a test used it); an enum
  component without it no longer gets a `map_entities` whose arms were all empty, so the trait's
  default, also empty, runs instead (36 enums in the build). `bevy_reflect` loses its generics
  info, which only its own tests read (`GenericInfo::name`/`ty`; `TypeInfo::generics` had no
  caller): `Generics`, `GenericInfo`, `TypeParamInfo`, `ConstParamInfo`, the infos' `generics`
  field with `generics`/`with_generics`, and the derive's `generate_generics`, so a generic type's
  info is built without it (3 types in the build, and `Vec`'s); also `Type::is` (tests only) and
  `Type::type_path_table` (no caller). Every other crate in the build expands to the same tokens.
- **The trait impls nothing reaches: `bevy_mesh`'s conversions, `bevy_animation`'s `Animatable`
  types, and `bevy_math`'s cubic splines and curve derivatives.** A trait impl raises no
  dead-code warning, so each was cut and the workspace (all targets) and the crates' own tests
  checked, putting back what failed. `VertexAttributeValues` keeps `From` for `Vec<[f32; 2]>`,
  `Vec<[f32; 3]>`, `Vec<Vec3>`, `Vec<[f32; 4]>` and `Vec<u32>` (what benilla, benilla-gfx and the
  kept meshes and decal insert); the other 17 `From`s, all 30 `TryFrom`s back to a `Vec`,
  `FromVertexAttributeError` and, with its only reader, the `EnumVariantMeta` derive (on
  `VertexAttributeValues`, and in `bevy_derive`) go. `Animatable` keeps `Vec3`, `Vec3A` (which
  `Vec3`'s blend uses) and `Quat` (benilla animates `Transform`'s fields); `f32`, `f64`, `Vec2`,
  `Vec4`, `DVec2`-`DVec4`, `bool` (with `step_unclamped`, its only helper) and `Transform` go.
  No call site passes a literal whose type the removed impls could have decided. `bevy_math`
  loses `cubic_splines` (nothing outside the crate names a spline; the prelude re-exported
  them), `curve::derivatives` (only the splines implemented it) and `HasTangent`,
  `WithDerivative`, `WithTwoDerivatives` and `Sum`, which only those two read.
- **`bevy_math`'s modules nothing reaches, whole.** No name in them is used outside the crate
  except by `bevy_heavy`'s own tests (which do not build here: their dev-dependencies are not
  resolved for a patched crate), so the workspace (all targets) and the dependents' own tests
  check without them. Gone: `compass` (`CompassOctant`, `CompassQuadrant`), `IRect` with
  `Rect`/`URect::as_irect`, `curve::easing` (`Ease`, `EasingCurve`, `EaseFunction`, `JumpAt`, the
  easing curves; `avian3d` loses its two `Ease` impls, for `Position` and `Rotation`, which no
  caller reached), `sampling` (`ShapeSample`, `FromRng`, the `StandardUniform` distributions,
  `UniformMeshSampler`; the `rand` feature stays declared and on, with no code behind it), and
  `bounding` but `Aabb3d`: the 2D volumes, the ray and volume casts, `Bounded3d`, `BoundedExtrusion`
  and the primitives' impls, `BoundingSphere`, `IntersectsVolume`, `Aabb3d`'s inherent methods,
  and `BoundingVolume`'s methods but `center` and `half_size` with the `Rotation` type only they
  used (`bevy_camera`'s `Aabb` conversions are the one reader).
- **`bevy_math`'s primitives but the four the build meshes.** `Rectangle`, `Cuboid`, `Plane3d`
  and `Sphere` stay with their inherent methods (benilla and the kept meshes build them); every
  other 2D and 3D primitive goes with `polygon`, `Inset`, `Ring`/`ToRing`, `WindingOrder`, the
  `Primitive2d`/`Primitive3d` markers, `Measured2d`/`Measured3d` and the kept four's impls of them,
  and with `Ray2d` and `Ray3d`'s plane intersections (`InfinitePlane3d` and `Plane2d` were their
  arguments). Their only users went too: `bevy_heavy`'s 2D half (`ComputeMassProperties2d`,
  `MassProperties2d` and its primitive impls) and its `ComputeMassProperties3d` impls for the
  primitives (avian3d computes a `Collider`'s mass properties through parry, its own impl), and
  avian3d's `IntoCollider` with its primitive impls and the blanket `From` for `Collider` (benilla
  builds colliders with `Collider::sphere`, `capsule` and `trimesh`), with the two 2D names in
  avian3d's prelude. The crate docs that showed the removed API lose those parts. `bevy_heavy`'s
  and avian3d's own tests that set up with a primitive's mass properties (the `Cuboid` sum test,
  avian3d's integrator and `tests` module) did not build before (unresolved dev-dependencies) and
  name removed impls now.
- **`bevy_gizmos_render`**: nobody names it, and `GizmoRenderPlugin` did nothing in the main
  world but embed its WGSL (its render-app block only logged that no `RenderApp` exists), so it
  is deleted like `bevy_post_process`; the `bevy_gizmos_render` feature enables `bevy_gizmos`.
- **The render-app halves** (no `RenderApp` exists under gfx, so none of it ever ran). Every
  plugin keeps what it did in the main world (systems, sets, assets, types, required components,
  hooks), so benilla's recorded frame is unchanged, and every type benilla or another kept crate
  names stays:
  - `bevy_sprite_render`: the sprite, mesh2d, text2d and wireframe2d pipelines, extraction,
    batching and draw commands, `Mesh2dRenderPlugin` (it had no main-world half) and all WGSL. Kept:
    `SpriteRenderPlugin`'s slice systems (stand-ins since the `bevy_sprite` trim), `Material2d`,
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
  - `bevy_render`'s `render_resource` and `bevy_shader`'s render half (the third piece): the
    pipeline cache and shader cache (`bevy_shader` keeps `Shader`, its loader and `ShaderDefVal`,
    which `RenderPipelineDescriptor` names), the GPU buffer wrappers (`BufferVec`, the uniform,
    storage, batched and array buffers), `BindGroupEntries`, the layout-entry containers (the
    builder and `binding_types` stay for the bindless descriptors), the render and compute
    pipeline specializers with the `Specializer`/`SpecializerKey` derives
    (`SpecializedMeshPipelineError` stays for the material `specialize` signatures),
    `AsBindGroup::as_bind_group` and `PreparedBindGroup` (the derive's output and
    `unprepared_bind_group` stay), `GlobalsBuffer` (the `GlobalsUniform` resource stays), and the
    view uniforms, `ColorGradingUniform` and `ViewDepthTexture` (`ViewTarget`, which benilla-world's
    final pass reads, stays). No plugin's build changed.
  - `bevy_render` without wgpu (the fourth piece): no device is ever opened, so no GPU handle is
    ever created. `bevy_render/src/wgpu.rs` holds the part of wgpu's API the crate names:
    wgpu-types' plain-data types, wgpu's descriptor aliases, attachment structs and
    `BufferInitDescriptor`, and an uninhabited enum per handle type (device, buffer, texture,
    view, surface texture, sampler, bind group and layout, render and compute pipeline), so code
    holding one is unreachable. The wrappers (`Buffer`, `Texture`, `BindGroup`, ...) keep their
    ids and shapes; `RenderDevice` keeps the four methods the `AsBindGroup` derive's output and
    `AsBindGroup::bind_group_layout` call (`features`, `limits`, `create_bind_group_layout`,
    `create_buffer_with_data`). Gone with the preparation `RenderAssetPlugin` ran: the
    `RenderAsset` trait's methods (it keeps `SourceAsset`, which `RenderAssets` is keyed by),
    `PrepareAssetError`, `AssetExtractionError`, the fallback images' construction and the
    zero, cubemap and MSAA fallbacks (`FallbackImage` stays, the derive's parameter), the
    render queue, adapter and instance resources, `DefaultImageSampler`, the render-pass-only
    buffer slice, view creation and `OwnedBindingResource::get_binding`, and every wgpu
    re-export nothing names (`naga::ShaderStage` among them). wgpu, wgpu-core, wgpu-hal and
    their platform crates left the build; naga stays for `bevy_shader`'s `naga_oil`, without
    the `hlsl-out`, `msl-out` and `spv-out` writers only wgpu-core enabled.
  - `bevy_render`'s render-world remainder (the fifth piece): `ExtractedView` and
    `RetainedViewEntity`, `ExtractedWindow(s)` with the surface texture, the render targets'
    texture-view lookups (`NormalizedRenderTargetExt` keeps `get_render_target_info` and
    `is_changed`, which `camera_system` calls), the shadow views' occlusion-culling components,
    `ReadbackComplete` and `Readback`'s constructors, `TemporaryRenderEntity`, the render and
    compute pipeline wrappers with `ComputePipelineDescriptor`, the uncallable `From<wgpu::..>`
    impls on the wrappers, the `binding_types` helpers the bindless descriptors do not call, and
    the methods nothing calls on `ViewTarget` (it keeps the five benilla-world's final pass uses),
    `ColorAttachment`, `OutputColorAttachment`, `ColorGrading`, `GpuImage` and
    `ShaderStorageBuffer`; `TemporalJitter` (a camera component nothing adds) with them. Types
    only: no plugin's build changed. `ReadbackComplete`, `TemporaryRenderEntity` and
    `TemporalJitter` leave the auto-registered type registry.

  The manifests drop the dependencies no kept code uses; no crate's resolved features change
  (`fixedbitset`, `nonmax` and `slotmap` lose `default`, which only enabled `std`, still on)
  except naga's (above), `web-sys`'s WebGL, clipboard and input-event features and
  `getrandom`'s `wasm_js`, which only wasm builds compile.
- **`bevy` and `bevy_internal`'s manifests** keep only the features the build enables or a
  manifest in it names (the workspace's list, `debug`, `trace_tracy`, `trace_chrome`, and
  `bevy_transform_interpolation`'s `libm`), each
  enabling what it did minus the cut crates; `bevy`'s examples, tests, dev-dependencies, profiles
  and `dynamic_linking` are gone. The optional dependencies no kept feature reaches left with them
  (`bevy_audio`, `bevy_dev_tools`, `bevy_feathers`, `bevy_ui_widgets`, `bevy_solari`,
  `bevy_remote`, `bevy_camera_controller`; `bevy_winit` stays a crate for bevy_egui and
  benilla-gfx, not a `bevy` feature, and is itself trimmed above), and so did every crate only they pulled (gltf, gilrs, cpal,
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
glob import also reached (gone since, with automatic registration), and `bevy_ui`'s three float literals passed to taffy's
`length`/`percent` spell out the `f32` the compiler already fell back to.
