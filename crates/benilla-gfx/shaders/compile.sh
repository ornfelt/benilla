#!/usr/bin/env bash
# Recompiles every `.gfxs` source under `src/` into the four shader families the loader picks
# from at runtime (`crates/benilla-gfx/src/shader_loader.rs`):
#   shaders/            gl3, gl4, gles3
#   shaders_vk/         vk (SPIR-V, through glslangValidator)
#   shaders_d3/         d3d11 (HLSL; d3d12 reads the same blob)
#   shaders_gles3_dark/ gles3 (the native-window gles3 family off Linux)
#
# The compiler is `wc_compiler_rs`: `$GFX_SHADER_COMPILER`, else the release or debug build under
# `$code_root_dir/Code2/General/gfx/wc_compiler_rs` (`cargo build --release` there).
#
#   crates/benilla-gfx/shaders/compile.sh          # all
#   crates/benilla-gfx/shaders/compile.sh blit     # one base name
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="${code_root_dir:-$HOME}"

compiler="${GFX_SHADER_COMPILER:-}"
if [[ -z "$compiler" ]]; then
    for c in "$root/Code2/General/gfx/wc_compiler_rs/target/release/wc_compiler_rs" \
             "$root/Code2/General/gfx/wc_compiler_rs/target/debug/wc_compiler_rs"; do
        if [[ -x "$c" ]]; then compiler="$c"; break; fi
    done
fi
if [[ -z "$compiler" || ! -x "$compiler" ]]; then
    echo "compile.sh: wc_compiler_rs not found; set GFX_SHADER_COMPILER" >&2
    exit 1
fi

families=("shaders:gl3,gl4,gles3" "shaders_vk:vk" "shaders_d3:d3d11" "shaders_gles3_dark:gles3")

shopt -s nullglob
names=()
if (( $# > 0 )); then
    names=("$@")
else
    for f in "$here"/src/*.vs.gfxs; do
        b="$(basename "$f")"
        names+=("${b%.vs.gfxs}")
    done
fi

failed=0
for family in "${families[@]}"; do
    dir="$here/${family%%:*}"
    backends="${family#*:}"
    mkdir -p "$dir"
    for name in "${names[@]}"; do
        for stage in vs fs; do
            src="$here/src/$name.$stage.gfxs"
            if [[ ! -f "$src" ]]; then
                echo "compile.sh: missing $src" >&2
                failed=1
                continue
            fi
            if ! "$compiler" -t "$stage" -x "$backends" -i "$src" -o "$dir/$name.$stage.gfx" >/dev/null; then
                echo "compile.sh: FAILED $name.$stage ($backends)" >&2
                failed=1
            fi
        done
    done
    echo "compile.sh: ${family%%:*} ($backends): ${#names[@]} shader(s)"
done
exit "$failed"
