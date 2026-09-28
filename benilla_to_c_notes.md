what rust crates and dependencies does benilla use?
If i wanted to make it use my gfx library for graphics and window (sdl,glfw,x11,win32 and vk/d3d11/d3d12/gl4/gl3/gles3). I think that would work but if i then wanted to translate the project to c# or c, what packages / rust crates do i need to also port (or use appropriate nuget or similar)

--

I checked the current Benilla tree. It is now a fairly complete 1.12.1 client, and its architecture is actually quite favorable for what you're thinking: the file-format readers, protocol/SRP code, and core UI engine are already separated from Bevy. The renderer/world and application integration are the Bevy-heavy parts. :chatgpt-content-reference{index="0"}

## 1. What Benilla currently uses

The main external Rust dependencies visible across the Benilla workspace are roughly:

| Rust crate | What Benilla uses it for | If porting to C | If porting to C# |
|---|---|---|---|
| **bevy** | ECS, app loop, assets, rendering integration, scene/world, input/window integration | custom engine/ECS or library | custom engine/ECS or Arch/Flecs.NET/etc. |
| **wgpu** | low-level GPU access used alongside Bevy | **your GFX library** | your GFX C API via P/Invoke |
| **avian3d** | collision/physics | Jolt, Bullet, PhysX, custom | JoltPhysicsSharp, BepuPhysics, etc. |
| **bevy_egui** | debug/developer UI | Dear ImGui | ImGui.NET |
| **glam** | vectors/matrices/quaternions | cglm/custom math | `System.Numerics` |
| **mint** | math interoperability | unnecessary | unnecessary |
| **bytemuck** | safe casts of structs ↔ byte buffers | ordinary C structs/memcpy | `MemoryMarshal`, `Span<T>` |
| **half** | fp16 | `_Float16` / half helper | `System.Half` |
| **image** | PNG/etc image handling | stb_image/libpng | ImageSharp / built-in alternatives |
| **texpresso** | BC/DXT texture decompression, relevant for BLP | custom BC1/2/3 decoder or library | BCnEncoder.Net or custom |
| **flate2** | zlib/deflate, particularly MPQ/network | zlib | `System.IO.Compression` |
| **num-bigint** | SRP6 big integers | GMP / LibTomMath / custom | `System.Numerics.BigInteger` |
| **sha1** | WoW auth/SRP | OpenSSL/etc or compact SHA1 impl | `System.Security.Cryptography` |
| **rand** | randomness | OS RNG / small PRNG | `Random` / `RandomNumberGenerator` |
| **serde** | serialization/data structures | hand parsing / serialization lib | `System.Text.Json` where applicable |
| **toml** | config | tomlc99/etc | Tomlyn |
| **clap** | CLI arguments | getopt/custom | `System.CommandLine` or custom |
| **anyhow** | Rust error handling | return/error structures | exceptions/results |
| **crossbeam-channel** | threaded queues | pthread/Win32 queue/custom | `Channel<T>` |
| **rtrb** | realtime ring buffer | custom ring buffer | custom ring buffer |
| **cpal** | raw audio output | PortAudio/miniaudio | PortAudio binding / native audio layer |
| **kira** | audio engine/mixing/spatial audio | custom/PortAudio+own mixer/miniaudio | custom/native engine |
| **symphonia** | audio decoding | libvorbis/etc | NVorbis/etc |
| **cosmic-text** | text shaping/layout | FreeType + HarfBuzz | HarfBuzzSharp + FreeTypeSharp |
| **fontdb** | font discovery | fontconfig/DirectWrite/etc | platform font APIs |
| **arboard** | clipboard | your window/platform layer | native .NET/platform API |
| **smithay-clipboard** | Wayland clipboard | only needed for Wayland | likely unnecessary |
| **raw-window-handle** | native window handles | **your GFX/window API** | your native interop |
| **libc** | OS calls | native C already | P/Invoke/native shim |
| **objc2** | macOS Cocoa integration | Objective-C calls | native shim |
| **mlua** | Lua / WoW FrameXML scripting | **Lua 5.1 C directly** | Lua C library + P/Invoke |
| **roxmltree** | XML/FrameXML parsing | libxml2 / expat | `System.Xml` / LINQ to XML |
| **include_dir** | compile-time embedded assets | generated C arrays / resource files | EmbeddedResource |

Benilla also vendors two particularly important modified dependencies:

**Lua 5.1** is patched to accept the exact grammar expected by WoW 1.12, and **Kira** has Benilla-specific audio changes. :chatgpt-content-reference{index="1"}

