#!/usr/bin/env bash
# What the window manager made of the parity window under one PARITY_WINDOW spec: its geometry,
# its Motif hints and its WM_NORMAL_HINTS, sampled <delay> s in. Run it once per path and compare.
#
#   tools/window_props.sh <spec> <delay> <label> target/debug/examples/parity wgpu
#   WOW_GFX_WINDOW=x11 WOW_GFX_DEVICE=vk tools/window_props.sh "nodeco,fixed,at=300:120" 1.2 x11 \
#       target/debug/examples/parity gfx
#
# The example loads `$ORIGIN/../libgfx.so` (target/debug/libgfx.so): copy a rebuilt gfx_benilla
# there first. zsh does not word-split a variable holding a command: call this script directly.
spec=$1; delay=$2; label=$3; shift 3
PARITY_WINDOW="$spec" WOW_UNATTENDED=1 WOW_NOSOUND=1 "$@" >/dev/null 2>&1 &
pid=$!
sleep "$delay"
id=$(xdotool search --name "^benilla parity" | head -1)
geo=$(xwininfo -id "$id" | awk '/Absolute upper-left X/{x=$4}/Absolute upper-left Y/{y=$4}/Width:/{w=$2}/Height:/{h=$2}END{printf "%sx%s+%s+%s", w, h, x, y}')
motif=$(xprop -id "$id" _MOTIF_WM_HINTS | sed 's/.*= //')
hints=$(xprop -id "$id" WM_NORMAL_HINTS | grep -E "location|minimum|maximum|gravity" | sed 's/^\s*//' | tr '\n' ';')
printf '%-24s %-10s geo %-18s motif [%s] hints [%s]\n' "$spec" "$label" "$geo" "$motif" "$hints"
wait $pid
