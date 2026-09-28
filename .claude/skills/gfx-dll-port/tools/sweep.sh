#!/usr/bin/env bash
# The capture sweep: every scenario in $SCEN (default: all of `WOW_CAPTURE=list` and the ui-*
# fixtures) through probe_ab.sh into target/ab/sweep/<scenario>, one line per pair: the diff
# against wgpu and, masked to outside the chat dock (rectmask.py), again; plus the count of
# validation / GL / panic lines. Never run it beside another windowed capture: the tiling WM
# splits the screen and the sizes stop matching.
#
#   WGPU=target/ab/wgpu/benilla GFX=target/ab/gfx/benilla sweep.sh lvp:vk x11:vk x11:gl4
set -u
T=$(cd "$(dirname "$0")" && pwd)
wgpu=${WGPU:-target/ab/wgpu/benilla} gfx=${GFX:-target/ab/gfx/benilla}
if [ -z "${SCEN:-}" ]; then
  SCEN="$(WOW_CAPTURE=list WOW_UNATTENDED=1 WOW_NOSOUND=1 timeout 60 "$wgpu" 2>/dev/null | tr '\n' ' ')"
  SCEN="$SCEN glue-login glue-charcreate glue-realmlist $(grep -oE '"ui-[a-z-]+"' crates/benilla-app/src/capture/scenarios.rs | tr -d '"' | sort -u | tr '\n' ' ')"
fi
for s in $SCEN; do
  d=target/ab/sweep/$s; mkdir -p "$d"
  OUT=$d SCENARIO=$s KEEP=zzzz REUSE_WGPU=1 "$T/probe_ab.sh" "$wgpu" "$gfx" "$@" >/dev/null 2>&1
  for p in "$@"; do
    n=${p/:/-}
    r=$(python3 "$T/parity_diff.py" "$d/wgpu.png" "$d/$n.png" "$d/diff-$n.png" 2>&1 | sed -n 3,5p | tr -s ' \n' ' ')
    m=$(python3 "$T/rectmask.py" "$d/wgpu.png" "$d/$n.png" 0,680,600,960 2>&1 | sed -E 's/ bbox.*//')
    v=$(sed -E 's/\x1b\[[0-9;]*m//g' "$d/$n.log" 2>/dev/null | grep -cE "Validation Error|VUID|GL_INVALID|panicked")
    echo "$s $n |$r| $m | errs $v"
  done
done