That means I would not casually replace those with generic alternatives if compatibility matters.

---

# 2. Where your GFX library fits

Your existing abstraction:

```text
window:
    SDL
    GLFW
    X11
    Win32

graphics:
    Vulkan
    D3D11
    D3D12
    OpenGL 4
    OpenGL 3
    OpenGL ES 3
```

is conceptually a very good fit for replacing this section:

```text
Benilla
   |
 Bevy rendering/window
   |
 wgpu + winit
   |
 Vulkan / DX12 / Metal / GL etc.
```

with:

```text
Benilla
   |
   +--------- your engine-side renderer
   |
 GFX
   |
   +-- Window
   |    +-- SDL
   |    +-- GLFW
   |    +-- X11
   |    `-- Win32
   |
   `-- Device
        +-- Vulkan
        +-- D3D11
        +-- D3D12
        +-- GL4
        +-- GL3
        `-- GLES3
```

So **`wgpu`, `raw-window-handle`, and Bevy's window/render backend become unnecessary**.

But there's a catch.

Benilla's `benilla-world` isn't merely issuing raw wgpu calls. It is built around Bevy concepts including rendering, assets, meshes, shaders, schedules, ECS components, cameras, transforms and other engine facilities. The project map explicitly describes `benilla-world` as the world renderer and `benilla-assets` as Bevy `AssetSource`/`AssetLoader` integration. :chatgpt-content-reference{index="2"}

Therefore replacing:

```text
wgpu -> gfx
```

is not enough.

You'd effectively be doing:

```text
Bevy Renderer
Bevy RenderWorld
Bevy Assets
Bevy Mesh
Bevy Material
Bevy Camera
Bevy shaders/WGSL
Bevy window/input
wgpu
winit

        ↓

your own equivalents

        ↓

GFX
```

---

# 3. Luckily, a large chunk of Benilla is already portable

This is probably the most interesting thing for your plan.

The current architecture explicitly labels these as **no Bevy**: :chatgpt-content-reference{index="3"}

```text
benilla-adt
benilla-blp
benilla-buildstamp
benilla-bytes
benilla-dbc
benilla-formats
benilla-m2
benilla-mpq
benilla-protocol
benilla-srp
benilla-ui
benilla-visual
benilla-wdt
benilla-wmo
benilla-worldview
benilla launcher
```

Only these major pieces are specifically marked Bevy:

```text
benilla-app
benilla-assets
benilla-world
```

That's an unusually nice boundary for a port.

Benilla's README even explicitly says:

> The format readers and the UI engine core are plain Rust with no Bevy in them

and says the world renderer can run without the game attached. :chatgpt-content-reference{index="4"}

---

# 4. If you port Benilla to C

I think your dependency stack could actually become fairly small.

Something like:

```text
benilla-c
│
├── gfx                         your library
│   ├── SDL / GLFW / X11 / Win32
│   └── VK / D3D11 / D3D12 / GL4 / GL3 / GLES3
│
├── zlib                        MPQ / network compression
│
├── Lua 5.1                     FrameXML/addons
│   └── Benilla WoW grammar patches
│
├── audio
│   ├── PortAudio              <- you already use this
│   └── own WoW mixer
│
├── text
│   ├── FreeType
│   └── HarfBuzz
│
├── XML
│   └── libxml2 / expat
│
├── image
│   └── stb_image / libpng
│
├── crypto
│   ├── SHA1
│   └── bigint/SRP
│
└── physics/collision
    └── Jolt / custom
```

A lot of Rust dependencies become unnecessary simply because C provides their functionality naturally.

For example:

```text
bytemuck
libc
raw-window-handle
mint
anyhow
include_dir
crossbeam
```

don't need direct 1:1 replacements.

---

# 5. Some crates would probably be easier to port than replace

For these:

```text
benilla-bytes
benilla-mpq
benilla-blp
benilla-dbc
benilla-adt
benilla-wdt
benilla-wmo
benilla-m2
benilla-srp
benilla-protocol
```

I'd translate the actual Rust implementation.

They are domain-specific Benilla code.

For example:

```text
benilla-mpq
    Rust flate2
         ↓
    C zlib

benilla-srp
    Rust num-bigint + sha1
         ↓
    C bigint + SHA1

benilla-m2
    Rust byte parser
         ↓
    C structs + bounds-checked reader
