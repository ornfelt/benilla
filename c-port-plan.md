# Porting benilla to C: dependencies and a rough plan

Notes from 2026-09-28, answering three questions:

1. What other Rust crates and dependencies does benilla use?
2. Could all of benilla be ported to C, and what C packages could replace them?
3. A rough plan for a C port that uses gfx as a library (not necessarily a DLL), with C
   libraries or direct Rust-to-C ports for everything else.

Numbers are from the `gfx-dll-backend` branch at `915d50a4`. The plan starts with a Rust-side
step (Phase A) that runs benilla, unchanged, on a trimmed Bevy of its own; that trimmed Bevy is
then ported to C and benilla is translated onto it.

## 1. What benilla is made of

### Size

About 688k lines of Rust in 20 workspace crates. About 166k of those are test modules
(`#[cfg(test)]` blocks and `*tests*.rs` files), which leaves roughly **500k lines of program
code**. There are also 26 shader programs: WGSL for the wgpu path, and the `.gfxs` ports under
`crates/benilla-gfx/shaders` for the gfx path (about 6.8k shader lines in all).

### The workspace crates

| Crate | Rust lines | What it is | External crates |
|---|---|---|---|
| `benilla-app` | 359k | The game: login, network session, entities, player, UI glue and bindings, sound, portraits, chat, capture harness, dev tools | bevy, avian3d, cosmic-text, fontdb, kira, symphonia, cpal, rtrb, crossbeam-channel, regex, serde, toml, image, half, bytemuck, mint, include_dir, arboard / smithay-clipboard, libc, assert_no_alloc, rand, bevy_egui (dev), objc2 / coreaudio (macOS), windows-sys (Windows) |
| `benilla-ui` | 120k | Engine-free WoW UI core: TOC and FrameXML parsing, templates, anchors, layout, z-order, the Lua bindings | mlua (Lua 5.1, vendored and patched), roxmltree |
| `benilla-world` | 62k | The world renderer and simulation: terrain, WDL, water, sky, weather, M2/WMO models, lighting, collision, the frame ladder | bevy, avian3d, bytemuck, image, objc2 (macOS) |
| `benilla-formats` | 51k | The 1.12 file formats on top of the small parser crates, plus the dev dump tools | image, clap, glam (tests) |
| `benilla-protocol` | 34k | The 1.12.1 wire protocol: auth and world packets, compression | flate2, strum, clap, num-bigint and rand (tests) |
| `benilla-gfx` | 12k | The gfx backend (this branch): FFI, the Bevy-ECS-to-gfx renderer, window and input | bevy, wgpu (a no-op device only), include_dir, bevy_egui |
| `benilla-assets` | 8k | Bevy asset loaders over the MPQ chain; most of the WGSL | bevy, image, serde |
| `benilla-visual` | 2k | Image diff and crop tooling for the capture harness | image |
| `benilla-m2`, `-mpq`, `-srp`, `-adt`, `-blp`, `-dbc`, `-wmo`, `-wdt`, `-bytes`, `-buildstamp` | 0.1-1.8k each | Small byte-level parsers: M2, MPQ (flate2), SRP6 (num-bigint, rand), ADT, BLP (texpresso), DBC, WMO, WDT, byte reading, the build stamp | as listed |
| `benilla`, `benilla-worldview` | tiny | The launcher and the engine viewer binaries | none |

Vendored and patched in `third_party/`:

- **lua-src:** Lua 5.1.5 with six grammar hunks, so it accepts exactly the 1.12 client's Lua
  (5.0 constructs such as the iterator-less `for k, v in t do`). See its `BENILLA.md`.
- **kira:** one patch that evaluates spatial gains per chunk instead of per sample, for mixer
  cost. See its `BENILLA.md`.

### How much Bevy is used

Bevy 0.18.1 is the backbone, not a thin layer. About 476 source files in `benilla-app` and
`benilla-world` use ECS system parameters (`Query`, `Res`, `Commands`). The most-used modules:

- **ECS:** systems, schedules, resources, messages and events, observers, states, change
  detection.
- **render:** meshes, images, materials, cameras, render layers, visibility.
- **camera**, **mesh**, **image**, **asset** (the `mpq://` asset source, handles, async loaders).
- **animation:** `AnimationPlayer` and `AnimationGraph` drive M2 skeletal animation (about 250
  uses), with `SkinnedMesh`.
