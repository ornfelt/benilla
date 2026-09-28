# Porting benilla to C: dependencies and a rough plan

Notes from 2026-09-28, answering three questions:

1. What other Rust crates and dependencies does benilla use?
2. Could all of benilla be ported to C, and what C packages could replace them?
3. A rough plan for a C port that uses gfx as a library (not necessarily a DLL), with C
   libraries or direct Rust-to-C ports for everything else.

Numbers are from the `gfx-dll-backend` branch at `915d50a4`. The plan starts with a Rust-side
step that removes Bevy before any C is written (Phase A), then translates the Bevy-free Rust to
C.

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

**The cost is the size and the architecture, not the libraries.** 500k lines is a multi-year
effort by hand. The hard part is that the game is written as Bevy ECS systems with an implicit
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
| bevy_ecs | the whole game's structure | **flecs** (a C ECS: systems, queries, pipelines, observers, change detection), or plain data-oriented structs with a hand-written frame order | Chosen in Phase A, while still in Rust (see the plan) |
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
Phase A (Rust)       Rust + own runtime and ECS + gfx; no Bevy, no wgpu, no winit
   |
Phases 0-7 (C)       C + gfx; a mechanical translation of the Phase A code
```

The first step off Bevy's renderer is already done: this branch draws everything through gfx,
with all 26 shader programs ported to `.gfxs` and every Linux window and device pair verified
(`GFX.md`). Phase A finishes the job in Rust, and the C port then translates code that no
longer depends on Bevy.

Why Phase A first:

- **The hard part is solved once, with a safety net.** Turning Bevy's implicit behaviour
  (system order, change detection, deferred commands, asset readiness, the animation player)
  into explicit code is where subtle bugs come from. In Rust, the compiler and benilla's roughly
  166k lines of tests keep checking that work, and the capture and live harnesses still compare
  against upstream benilla.
- **The C translation becomes mechanical.** Once the Rust uses the same runtime shape and the
  same libraries the C code will, each file maps onto C nearly one to one:

  | Rust | C |
  |---|---|
  | `Vec3`, `Mat4` (glam) | cglm types |
  | `Vec`, `HashMap`, `String` | the chosen container library |
  | enums with data | tagged unions |
  | `Result` and `?` | error codes with an error struct |
  | `Arc` | reference counts |
  | channels | queues |

- **The same result can feed a C# port later**, if that is ever wanted (see the appendix).

The price is a large refactor of the Bevy-bound Rust (about 430k lines in `benilla-app`,
`benilla-world` and `benilla-assets`), mostly structural rather than new logic. The Bevy-free
crates (formats, protocol, SRP6, UI core) do not need Phase A and can be ported to C in parallel
(Phase 1).

### Principles

- **Upstream benilla is the oracle.** Phase A happens in a fork (or long-lived branch), because
  it drops the wgpu path upstream keeps. Keep an unmodified upstream build beside it, and check
  every step against it: the capture sweep (`scripts/visual.sh`, `benilla-visual`,
  `probe_ab.sh`), `smoke.sh`, and live A/B logins as this branch did for gfx.
- **Pick the C libraries during Phase A, and swap them in while still in Rust** wherever the
  swap changes something measurable: text rasterization, audio mixing, collision. Settle those
  differences with the Rust harnesses, not in C. Libraries whose output is identical (zlib for
  flate2, a SHA-1) can wait for the translation.
- **Write the new runtime in a C-shaped Rust style:** plain structs, explicit allocation and
  ownership, few trait objects, no deep generics, closures only where C would take a callback
  and a context pointer.
- **gfx as a static library.** Link `gfx_benilla` statically (CMake `add_subdirectory`) instead
  of loading it as a DLL; the benilla-specific API additions are already there.
- **Keep the offline tools in Rust at first:** `wc_compiler_rs` (the `.gfxs` compiler),
  `benilla-visual` and the dump tools. They are not part of the client.
- **The rules still hold:** the install stays read-only, local state lives in
  `benilla-config/`, no assets or client code are committed, and settings default to the stock
  1.12 values.

### Phase A (Rust): remove Bevy

Order the steps so each one leaves a working client that passes the capture sweep and a live
login.

**A1. Make gfx the only backend.** In the fork, drop the feature-off wgpu path and the `gfx`
feature switch, so `benilla-gfx` is always on. Remove `WinitPlugin`, `RenderPlugin`, the no-op
wgpu device (`noop_device.rs`) and the WGSL.

**A2. Record the frame.** Dump Bevy's real system order before replacing the scheduler.
`headless_client()` in `benilla-app` builds the full schedule graph without a window. Write out
every schedule, set and system with its ordering constraints. This list becomes the explicit
frame function, and a test pins it.

**A3. Choose the runtime shape, the one the C code will have.**

- **flecs through its Rust bindings** (`flecs_ecs`; check its maturity first). The Rust and C
  code then share the same ECS: entities, components, queries, systems in pipelines,
  observers. The C translation keeps every system's structure. This is the recommendation for
  the 400+ system files in `benilla-app`.
- **Own data-oriented modules** with an explicit frame function: simpler and faster, but every
  system is redesigned rather than translated.

Hot subsystems (terrain streaming, particles, model draw lists) can use plain arrays inside
either choice.

The shape of a system in each:

```rust
// Bevy today
commands.spawn((Transform::from_translation(p), Mesh3d(mesh), UnitModel { .. }));
```

```c
// flecs in C (the Rust bindings are the same shape)
ecs_entity_t e = ecs_new(world);
ecs_set(world, e, Transform, { .translation = p });
ecs_set(world, e, Mesh3d, { mesh });
ecs_set(world, e, UnitModel, { ... });
```

**A4. Replace Bevy's pieces one at a time,** roughly from the leaves inward:

| Bevy piece | Replacement in Rust (then in C) | Notes |
|---|---|---|
| `Time`, `FrameCount` | own clock resource | trivial |
| input (`ButtonInput`, mouse and keyboard messages) and `Window` | own input and window state, fed from gfx events | `benilla-gfx/src/window.rs` and `input.rs` already translate gfx events into Bevy's; point them at the new state instead |
| messages and events | own double-buffered queues | keep Bevy's one-frame lifetime semantics |
| `States` (the client state machine) | an explicit state enum with enter and exit hooks | |
| transform and hierarchy | own `Transform`, `GlobalTransform`, parent and children, propagation | |
| visibility, frustum culling, `RenderLayers` | own | the gfx renderer already consumes these from the ECS |
| `Assets<T>`, `Handle<T>`, `AssetServer`, `AssetEvent` | own ref-counted handle tables, an IO thread pool, load events | the `mpq://` source and loaders are benilla's own code already |
| `Mesh`, `Image`, materials | plain structs the gfx renderer reads | the gfx path already reads them field by field |
| `AnimationPlayer`, `AnimationGraph`, `SkinnedMesh` | own M2 animation sampler (tracks, interpolation, blend slots) and skinning | about 250 uses; benilla already implements the reference's blend rules on top of Bevy, so port the rules |
| cameras and projections | own camera components | `WowPortraitProjection` is already custom |
| bevy_ui and bevy_text (glue screens) | benilla's own quad and text path | the in-game UI does not use them |
| gizmos | own line drawing (the bowstring and fishing line) | |
| picking | own ray picking | |
| bevy_egui | Dear ImGui through the gfx `gfx_imgui` layer (a Rust binding now, cimgui in C) | dev builds only |
| avian3d | own triangle BVH with ray and capsule sweeps, or JoltC | settle collision parity here, in Rust |
| the change-detection uses (`Changed<T>`, `Added<T>`, `is_changed`) | flecs change tracking or explicit dirty flags | audit each use; these are where ordering bugs hide |
| deferred `Commands` | flecs deferred mode, or an explicit command buffer applied at the same points Bevy did | the A2 dump says where |