```

There isn't much advantage in finding some giant third-party C WoW library.

---

# 6. Lua is particularly easy for the C port

This is actually one place where **C becomes simpler than Rust**.

Currently:

```text
Benilla
  ↓
mlua
  ↓
lua-src
  ↓
patched Lua 5.1
```

Benilla builds a patched Lua 5.1 source tree specifically for WoW's scripting grammar. :chatgpt-content-reference{index="5"}

For C:

```text
Benilla C
   ↓
patched Lua 5.1 C API
```

That's it.

No `mlua` equivalent needed.

This would probably be one of the cleaner pieces of the port.

---

# 7. C# is also surprisingly feasible

For C#, I'd retain your GFX library as native C rather than porting it.

For example:

```text
BenillaSharp
      |
      | P/Invoke
      v
libgfx.dll / libgfx.so
      |
      +-- Vulkan
      +-- D3D11
      +-- D3D12
      +-- OpenGL
      +-- SDL
      +-- GLFW
      +-- X11
      `-- Win32
```

Your public API could expose handles like:

```c
gfx_window_t *
gfx_device_t *
gfx_buffer_t *
gfx_texture_t *
gfx_pipeline_t *
```

and C# wraps them:

```csharp
sealed class GfxTexture : SafeHandle
{
    ...
}
```

That's much preferable to implementing:

```text
Vulkan
D3D11
D3D12
OpenGL
SDL
GLFW
X11
Win32
```

again in managed code.

---

# 8. The C# equivalents are mostly in the BCL

A C# Benilla could have surprisingly few NuGets.

For example:

```text
Rust                         C#

glam                         System.Numerics
half                         System.Half
num-bigint                   System.Numerics.BigInteger
sha1                         System.Security.Cryptography
crossbeam-channel            System.Threading.Channels
rand                         System.Random /
                             RandomNumberGenerator
flate2                       System.IO.Compression
roxmltree                    System.Xml.Linq
serde                        built-in serialization /
                             manual binary parsing
libc                         P/Invoke
raw-window-handle            your GFX binding
wgpu                         your GFX binding
```

So those require **zero NuGet packages**.

Then perhaps:

```text
ImageSharp
HarfBuzzSharp
FreeTypeSharp
ImGui.NET             # dev/debug UI
```

plus whatever you pick for physics/audio.

---

# 9. Bevy is by far the biggest question

Here's how I'd categorize the migration difficulty.

| Component | Difficulty |
|---|---:|
| `wgpu` → your GFX | ★★ |
| winit/window → your GFX window API | ★ |
| glam math | ★ |
| zlib/flate2 | ★ |
| SHA1 | ★ |
| bigint/SRP | ★★ |
| file parsers | ★★ |
| XML | ★ |
| Lua | ★★ |
| audio | ★★★ |
| physics/collision | ★★★ |
| text shaping | ★★★ |
| **Bevy Assets** | ★★★ |
| **Bevy ECS/game scheduling** | ★★★★ |
| **Bevy renderer → your renderer** | ★★★★★ |

In particular Benilla currently asks Bevy for explicitly selected features including rendering, scene support, picking and UI-related APIs. :chatgpt-content-reference{index="6"}

So there's a lot beyond:

```rust
device.create_buffer(...)
```

that needs replacing.

---

# 10. I'd keep an ECS

I would **not** translate Bevy ECS usage into conventional giant C structs/classes.

Benilla clearly benefits from ECS architecture.

For C, I'd consider:

```text
Flecs
```

or a small purpose-built ECS.

Flecs is particularly interesting because the same conceptual architecture could work for both languages:

```text
C:
    Flecs

C#:
    Flecs.NET
```

Then your conceptual translation becomes:

```rust
#[derive(Component)]
struct Position { ... }

commands.spawn((
    Position(...),
    Model(...),
    Unit(...),
));
```

→ C:

```c
ecs_entity_t e = ecs_new(world);

ecs_set(world, e, Position, {...});
ecs_set(world, e, Model, {...});
ecs_add(world, e, Unit);
```

and could remain structurally similar in C#.

That would make a cross-language port much easier than abandoning ECS entirely.

---

# 11. Shaders are another significant part

Current Benilla uses **WGSL** shaders. The generated project map shows many embedded WGSL shaders for UI/world rendering. :chatgpt-content-reference{index="7"}

Your GFX has:

```text
Vulkan
D3D11
D3D12
GL4
GL3
GLES3
```

so you'd need a shader strategy.

I would strongly avoid maintaining:

