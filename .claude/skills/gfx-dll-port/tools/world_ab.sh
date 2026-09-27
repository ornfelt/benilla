#!/usr/bin/env bash
# The live world A/B: `benilla-worldview` over the install, one fixed view (the default Northshire
# overview, or WOW_WORLDVIEW_AT), noon pinned (WOW_CLOCK, default 720), through a wgpu build and a gfx build
# on each named pair; each window is captured with ImageMagick `import` AT seconds in (default 30)
# and every gfx shot is diffed against the wgpu one (`parity_diff.py`).
#
#   OUT=<dir> AT=30 world_ab.sh <wgpu-bin> <gfx-bin> x11:gl4 [x11:vk ...]
#
# The two binaries are the same crate built without and with `--features gfx`; copy each out of
# `target/debug` after building, since both builds write the same path.
set -euo pipefail
tools="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
wgpu_bin="$1"; gfx_bin="$2"; shift 2
out="${OUT:-$(mktemp -d)}"
at="${AT:-30}"
mkdir -p "$out"
# WOW_BG=0: on top and focused, so `import` reads the window and not what covers it.
export WOW_UNATTENDED=1 WOW_NOSOUND=1 WOW_BG=0 WOW_CLOCK="${WOW_CLOCK:-720}" WOW_WIN=1280x720
export WOW_WORLDVIEW_CHECK=$((at + 4))

shot() { # <label> <bin>
    local label="$1" bin="$2"
    local log="$out/$label.log"
    "$bin" >"$log" 2>&1 &
    local pid=$!
    local id=""
    for _ in $(seq 1 300); do
        id="$(xdotool search --pid "$pid" 2>/dev/null | tail -1 || true)"
        [[ -n "$id" ]] && break
        sleep 0.1
    done
    if [[ -z "$id" ]]; then
        echo "world_ab.sh: $label opened no window (see $log)" >&2
    else
        sleep "$at"
        import -window "$id" "$out/$label.png" || echo "world_ab.sh: $label capture failed" >&2
    fi
    wait "$pid" || echo "world_ab.sh: $label exited $?" >&2
}

shot wgpu "$wgpu_bin"
for pair in "$@"; do
    w="${pair%%:*}"; d="${pair#*:}"
    WOW_GFX_WINDOW="$w" WOW_GFX_DEVICE="$d" shot "gfx_${w}_${d}" "$gfx_bin"
done
for pair in "$@"; do
    w="${pair%%:*}"; d="${pair#*:}"
    python3 "$tools/parity_diff.py" "$out/wgpu.png" "$out/gfx_${w}_${d}.png" "$out/diff_${w}_${d}.png" || true
done
echo "world_ab.sh: out $out"