**A5. Swap the behaviour-visible libraries** to their C choices, still in Rust through `-sys`
crates or small bindings:

- text: FreeType (plus HarfBuzz if needed), or stb_truetype
- audio: miniaudio (or PortAudio and an own mixer), carrying kira's per-chunk spatial-gain patch
- regex: PCRE2 (what the 1.12 client itself used for the chat filters)
- XML: expat or yxml (optional; roxmltree's output is easy to match)

**A6. Checkpoint.** `cargo tree` shows no `bevy`, `wgpu` or `winit`. The capture sweep matches
upstream benilla as closely as the gfx branch did, `smoke.sh` is green, and live A/B logins
match on every gfx pair.

### Phase 0 (C): foundations (weeks)

- Build: CMake, C11 or C17 (C23 if `#embed` is wanted), with warnings as errors, ASan and UBSan
  in debug, and clang-tidy.
- Pick the container and allocation style (stb_ds or klib, arenas) and an error style (return
  codes with an error struct), matching what Phase A settled on.
- A platform layer: gfx for the window, input and device; the audio library from A5; threads;
  sockets; files; the clock; the OS RNG.
- A test harness (plain C test runner, e.g. greatest or utest.h) and a differential-test
  runner that feeds the same input to the Rust and C builds and compares the outputs.

A source layout along the lines of:

```text
src/
  app/        the frame function, states, settings
  ecs/        flecs setup, or the own runtime
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

### Phase 2 (C): the runtime

Translate the Phase A runtime: the ECS setup (flecs has the same API in C), the frame function
from A2, assets and the IO pool, transforms, visibility, input, time, messages and states.
Because Phase A already made the design decisions, this is translation, not design.

### Phase 3 (C): rendering on gfx

Translate `benilla-gfx` (12k lines): images, meshes, data textures, materials, draw lists, post
(FFXGlow), UI lane, screenshots, the GPU meter. It already talks to gfx through FFI, and after
Phase A it reads the runtime's own components. The `.gfxs` shaders are reused unchanged: the
single-source shader pipeline (`.gfxs` compiled by `wc_compiler_rs` into SPIR-V, HLSL and GLSL
families) already exists, so no per-backend shader copies are maintained by hand.

Checkpoint: the engine viewer (`benilla-worldview`'s equivalent) renders Northshire from the
install, and the capture diffs against the Rust build are as small as the gfx port's own.

### Phase 4 (C): the world (`benilla-world`, 62k lines)

Terrain and WDL streaming, liquid, sky, weather, the M2 and WMO model paths, lighting and the
shared light buffer, particles, the M2 animation sampler, and collision, all as settled in
Phase A.

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
8. sound
9. text rendering
10. settings and CVars (tomlc99)
11. the capture harness
12. the dev tools (cimgui)

### Phase 7 (C): tests and parity

About 166k lines of Rust tests. Port the ones that pin reference facts (packet layouts, DBC
columns, UI behaviour, schedule-order invariants). Cover the rest with the differential runner
and the capture and live A/B harnesses. The project is at parity when `smoke.sh`, the capture
sweep and a live session match the Rust client on every platform gfx supports.

### Effort and risks

- **Size.** About 500k lines of program code. Phase A is the single biggest step (structural
  work across about 430k lines of Rust). Phases 1 and 3 are weeks to a few months. Phases 2, 4,
  5 and 6 shrink to mostly translation after Phase A, but are still large by volume.

  Rough difficulty per piece, given what this branch already did:

  | Piece | Difficulty |
  |---|---|
  | window and input on gfx | done |
  | rendering on gfx, shaders | done in Rust; translation only |
  | math, zlib, SHA-1, XML | low |
  | SRP6, the file parsers, Lua bindings | medium |
  | audio, text, collision | medium to high (behaviour to match) |
  | Bevy assets and animation | high |
  | Bevy ECS, scheduling, change detection | highest |

- **What is lost from Rust:**
  - memory and thread safety
  - enums with data (tagged unions in C)
  - traits and generics (macros or function tables)
  - closures and iterators
  - `Result` and `?` error handling
  - exhaustive `match`

  Budget time for sanitizers, fuzzing the parsers (the install and the network are input), and
  review.
- **Bevy's implicit behaviour:** system ordering, change detection, deferred commands, and
  asset readiness gates such as the booth "pending" waits. These are easy to get subtly wrong;
  Phase A's recorded frame (A2) and the differential tests are the defence.
- **A cheaper middle path:** keep Rust for the game and expose a C API around it (or around
  the finished leaf crates), porting only the parts that need to be C. Decide before Phase A,
  since Phase A is the big commitment.

## Appendix: if C# is ever a target

The Phase A result also feeds a C# port: the de-Bevyed Rust translates closely into C#
(records and pattern matching, interfaces and generics, lambdas, built-in collections, a
garbage collector), and flecs has the same model in C# (Flecs.NET).

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
