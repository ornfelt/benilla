# Running benilla on the gfx library

On this branch benilla draws only through the gfx library (`libgfx.so` / `gfx.dll`), a C library
with its own window backends (x11, sdl, glfw, win32) and device backends (gl3, gl4, gles3, vk,
d3d11, d3d12). The gfx library opens the window, reads input and draws: winit is not in the build,
and Bevy's `RenderPlugin` is never added. Upstream benilla's wgpu and winit client is the
reference this build is checked against.

The code is `crates/benilla-gfx`; every build links the gfx library, so build it first.

## Building

### 1. Build the gfx library

benilla uses `gfx_benilla`, a copy of gfx with the calls benilla needs (clip-depth control,
texture copies and readback, depth bias, window modes, placement and decorations, GPU
timestamps, vk multisampling and more). It lives beside the original in the gfx repository:
`<code_root>/Code2/General/gfx/gfx_dll/gfx_benilla`, where `<code_root>` is `$code_root_dir`, else
`/home/jonas` on Linux and `C:\Users\jonas` on Windows.

Linux needs CMake, a C compiler, X11 with Xi and Xcursor, OpenGL, GLFW 3, SDL2 and the Vulkan
loader (Debian: `cmake libx11-dev libxi-dev libxcursor-dev libgl-dev libglfw3-dev libsdl2-dev
libvulkan-dev`):

```sh
cd <code_root>/Code2/General/gfx/gfx_dll/gfx_benilla
mkdir -p build && cd build
cmake .. -DCMAKE_BUILD_TYPE=Release && make -j"$(nproc)"   # -> bin/Release_x64/libgfx.so
```

Windows builds it with the MSVC toolchain, SDL2's VC development package under `C:\local`
(`CMakeLists.txt` names the versions it looks for):

```powershell
cd <code_root>\Code2\General\gfx\gfx_dll\gfx_benilla
cmake -S . -B build -A x64
cmake --build build --config Release    # -> bin\Release_x64\gfx.dll, lib\Release_x64\gfx.lib
```

`Debug` works as well (`bin/Debug_x64`); `Release` is taken when both exist.

### 2. Build benilla

```sh
WOW_DATA=/path/to/WoW/Data cargo run --release -p benilla
```

`crates/benilla-gfx/build.rs` finds the library, links it and copies it beside the binary
(`target/<profile>/`); on Linux the binary's rpath finds it there, on Windows the DLL (and
`SDL2.dll`) sits next to the exe. After rebuilding the library, rebuild benilla so the copy is
refreshed. The search:

- `GFX_DIR`: a gfx checkout, the folder holding `bin/<Configuration>_x64`.
- Else `<code_root>/Code2/General/gfx/gfx_dll/gfx_benilla` when it exists, else `.../gfx_dll/gfx`
  (the unmodified library, which lacks calls benilla makes).
- `GFX_CONFIGURATION`: `Release` or `Debug`; unset, `Release` when it is built.

The player build is `cargo build --release -p benilla --no-default-features`. With `dev`, the dev
egui panel draws through gfx as well.

### 3. Pick the backends

Two variables, read once at boot:

| Variable | Values | Unset |
|---|---|---|
| `WOW_GFX_WINDOW` | `x11`, `sdl`, `glfw`, `win32` | the first the library has, in that order: x11, win32, glfw, sdl |
| `WOW_GFX_DEVICE` | `gl4`, `gl3`, `gles3`, `vk`, `d3d11`, `d3d12` | the first the library has, in that order: gl4, gl3, d3d11, gles3, vk, d3d12 |

A name the library was not built with stops the boot with the names it has; there is no silent
fallback. The boot log names the pair it opened (`gfx: ...`).

```sh
WOW_GFX_WINDOW=sdl WOW_GFX_DEVICE=vk cargo run --release -p benilla
```

### Supported pairs

| Platform | Window | Device | State |
|---|---|---|---|
| Linux (X11) | x11, sdl, glfw | gl3, gl4, gles3, vk | built and run: every pair through the capture scenarios (the login screen, character creation, the Northshire UI scenes), vk and gl4 through every capture scenario, renderScale 0.5 and 2 and the worldview against the wgpu build; no live server login yet |
| Windows | win32, sdl, glfw | gl3, gl4, gles3, vk; d3d11, d3d12 on win32 | written, not built yet |
| macOS | | | not built: gfx has no Metal device |

## Shaders

The shaders are `.gfxs` sources under `crates/benilla-gfx/shaders/src`, compiled into four
families the loader picks per device: `shaders/` (gl3, gl4, gles3), `shaders_vk/` (SPIR-V),
`shaders_d3/` (HLSL for d3d11 and d3d12) and `shaders_gles3_dark/`. The compiled families are
committed and embedded in the binary, so a build needs no shader compiler. After editing a source,
recompile with `wc_compiler_rs` (`cargo build --release` in
`<code_root>/Code2/General/gfx/wc_compiler_rs`, or `GFX_SHADER_COMPILER=<path>`):

```sh
crates/benilla-gfx/shaders/compile.sh          # every shader
crates/benilla-gfx/shaders/compile.sh terrain  # one base name
```

`WOW_GFX_SHADERS=crates/benilla-gfx/shaders` reads the families from disk instead of the embedded
copies, to iterate without a rebuild.

## Other switches

- `WOW_GFX_INPUT_TRACE=<path>`: one line per input message sent to Bevy and per call made to the
  window.
- `WOW_GFX_DEPTH_REMAP=1`: GL keeps clip depth in [-1, 1] and remaps per vertex, and draws its
  targets bottom-up, as it does on a GL without clip control; for measuring that path.
- The instruments work as on the wgpu client: `WOW_CAPTURE`, `WOW_DEPTH`, `WOW_PHASE`, `WOW_GPU_MS`,
  screenshots.

## Known differences from the wgpu path

- A GL without clip control draws its targets bottom-up, so its multisample pattern is mirrored
  against wgpu's on diagonal edges (under 1% of a 4x frame).
- Mesa's radeonsi GL rounds the UI target's 8-bit sRGB stores one step differently from RADV and
  wgpu on about 5% of bright values: 1-2 levels over about 2.6% of a frame where the UI lane
  carries the world.
- On GL, about 100 isolated pixels of a world frame (0.01%) take the other side of a foliage
  alpha-test edge (leaf against sky) from wgpu and vk.
- A tiling window manager: winit runs frames at the asked size before the tile arrives, while a
  gfx window is shown at its tile. benilla re-lays out the interface on a size change, so a
  layout that depends on that late pass (the chat dock over the bottom-left action bar) can end
  in a different place, and with it the frames anchored above the bottom bars (bags, the default
  tooltip).
