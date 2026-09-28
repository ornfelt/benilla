# Porting benilla to C: dependencies and a rough plan

Notes from 2026-09-28, answering three questions:

1. What other Rust crates and dependencies does benilla use?
2. Could all of benilla be ported to C, and what C packages could replace them?
3. A rough plan for a C port that uses gfx as a library (not necessarily a DLL), with C
   libraries or direct Rust-to-C ports for everything else.

Numbers are from the `gfx-dll-backend` branch at `915d50a4`.

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
| bevy_ecs | the whole game's structure | **flecs** (a C ECS: systems, queries, pipelines, observers, change detection), or plain data-oriented structs with a hand-written frame order | See "the runtime" below; the biggest single decision |
| bevy asset | MPQ asset source, handles, async loaders | own: a ref-counted handle table plus an IO thread pool | Small once the MPQ reader exists |
| bevy transform, hierarchy, visibility | transform propagation, frustum culling, render layers | own | Straightforward |
| bevy animation | M2 skeletal animation and blending | own M2 sampler (tracks, interpolation, blend slots) | benilla already implements the reference's blend rules on top of Bevy; port the rules, not Bevy |
| bevy ui, text | glue screens | benilla's own quad and text path | The in-game UI does not use them |
| glam (bevy math) | vectors, matrices, quaternions | **cglm** (or HandmadeMath) | |
| avian3d | colliders, ray and shape casts, character sweep | own triangle BVH plus swept sphere or capsule; or **JoltC** (Jolt's C API), **ODE** or **Bullet**'s C API | Queries only, no dynamics: a small own implementation is likely simpler than a physics engine |
| mlua and lua-src (Lua 5.1, patched) | the UI VM | the **Lua C API** directly, keeping `third_party/lua-src`'s patched Lua 5.1.5 | Lua is C; mlua's glue becomes hand-written `lua_*` bindings |
| roxmltree | FrameXML and TOC XML | **expat**, **yxml** or **libxml2** | yxml is tiny and fast; expat is the safe default |
| kira (patched), symphonia, cpal, rtrb | mixer, MP3 / WAV / ADPCM decode, audio device, lock-free ring | **miniaudio** (device, mixing, spatialization) with **dr_mp3** and **dr_wav** (ADPCM included); a C11-atomics ring buffer | Carry kira's per-chunk spatial-gain patch into the mixer |
| cosmic-text, fontdb | font loading, shaping, layout, glyph raster | **FreeType** plus **HarfBuzz** (C API), or **stb_truetype** with own layout | WoW 1.12's fonts are simple; stb_truetype plus kerning may be enough |
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

### Principles

- **The Rust client is the oracle.** Keep it building on the side. Every ported piece is
  checked against it: byte-identical parser output, identical packets, and the capture harness
  images diffed as this branch did for gfx (`benilla-visual`, `probe_ab.sh`).
- **Port bottom-up.** Leaf crates first, where differential testing is easy; the ECS-bound game
  last.
- **gfx as a static library.** Link `gfx_benilla` statically (CMake `add_subdirectory`) instead
  of loading it as a DLL; the benilla-specific API additions are already there.
- **Keep the offline tools in Rust at first:** `wc_compiler_rs` (the `.gfxs` compiler),
  `benilla-visual` and the dump tools. They are not part of the client.
- **The rules still hold:** the install stays read-only, local state lives in
  `benilla-config/`, no assets or client code are committed, and settings default to the stock
  1.12 values.

### Phase 0: foundations (weeks)

- Build: CMake, C11 or C17 (C23 if `#embed` is wanted), with warnings as errors, ASan and UBSan
  in debug, and clang-tidy.
- Pick the container and allocation style (stb_ds or klib, arenas) and an error style (return
  codes with an error struct).
- A platform layer: gfx for the window, input and device; miniaudio; threads; sockets; files; the
  clock; the OS RNG.
- A test harness (plain C test runner, e.g. greatest or utest.h) and a differential-test
  runner that feeds the same input to the Rust and C builds and compares the outputs.

### Phase 1: formats and protocol (about 90k lines of Rust)

Port `benilla-bytes`, `-mpq`, `-blp`, `-dbc`, `-adt`, `-wdt`, `-wmo`, `-m2`, `-formats`, `-srp`
and `-protocol`.

- These are pure functions over bytes, the easiest code to port and to verify.
- Differential tests: parse every file in the install with both builds and compare dumps. Check
  packet encode and decode round trips, and the SRP6 login against a local server.
- Replace flate2 with zlib, num-bigint with libtommath, `sha1` with a C SHA-1. Optionally swap
  the MPQ reader for StormLib.

### Phase 2: the runtime that replaces Bevy (the key decision)

Choose between:

- **flecs.** Closest to Bevy: entities, components, queries, systems in pipelines, observers,
  change tracking. The port of each system stays structurally recognisable, which matters for
  400+ system files.
- **Hand-written data-oriented modules.** Explicit arrays per subsystem and an explicit frame
  function calling each update in order. Faster and simpler to debug, but every system has to be
  re-designed, not just translated.

Recommendation: flecs for `benilla-app` (so the translation stays mechanical), with plain arrays
inside hot subsystems (terrain streaming, particles, the model draw lists). Before porting,
record the real system order: `headless_client()` in `benilla-app` builds the full schedule
graph without a window, so it can be dumped.

Also in this phase:

- asset handles and the async IO pool
- transform propagation, visibility and culling, render layers
- time, input state (`ButtonInput` equivalents) and window state

### Phase 3: rendering on gfx

Translate `benilla-gfx` (12k lines): images, meshes, data textures, materials, draw lists, post
(FFXGlow), UI lane, screenshots, the GPU meter. It already talks to gfx through FFI, so the C
version drops the Bevy extraction and reads the new runtime's components instead. The `.gfxs`
shaders are reused unchanged.

Checkpoint: the engine viewer (`benilla-worldview`'s equivalent) renders Northshire from the
install, and the capture diffs against the Rust build are as small as the gfx port's own.

### Phase 4: the world (`benilla-world`, 62k lines)

Terrain and WDL streaming, liquid, sky, weather, the M2 and WMO model paths, lighting and the
shared light buffer, particles, the M2 animation sampler (replacing Bevy's `AnimationPlayer`),
and collision (the own BVH and sweep, or JoltC).

Checkpoint: every world capture scenario diffs against the Rust build.

### Phase 5: the UI engine (`benilla-ui`, 120k lines)

XML (expat or yxml), TOC, templates, anchors, layout, and the Lua bindings written against the
Lua C API on the patched Lua 5.1.5. This ports cleanly: the Lua side is already C, and the
bindings are mechanical.

Checkpoint: the stock FrameXML loads with the same frame tree and the same errors, and the
`ui-*` captures match.

### Phase 6: the game (`benilla-app`, 359k lines)

Port by subsystem, each behind a live A/B against the Rust client on the local server (the
`.probe-identity` account, `smoke.sh`-style login and logout, `WOW_LIVE_SHOT` diffs):

1. network session and object store
2. login, realm and character screens
3. entities
4. player and movement
5. chat and text filters (PCRE2)
6. UI glue and bindings
7. portraits
8. sound (miniaudio with the kira patch's per-chunk gains)
9. text rendering (FreeType or stb_truetype)
10. settings and CVars (tomlc99)
11. the capture harness
12. the dev tools (cimgui)

### Phase 7: tests and parity

About 166k lines of Rust tests. Port the ones that pin reference facts (packet layouts, DBC
columns, UI behaviour, schedule-order invariants). Cover the rest with the differential runner
and the capture and live A/B harnesses. The project is at parity when `smoke.sh`, the capture
sweep and a live session match the Rust client on every platform gfx supports.

### Effort and risks

- **Size.** About 500k lines of program code. Phases 1 and 3 are weeks to a few months;
  phases 2, 4, 5 and 6 are the bulk and realistically take years without heavy tooling help.
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
  the recorded schedule graph and the differential tests are the defence.
- **A cheaper middle path:** keep Rust for the game and expose a C API around it (or around
  the finished leaf crates), porting only the parts that need to be C. Worth deciding before
  Phase 2, because Phase 2 is where the port stops being mechanical.