```text
shader.vert.glsl
shader.frag.glsl
shader.hlsl
shader_vk.glsl
shader_gles.glsl
...
```

manually.

Instead I'd use one source language such as:

```text
HLSL
       ↓
   shader compiler
       ↓
SPIR-V       Vulkan
DXBC         D3D11
DXIL         D3D12
GLSL         OpenGL
GLSL ES      GLES3
```

This is likely one of the things you'd want to add to GFX if it doesn't already provide it.

---

# 12. Audio is another area your existing client helps

Since your other WoW client already uses **PortAudio**, you could reuse that infrastructure.

Benilla currently deliberately disables Bevy's normal audio stack and instead mixes through **Kira**; its Kira copy is patched for Benilla's spatial gain and volume interpolation behavior. :chatgpt-content-reference{index="8"}

So rather than:

```text
kira
cpal
symphonia
```

you could eventually have:

```text
WoW audio engine
      ↓
your mixer
      ↓
PortAudio
```

plus decoders for whatever WoW sound formats you need.

That is probably a better long-term fit with your existing client code. 

---

# 13. I'd structure a C port like this

Something along these lines:

```text
src/
├── app/
├── assets/
├── audio/
├── ecs/
├── game/
├── gfx/
├── input/
├── net/
├── protocol/
├── render/
│   ├── terrain.c
│   ├── m2.c
│   ├── wmo.c
│   ├── water.c
│   ├── particles.c
│   ├── sky.c
│   └── ui.c
├── ui/
│   ├── framexml.c
│   ├── lua.c
│   └── widget.c
└── wow/
    ├── adt/
    ├── blp/
    ├── dbc/
    ├── m2/
    ├── mpq/
    ├── wdt/
    └── wmo/
```

and external dependencies perhaps just:

```text
gfx             yours
zlib
lua51-benilla
freetype
harfbuzz
portaudio
physics library
XML parser
```

plus maybe a tiny image library.

That's actually a fairly lean dependency tree.

---

# 14. I'd approach the Rust → GFX → C/C# work in stages

I wouldn't simultaneously remove Bevy **and** translate languages.

I'd do:

```text
Current
Rust + Bevy + wgpu
        │
        ▼
Phase 1
Rust + Bevy ECS + your GFX
        │
        ▼
Phase 2
Rust + own engine/ECS + your GFX
        │
        ├───────────────┐
        ▼               ▼
Phase 3a            Phase 3b
C + GFX             C# + GFX
```

Phase 1 proves that your renderer abstraction can actually express everything Benilla needs.

Phase 2 is crucial because then **the Rust code no longer semantically depends on Bevy**.

At that point translation becomes much more mechanical:

```text
Vec3              → gfx_vec3 / Vector3
HashMap            → hashmap / Dictionary
Vec                → dynamic array / List
Result             → error codes / exceptions
Arc                → refcount / class reference
Channel            → queue / Channel<T>
```

instead of trying to simultaneously translate Bevy idioms into a completely different engine architecture.

## The bottom line

If your end goal is **Benilla in C or C# using your GFX library**, I wouldn't worry much about porting its dozens/hundreds of transitive Cargo dependencies. Most disappear with Bevy.

The core things you'd actually need to retain or replace are approximately:

```text
                    C                         C#
                    ─                         ──
Graphics/window     your GFX                  your GFX via P/Invoke
ECS                 Flecs/custom              Flecs.NET/custom
Math                cglm/custom               System.Numerics
Physics             Jolt/etc                  Bepu/Jolt
Lua                 patched Lua 5.1           patched Lua + P/Invoke
XML                 expat/libxml2             System.Xml
Compression         zlib                      System.IO.Compression
Big integers        GMP/LibTomMath             BigInteger
SHA1                library/custom            built-in
Images               stb/libpng                ImageSharp
Text                 FreeType+HarfBuzz         HarfBuzzSharp/etc
Audio                PortAudio + mixer         native mixer/GFX-style wrapper
Debug UI             Dear ImGui                ImGui.NET
```

**For C, I'd expect roughly 6–10 real third-party libraries. For C#, perhaps 3–6 NuGets plus your native GFX/audio/Lua libraries.** The enormous Cargo dependency graph is predominantly a consequence of Bevy/wgpu and their ecosystem rather than functionality that you would need to individually recreate.

And architecturally, Benilla is unusually suitable for this because most of the WoW-specific parsing/protocol/UI-core code has already been separated from Bevy. :chatgpt-content-reference{index="10"}