- **input** and **window**, **time**, **math** (glam), **transform** and its hierarchy.
- **pbr** and **core_pipeline** (material extensions, HDR, tonemapping off).
- **ui** and **text:** only for the glue screens. The in-game UI is benilla's own FrameXML
  engine.
- Also enabled: scene, picking, gizmos (the bowstring and fishing line), sprites.

avian3d (physics) is used almost only for queries: colliders, `SpatialQuery` ray and shape
casts, AABBs and the `move_and_slide` character controller. There is only one `RigidBody` use,
so no dynamics simulation is needed.

## 2. Is a C port possible, and with what?

**Possible: yes.** Nothing in benilla depends on something only Rust can do. The platform is
gfx plus sockets, threads, files and audio, all available in C. Lua and PCRE are C already. The
format parsers and the protocol are plain byte work.

**The cost is the size and the architecture, not the libraries.** 500k lines is a large
body of code to port by hand. The hard part is that the game is written as Bevy ECS systems with an implicit
schedule, change detection and asset handles. That runtime has to be rebuilt or replaced before
most of `benilla-app` and `benilla-world` can be ported.

There is no practical automatic Rust-to-C translator. `mrustc` compiles Rust to C, but its
output is machine code in C syntax: unreadable and unmaintainable, so it is not a porting path.
`c2rust` only goes the other way. Expect a manual port (with LLM help), module by module,
checked against the Rust client.

### Dependency map: Rust crate to C replacement

