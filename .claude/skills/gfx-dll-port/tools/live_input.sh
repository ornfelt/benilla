#!/usr/bin/env bash
# Live input check of the gfx build on one backend pair. Synthetic X events go to benilla's own
# window only (xsend: XSendEvent), plus one 12 px XTEST pointer nudge that is undone. Writes
# $OUT/trace_<w>_<d>.txt (WOW_GFX_INPUT_TRACE) and $OUT/run_<w>_<d>.log; read the trace.
# Usage: OUT=<dir> live_input.sh <window-backend> <device-backend>   (from the repo root)
set -u
W=$1
D=$2
OUT=${OUT:-$(mktemp -d)}
TOOLS=$(cd "$(dirname "$0")" && pwd)
[ -x "$OUT/xsend" ] || cc "$TOOLS/xsend.c" -lX11 -o "$OUT/xsend" || exit 1
TRACE=$OUT/trace_${W}_${D}.txt
LOG=$OUT/run_${W}_${D}.log
rm -f "$TRACE"
WOW_UNATTENDED=1 WOW_NOSOUND=1 WOW_GFX_WINDOW=$W WOW_GFX_DEVICE=$D WOW_GFX_INPUT_TRACE=$TRACE \
    WOW_PROBE_EXIT_AT=25 ./target/debug/benilla >"$LOG" 2>&1 &
PID=$!
WID=
for _ in $(seq 1 300); do
    WID=$(xdotool search --pid $PID 2>/dev/null | tail -1)
    [ -n "$WID" ] && break
    sleep 0.1
done
[ -z "$WID" ] && { echo "no window for pid $PID"; wait $PID; exit 1; }
X="$OUT/xsend $WID"
sleep 4 # the boot's first frames load the UI
$X focus in
sleep 0.3
$X key a
$X key 1 shift
$X key space
$X key Return
$X hold w 400
sleep 0.3
$X motion 100 50
sleep 0.1
$X motion 200 100
sleep 0.1
$X button 1 200 100
$X button 4 200 100
$X button 5 200 100
$X button 8 200 100
sleep 0.3
xdotool mousemove_relative -- 12 5
sleep 0.2
xdotool mousemove_relative -- -12 -5
sleep 0.3
$X focus out
sleep 0.3
$X delete
wait $PID
echo "exit $? (trace $TRACE)"
