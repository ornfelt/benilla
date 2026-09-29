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
  - `bevy_egui`: the render-graph edge ordering egui against `bevy_ui_render`'s UI pass.

  The manifests drop the dependencies no kept code uses; no crate's resolved features change.
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
