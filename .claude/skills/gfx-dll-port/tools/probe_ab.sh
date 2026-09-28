#!/usr/bin/env bash
# Runs one capture scenario through the wgpu build and the gfx build on each window:device pair,
# with the probe variables the caller exports (WOW_DEPTH, WOW_PHASE, ...), and keeps each run's
# log and its lines matching $KEEP (default: the depth and phase probes') for a side-by-side diff.
#
# DIFF_ONLY=1 re-diffs the logs in <dir>; REUSE_WGPU=1 keeps a wgpu.log already there; SHOW=<n> prints the first n diff lines per pair.
#
#   OUT=<dir> SCENARIO=ui-bag WOW_DEPTH="500,500;100,100" \
#     probe_ab.sh <wgpu-bin> <gfx-bin> x11:vk x11:gl4 ...
#
# A pair spec `lvp:vk` runs vk on lavapipe with the validation layer, `soft:gl3` GL on llvmpipe.
set -u
wgpu=$1 gfx=$2
shift 2
out=${OUT:?OUT=<dir>}
scenario=${SCENARIO:?SCENARIO=<capture scenario>}
keep=${KEEP:-'depth#|depth:|phase#|phase:'}
mkdir -p "$out"
export WOW_UNATTENDED=1 WOW_NOSOUND=1 WOW_CAPTURE="$scenario"

run() { # name, binary, extra env...
  local name=$1 bin=$2
  shift 2
  pkill -x import 2>/dev/null
  env WOW_CAPTURE_OUT="$out/$name.png" "$@" timeout -s KILL 240 "$bin" >"$out/$name.log" 2>&1
  echo "$name: exit $?"
  norm "$out/$name.log" >"$out/$name.probe"
}

# The probe lines without colour, timestamp and module; entity ids (allocation order differs
# run to run) as `E`.
norm() {
  sed -E 's/\x1b\[[0-9;]*m//g' "$1" | grep -E "$keep" |
    sed -E 's/^.*(depth_probe|phase_probe)(::gfx)?: //; s/MainEntity\(([0-9]+v[0-9]+)\)/\1/g; s/\b[0-9]+v[0-9]+\b/E/g'
}

# DIFF_ONLY=1 re-reads the logs already in $out instead of running.
if [ "${DIFF_ONLY:-}" ]; then
  set --
  for l in "$out"/*.log; do norm "$l" >"${l%.log}.probe"; done
elif [ -s "$out/wgpu.log" ] && [ "${REUSE_WGPU:-}" ]; then
  norm "$out/wgpu.log" >"$out/wgpu.probe"
else
  run wgpu "$wgpu"
fi
for spec in "$@"; do
  window=${spec%%:*} device=${spec#*:}
  case $window in
    lvp) run "lvp-$device" "$gfx" WOW_GFX_WINDOW=x11 WOW_GFX_DEVICE="$device" \
           VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json \
           VK_INSTANCE_LAYERS=VK_LAYER_KHRONOS_validation ;;
    soft) run "soft-$device" "$gfx" WOW_GFX_WINDOW=x11 WOW_GFX_DEVICE="$device" \
            LIBGL_ALWAYS_SOFTWARE=1 ;;
    *) run "$window-$device" "$gfx" WOW_GFX_WINDOW="$window" WOW_GFX_DEVICE="$device" ;;
  esac
done
for f in "$out"/*.probe; do
  [ "$f" = "$out/wgpu.probe" ] && continue
  echo "== $(basename "$f" .probe) vs wgpu: $(wc -l <"$f") lines, $(diff "$out/wgpu.probe" "$f" | grep -c '^>') differ"
  diff "$out/wgpu.probe" "$f" | head -"${SHOW:-0}"
done