| Rust | Used for | C replacement | Notes |
|---|---|---|---|
| bevy: window, input | window, events, cursor, grab | **gfx** window backends (x11, sdl, glfw, win32) | Already done on this branch through `gfx_benilla` |
| bevy: render, wgpu, WGSL | all drawing | **gfx** devices (gl3, gl4, gles3, vk, d3d11, d3d12) with the existing `.gfxs` shaders | The shaders and draw logic are already ported (`benilla-gfx`, 12k lines of mostly FFI-shaped Rust); translate that crate almost line for line |
| bevy_ecs | the whole game's structure | **the trimmed Bevy ECS, ported to C** (Phase 2); flecs is the off-the-shelf alternative, but its semantics differ from Bevy's | See Phases A and 2 |
| bevy asset | MPQ asset source, handles, async loaders | own: a ref-counted handle table plus an IO thread pool | Small once the MPQ reader exists |
| bevy transform, hierarchy, visibility | transform propagation, frustum culling, render layers | own | Straightforward |
| bevy animation | M2 skeletal animation and blending | own M2 sampler (tracks, interpolation, blend slots) | benilla already implements the reference's blend rules on top of Bevy; port the rules, not Bevy |
| bevy ui, text | glue screens | benilla's own quad and text path | The in-game UI does not use them |
| glam (bevy math) | vectors, matrices, quaternions | **cglm** (or HandmadeMath) | |
| avian3d | colliders, ray and shape casts, character sweep | own triangle BVH plus swept sphere or capsule; or **JoltC** (Jolt's C API), **ODE** or **Bullet**'s C API | Queries only (one `RigidBody` use, no dynamics simulation): a small own implementation is likely simpler than a physics engine; PhysX is overkill |
| mlua and lua-src (Lua 5.1, patched) | the UI VM | the **Lua C API** directly, keeping `third_party/lua-src`'s patched Lua 5.1.5 | Lua is C; mlua's glue becomes hand-written `lua_*` bindings |
| roxmltree | FrameXML and TOC XML | **expat**, **yxml** or **libxml2** | yxml is tiny and fast; expat is the safe default |
| kira (patched), symphonia, cpal, rtrb | mixer, MP3 / WAV / ADPCM decode, audio device, lock-free ring | **miniaudio** (device, mixing, spatialization) with **dr_mp3** and **dr_wav** (ADPCM included), or **PortAudio** plus an own mixer if that already exists in another client; a C11-atomics ring buffer | Carry kira's per-chunk spatial-gain patch into the mixer. WoW 1.12 has only MP3 and WAV (with ADPCM): no Ogg, so no libvorbis |
| cosmic-text, fontdb | font loading, shaping, layout, glyph raster | **FreeType** plus **HarfBuzz** (C API), or **stb_truetype** with own layout | WoW 1.12's fonts are simple; stb_truetype plus kerning may be enough. benilla hands fontdb the font bytes it loads itself, so no system font lookup (fontconfig, DirectWrite) is needed |
| image | PNG read and write (captures, tools) | **stb_image** and **stb_image_write**, or lodepng | |
| texpresso | BC1-5 decode on the CPU (devices without BC) | **bcdec** (single-header C) | |
| flate2 | zlib in MPQ and packets | **zlib** or **miniz** | Or replace `benilla-mpq` with **StormLib** (C++ with a C API) |
| num-bigint, sha1, rand | SRP6 login | **libtommath** or **mbedTLS**'s bignum; a public-domain `sha1.c` or mbedTLS; the OS RNG (`getrandom`, `BCryptGenRandom`) | Small: `benilla-srp` is under 1k lines |
| regex | the chat profanity and spam filters | **PCRE2** | The 1.12 client itself used PCRE (`text_filter.rs` documents the reference's PCRE behaviour) |
| serde, toml | `benilla-config/config.toml` | **tomlc99** or toml-c | |
| strum, bytemuck, mint, half, include_dir | enum names, casts, math interop, f16, embedded files | macro tables, memcpy, a small f16 routine, `xxd -i` or C23 `#embed` | Not needed as libraries |
| crossbeam-channel | thread channels | a mutex-and-condvar queue or a lock-free MPMC queue | |
| arboard, smithay-clipboard | clipboard | add clipboard ops to `gfx_benilla` per window backend (SDL and GLFW have them; x11 and win32 are small) | |
| bevy_egui (dev panel) | debug UI | **cimgui** (Dear ImGui's C API) on gfx's existing `gfx_imgui` | |
| libc, windows-sys, objc2 / coreaudio | platform calls | the C APIs directly; Objective-C `.m` files on macOS | |
| clap | CLI dump tools | getopt or hand parsing | |
| std: threads, net, fs, time | IO pool, sockets, files | C11 threads or pthreads (tinycthread on Windows), BSD sockets and Winsock, stdio, a monotonic clock | |
| Rust containers (Vec, HashMap, String) | everywhere | **stb_ds**, **klib** (khash, kvec), sds; per-frame and per-load arenas | Pick one set early and use it everywhere |

## 3. A rough plan

### The route

```text
Today                Rust + Bevy + wgpu/winit (feature off) or gfx (this branch)
   |
Phase A (Rust)       benilla unchanged, on a trimmed Bevy of its own + gfx; no wgpu, no winit
   |
Phases 0-7 (C)       C + gfx; the trimmed Bevy ported to C, then benilla translated onto it
```

The first step off Bevy's renderer is already done: this branch draws everything through gfx,
with all 26 shader programs ported to `.gfxs` and every Linux window and device pair verified
(`GFX.md`).

**Phase A does not change benilla's code or structure.** It replaces the Bevy benilla depends
on with a trimmed copy that implements only what benilla uses, with the same API. benilla's
systems, components, plugins and queries stay exactly as they are and compile against the
trimmed copy. Once that copy runs benilla and is proven in Rust (the tests, the capture sweep,
`smoke.sh` and live logins match upstream), it becomes the translation reference for the C
port: first the trimmed Bevy is ported to C, keeping its behaviour, then benilla's code is
translated onto that C runtime, drawing through the gfx library.

Why this order:

- **Phase A is low-risk.** Vendoring changes no behaviour, and every cut after it is
  small and checked by benilla's roughly 166k lines of tests and the harnesses. benilla's 430k
  Bevy-bound lines are not touched.
- **The C port gets an exact, small reference.** The trimmed Bevy is the precise set of
  behaviours benilla relies on (system order, change ticks, deferred commands, message
  lifetimes, observer timing, asset events), readable in one place instead of spread across
  390k lines of Bevy.
- **The C runtime keeps benilla's semantics.** A C ECS written to match the trimmed Bevy behaves
  as benilla expects. A different ECS (flecs, for example) differs in ways benilla depends on:
  flecs tracks changes per table, Bevy per entity and component; command application, message
  lifetimes and observer timing differ too. Each difference is a potential bug across 400+
  systems.
- **Everything is checked against running code.** Upstream benilla checks Phase A; the Phase A
  Rust build checks the C runtime and the C translation.

The price moves to the C side: C cannot express Bevy's type-driven API (a system declares its
data through its signature, `Query<(&A, &mut B), Changed<A>>`), so every system's parameters
become explicit runtime queries and lookups in C. That work is regular and follows a written
mapping table (Phase 2), but there is a lot of it: about 1,400 queries, 4,000 resource
parameters and 1,100 `Commands` uses. The Bevy-free crates (formats, protocol, SRP6, UI core) do
not need Phase A and can be ported to C in parallel (Phase 1).

### Principles

- **Upstream benilla is the oracle.** Phase A happens in a fork (or long-lived branch), because
  it drops the wgpu path upstream keeps. Keep an unmodified upstream build beside it, and check
  every step against it: the capture sweep (`scripts/visual.sh`, `benilla-visual`,
  `probe_ab.sh`), `smoke.sh`, and live A/B logins as this branch did for gfx.
- **benilla's code and structure stay as they are in Phase A.** Only the Bevy underneath
  changes. The one edit to benilla itself is dropping the feature-off wgpu path (A1), which the
  trimmed Bevy no longer provides.
- **The C port translates, it does not redesign.** The trimmed Bevy is ported to C with its
  behaviour, and benilla's systems keep their names, order and data flow in C.
- **gfx as a static library.** Link `gfx_benilla` statically (CMake `add_subdirectory`) instead
  of loading it as a DLL; the benilla-specific API additions are already there.
- **Keep the offline tools in Rust at first:** `wc_compiler_rs` (the `.gfxs` compiler),
  `benilla-visual` and the dump tools. They are not part of the client.
- **The rules still hold:** the install stays read-only, local state lives in
  `benilla-config/`, no assets or client code are committed, and settings default to the stock
  1.12 values.

### Phase A (Rust): a trimmed Bevy under an unchanged benilla

Order the steps so each one leaves a working client that passes the capture sweep and a live
login.

**A1. Make gfx the only backend.** In the fork, drop the feature-off wgpu path and the `gfx`
feature switch, so `benilla-gfx` is always on. Remove `WinitPlugin`, `RenderPlugin`, the no-op
wgpu device (`noop_device.rs`) and the WGSL.

**A2. Record the frame.** Dump Bevy's real system order while it is still stock.
`headless_client()` in `benilla-app` builds the full schedule graph without a window. Write out
every schedule, set and system with its ordering constraints. A test pins it; the trimmed
scheduler must produce the same order, and so must the C one later.

**A3. Vendor the Bevy crates benilla uses.** Copy the Bevy 0.18.1 crates in benilla's build into
the workspace (as `third_party/kira` is vendored) and point the workspace at them through
`[patch.crates-io]`. This changes no behaviour, so it lands at once. Bevy is MIT OR Apache-2.0,
so vendoring is allowed; keep its licence files beside the copy. The crates that are Bevy
plugins themselves come along: avian3d (trimmed to the collision queries benilla uses) and
bevy_egui (dev builds only).

**A4. Trim to what benilla uses, keeping the API benilla calls.** Delete what nothing in
benilla reaches, crate by crate, with the tests and the capture sweep after each cut. The Bevy
crates in the build come to about 390k lines, docs and tests included; benilla names about 270
distinct Bevy types. The ECS features it uses, counted across the app, world and assets crates:

| Feature | Uses |
|---|---|
| `Res<>` / `ResMut<>` | 4,070 |
| `Query<>` | 1,373 |
| `Commands` | 1,123 |
| system ordering (`.before`, `.after`, `.chain`) and sets (`in_set`) | about 900 |
| messages (`MessageReader` / `MessageWriter`) | 648 |
| `Handle<>`, `AssetServer`, `AssetEvent` | about 820 |
| `Visibility`, `GlobalTransform` | about 890 |
| `Local<>` / `NonSend<>` | about 670 |
| animation (`AnimationPlayer`, `AnimationGraph`, `AnimationClip`) | 305 |
| change detection (`Changed`, `Added`, `is_changed`) | about 180 |
| parent and child relationships (`ChildOf`, `Children`) | 183 |
| observers and component hooks | about 120 |
| states (`OnEnter`, `OnExit`, `NextState`) | 69 |

Not used: reflection derives, sub-states and computed states, `FixedUpdate`. Per crate:

| Bevy crates | What happens to them |
|---|---|
| `bevy_render`, `bevy_pbr`, `bevy_core_pipeline`, `bevy_post_process`, `bevy_anti_alias`, `bevy_shader`, `bevy_winit`, `bevy_ui_render`, `bevy_sprite_render`, `bevy_gizmos_render` | **Cut to their data types.** The gfx renderer replaced their rendering; keep only the components and structs benilla and `benilla-gfx` name (`Mesh3d`, `MeshMaterial3d`, `Camera`, `RenderLayers`, `Msaa`, `StandardMaterial` and similar), with the same paths |
| `bevy_ecs` (about 64k non-comment lines) | **Trim** to the features in the table above: world, entities, archetype storage, queries and filters, change ticks, commands, messages, resources, `Local` and `NonSend`, relationships, observers and hooks, and a scheduler that keeps the recorded order. The multithreaded executor can stay, or give way to a single-threaded one that runs the A2 order |
| `bevy_app`, `bevy_time`, `bevy_input`, `bevy_window`, `bevy_state`, `bevy_transform`, `bevy_camera`, `bevy_tasks`, `bevy_diagnostic` | **Trim** to what benilla calls. Each is a few thousand lines before trimming |
| `bevy_asset` (21k) | **Trim** to the handle tables, `Assets<T>`, `AssetServer`, asset events and the loader path benilla's `mpq://` source uses; drop the processor, hot reload and file watching |
| `bevy_animation` (4.5k), `bevy_mesh`, `bevy_image` | **Trim** to the evaluation, skinning data and mesh and image types benilla uses |
| `bevy_ui`, `bevy_text`, `bevy_gizmos`, `bevy_picking`, `bevy_scene`, `bevy_sprite` | **Trim** to the glue screens' nodes and text, the two gizmo lines, ray picking and what avian's collider backend needs. `bevy_ui` lays out through taffy (flexbox), which stays in Rust; its C counterpart is Yoga (C API) or Clay |
| `bevy_reflect` (32k) | **Cut as far as the rest allows**; benilla derives no `Reflect` itself |
| `bevy_math` | **Keep as the glam re-export** benilla uses; glam maps onto cglm |

**A5. Checkpoint.** `cargo tree` shows no Bevy, wgpu or winit from crates.io: every Bevy crate
left is the trimmed, vendored copy. benilla's own code is unchanged apart from A1. The tests
pass, the A2 order test passes, the capture sweep matches upstream benilla as closely as the
gfx branch did, `smoke.sh` is green, and live A/B logins match on every gfx pair. From here the
trimmed Bevy is frozen as the reference.

Why not keep Bevy behind a C API instead: Bevy has no C bindings and its Rust ABI is not
stable. A hand-written `extern "C"` layer over its dynamic ECS API is possible, but the program
would still run on Bevy's runtime underneath; that is not a C codebase.

### Phase 0 (C): foundations

- Build: CMake, C11 or C17 (C23 if `#embed` is wanted), with warnings as errors, ASan and UBSan
  in debug, and clang-tidy.
- Pick the container and allocation style (stb_ds or klib, arenas) and an error style (return
  codes with an error struct).
- A platform layer: gfx for the window, input and device; the audio library (miniaudio, or
  PortAudio and an own mixer); threads; sockets; files; the clock; the OS RNG.
- A test harness (plain C test runner, e.g. greatest or utest.h) and a differential-test
  runner that feeds the same input to the Rust and C builds and compares the outputs.

A source layout along the lines of:

```text
src/
  app/        the frame function, states, settings
  bevy/       the trimmed Bevy, ported to C: ecs/ app/ asset/ transform/ animation/ time/ input/
  platform/   gfx glue, input, audio, net, threads, files
  assets/     handles, IO pool, the mpq:// source
  wow/        adt/ blp/ dbc/ m2/ mpq/ wdt/ wmo/ formats/
  protocol/   auth, SRP6, world packets
  render/     the benilla-gfx translation: images, meshes, materials, draw lists, post, UI lane
  world/      terrain, wdl, liquid, sky, weather, models, lighting, particles, collision
  ui/         FrameXML, TOC, layout, the Lua bindings
  game/       benilla-app's subsystems
third_party/  gfx_benilla, lua-5.1.5 (patched), zlib, freetype, audio, xml, pcre2, ...
```

### Phase 1 (C): formats and protocol (about 90k lines of Rust; can run alongside Phase A)

Port `benilla-bytes`, `-mpq`, `-blp`, `-dbc`, `-adt`, `-wdt`, `-wmo`, `-m2`, `-formats`, `-srp`
and `-protocol`.

- These are Bevy-free already, pure functions over bytes, the easiest code to port and to
  verify. Translate benilla's own code; there is no advantage in a third-party WoW library.
- Differential tests: parse every file in the install with both builds and compare dumps. Check
  packet encode and decode round trips, and the SRP6 login against a local server.
- Replace flate2 with zlib, num-bigint with libtommath, `sha1` with a C SHA-1. Optionally swap
  the MPQ reader for StormLib.

### Phase 2 (C): the trimmed Bevy, ported to C

Port the frozen Phase A Bevy to C, keeping its behaviour: benilla's own small C runtime.

- **ECS:** components registered at runtime (size, alignment, drop function), archetype tables,
  entities with generations, queries built from descriptors (read, write, optional, `With`,
  `Without`, `Changed`, `Added`), per-component change ticks as Bevy keeps them, a command
  buffer applied where Bevy applies it, resources, messages with Bevy's two-frame lifetime,
  relationships, observers and hooks.
- **Scheduler:** the A2 order as an explicit list of systems per schedule, single-threaded
  first. A test compares the order with the Rust one.
- **The rest:** app and plugin registration, time, input, window state, states, transforms and
  propagation, visibility, cameras, assets (handles, `Assets<T>`, events, the IO pool),
  animation evaluation, the glue-screen UI layout (Yoga or Clay in place of taffy).
- **Differential tests** against the Rust trimmed Bevy: the same operations in both, the same
  query results, change ticks and command outcomes.

Then write the **mapping table** the rest of the port follows, for example:

| Rust (benilla on the trimmed Bevy) | C |
|---|---|
| `fn sys(q: Query<(&A, &mut B), Changed<A>>, r: Res<R>, mut cmds: Commands)` | `void sys(World *w, SysCtx *ctx)` with a query descriptor `{ read A, write B, changed A }` built once at registration, and `res_get(w, R)`, `cmds_*(ctx, ...)` inside |
| `for (a, mut b) in &mut q` | `QueryIter it = query_iter(w, q); while (query_next(&it)) { const A *a = it_get(&it, 0); B *b = it_get_mut(&it, 1); }` |
| `commands.spawn((A { .. }, B { .. }))` | `Entity e = cmds_spawn(ctx); cmds_insert(ctx, e, A, &a); cmds_insert(ctx, e, B, &b);` |
| `Local<T>` | a per-system state struct in `SysCtx` |
| `NonSend<T>` | a main-thread resource |
| `MessageReader<M>` / `MessageWriter<M>` | `msg_read(ctx, M, &cursor)` / `msg_write(w, M, &m)` |
| `.add_systems(Update, sys.after(other).run_if(c))` | an entry in the recorded schedule list, with its run condition |
| `Handle<Image>` | a generation-checked handle into the image table |

### Phase 3 (C): rendering on gfx

Translate `benilla-gfx` (12k lines): images, meshes, data textures, materials, draw lists, post
(FFXGlow), UI lane, screenshots, the GPU meter. It already talks to gfx through FFI; in C it reads
components through the Phase 2 runtime instead of Bevy's ECS. The `.gfxs` shaders are reused unchanged: the
single-source shader pipeline (`.gfxs` compiled by `wc_compiler_rs` into SPIR-V, HLSL and GLSL
families) already exists, so no per-backend shader copies are maintained by hand.

Checkpoint: the engine viewer (`benilla-worldview`'s equivalent) renders Northshire from the
install, and the capture diffs against the Rust build are as small as the gfx port's own.

### Phase 4 (C): the world (`benilla-world`, 62k lines)

Terrain and WDL streaming, liquid, sky, weather, the M2 and WMO model paths, lighting and the
shared light buffer, particles, the M2 path on the Phase 2 animation evaluation, and collision
(avian3d's queries as trimmed in Phase A, as an own triangle BVH with ray and capsule sweeps, or
JoltC).

Checkpoint: every world capture scenario diffs against the Rust build.

### Phase 5 (C): the UI engine (`benilla-ui`, 120k lines)

XML, TOC, templates, anchors, layout, and the Lua bindings written against the Lua C API on the
patched Lua 5.1.5. This ports cleanly and is one place where C is simpler than Rust: the Lua
side is already C, so mlua's glue becomes direct `lua_*` calls with no binding layer in between.

Checkpoint: the stock FrameXML loads with the same frame tree and the same errors, and the
`ui-*` captures match.

### Phase 6 (C): the game (`benilla-app`, 359k lines)

Port by subsystem, each behind a live A/B against the Rust client on the local server (the
`.probe-identity` account, `smoke.sh`-style login and logout, `WOW_LIVE_SHOT` diffs):

1. network session and object store
2. login, realm and character screens
3. entities
4. player and movement
5. chat and text filters (PCRE2)
6. UI glue and bindings
7. portraits
8. sound (miniaudio, or PortAudio and an own mixer, carrying kira's per-chunk spatial-gain
   patch)
9. text rendering (FreeType, or stb_truetype)
10. settings and CVars (tomlc99)
11. the capture harness
12. the dev tools (cimgui)

### Phase 7 (C): tests and parity

About 166k lines of Rust tests. Port the ones that pin reference facts (packet layouts, DBC
columns, UI behaviour, schedule-order invariants). Cover the rest with the differential runner
and the capture and live A/B harnesses. The project is at parity when `smoke.sh`, the capture
sweep and a live session match the Rust client on every platform gfx supports.

### Effort and risks

- **Size.** About 500k lines of program code. Phase A trims Bevy and leaves benilla alone.
  Phases 1 and 3 are contained: Bevy-free code and an already FFI-shaped renderer. Phase 2 (the
  trimmed Bevy in C) is the hardest single piece. Phases 4, 5 and 6 are translation on top of
  it, following the mapping table, but they are the bulk of the volume: every system's
  parameters become explicit queries and lookups.

  Rough difficulty per piece, given what this branch already did:

  | Piece | Difficulty |
  |---|---|
  | window and input on gfx | done |
  | rendering on gfx, shaders | done in Rust; translation only |
  | math, zlib, SHA-1, XML | low |
  | SRP6, the file parsers, Lua bindings | medium |
  | audio, text, collision | medium to high (behaviour to match) |
  | trimming Bevy in Rust (Phase A) | medium: deletion, checked by the tests |
  | Bevy assets and animation in C | high |
  | Bevy ECS, scheduling, change detection in C | highest |
  | benilla's systems onto the C runtime | medium per system, large in total |

- **What is lost from Rust:**
  - memory and thread safety
  - enums with data (tagged unions in C)
  - traits and generics (macros or function tables)
  - closures and iterators
  - `Result` and `?` error handling
  - exhaustive `match`

  Plan for sanitizers, fuzzing the parsers (the install and the network are input), and
  review.
- **Bevy's implicit behaviour:** system ordering, change detection, deferred commands, and
  asset readiness gates such as the booth "pending" waits. These are easy to get subtly wrong;
  Phase A's recorded frame (A2), the trimmed Bevy as the reference, and the differential tests
  are the defence.
- **A cheaper middle path:** keep Rust for the game and expose a C API around it (or around
  the finished leaf crates), porting only the parts that need to be C. Decide before Phase 2,
  since Phase 2 is the big commitment; Phase A is useful either way.

## Appendix: if C# is ever a target

The Phase A result also feeds a C# port: the trimmed Bevy is a small, exact reference for a C#
runtime, and Rust translates closely into C# (records and pattern matching, interfaces and
generics, lambdas, built-in collections, a garbage collector). C# can also express much of
Bevy's system-parameter style with generics, so benilla's systems keep more of their shape
than in C.

- **Native libraries stay native,** called through P/Invoke: gfx (wrapped in `SafeHandle`
  types), the patched Lua, and the audio library.
- **.NET's standard library** covers most of the rest:

  | Need | .NET |
  |---|---|
  | math | `System.Numerics` |
  | f16 | `System.Half` |
  | big integers (SRP6) | `System.Numerics.BigInteger` |
  | SHA-1 | `System.Security.Cryptography` |
  | zlib | `System.IO.Compression` |
  | XML | `System.Xml` |
  | thread queues | `System.Threading.Channels` |
  | RNG | `RandomNumberGenerator` |
  | config (TOML) | Tomlyn (NuGet) |

- **NuGets for the rest:**
  - text: HarfBuzzSharp and FreeTypeSharp
  - images: ImageSharp
  - dev UI: ImGui.NET
  - collision: BepuPhysics or JoltPhysicsSharp, if not an own BVH
- **Watch:** garbage-collector pauses (per-frame allocation discipline, pooling) and P/Invoke
  call counts (batch draws, keep Lua bindings coarse).
