#!/usr/bin/env bash
# Draws the parity scene (`crates/benilla-gfx/examples/parity.rs`) through wgpu and through gfx on
# each named window/device pair, captures each window with ImageMagick `import` while it is up, and
# diffs every gfx capture against the wgpu one (`parity_diff.py`).
#
#   OUT=<dir> .claude/skills/gfx-dll-port/tools/parity.sh x11:gl4 x11:vk sdl:gl3
#
# Build first: `cargo build -p benilla-gfx --features gfx --example parity`.
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../.." && pwd)"
bin="$root/target/debug/examples/parity"
out="${OUT:-$(mktemp -d)}"
mkdir -p "$out"
export WOW_UNATTENDED=1 WOW_NOSOUND=1

shot() { # <label> <args...>
    local label="$1"; shift
    local log="$out/$label.log"
    "$@" >"$log" 2>&1 &
    local pid=$!
    for _ in $(seq 1 200); do
        grep -q "parity: ready" "$log" 2>/dev/null && break
        kill -0 "$pid" 2>/dev/null || break
        sleep 0.05
    done
    local id
    id="$(xdotool search --name "^benilla parity" | head -1 || true)"
    if [[ -n "$id" ]] && grep -q "parity: ready" "$log"; then
        import -window "$id" "$out/$label.png"
    else
        echo "parity.sh: $label never became ready (see $log)" >&2
    fi
    wait "$pid" || echo "parity.sh: $label exited $?" >&2
}

shot wgpu "$bin" wgpu
for pair in "$@"; do
    w="${pair%%:*}"; d="${pair#*:}"
    WOW_GFX_WINDOW="$w" WOW_GFX_DEVICE="$d" shot "gfx_${w}_${d}" "$bin" gfx
done
for pair in "$@"; do
    w="${pair%%:*}"; d="${pair#*:}"
    python3 "$(dirname "${BASH_SOURCE[0]}")/parity_diff.py" "$out/wgpu.png" "$out/gfx_${w}_${d}.png" "$out/diff_${w}_${d}.png" || true
done
echo "parity.sh: out $out"
