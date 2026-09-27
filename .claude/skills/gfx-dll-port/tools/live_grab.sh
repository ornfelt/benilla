#!/usr/bin/env bash
# Live grab check on the engine viewer, no data needed: a right-drag asks for CursorGrabMode::Locked
# and a hidden cursor (benilla-world worldview::fly), so the trace shows the gfx grab, the raw motion
# under it (a 20 px XTEST nudge, undone) and the release. The real pointer is held by the window
# for about half a second and put back at the drag start. Build with
# `cargo build -p benilla-worldview --features benilla-worldview/gfx` first.
# Usage: OUT=<dir> live_grab.sh <window-backend> <device-backend>   (from the repo root)
set -u
W=$1
D=$2
OUT=${OUT:-$(mktemp -d)}
TOOLS=$(cd "$(dirname "$0")" && pwd)
[ -x "$OUT/xsend" ] || cc "$TOOLS/xsend.c" -lX11 -o "$OUT/xsend" || exit 1
TRACE=$OUT/grab_${W}_${D}.txt
LOG=$OUT/grab_${W}_${D}.log
rm -f "$TRACE"
WOW_UNATTENDED=1 WOW_NOSOUND=1 WOW_DATA= WOW_GFX_WINDOW=$W WOW_GFX_DEVICE=$D \
    WOW_GFX_INPUT_TRACE=$TRACE WOW_WORLDVIEW_CHECK=20 ./target/debug/benilla-worldview >"$LOG" 2>&1 &
PID=$!
WID=
for _ in $(seq 1 300); do
    WID=$(xdotool search --pid $PID 2>/dev/null | tail -1)
    [ -n "$WID" ] && break
    sleep 0.1
done
[ -z "$WID" ] && { echo "no window"; wait $PID; exit 1; }
X="$OUT/xsend $WID"
sleep 4
$X focus in
$X motion 300 200
sleep 0.2
$X down 3 300 200
sleep 0.2
xdotool mousemove_relative -- 20 0
sleep 0.15
xdotool mousemove_relative -- -20 0
sleep 0.15
$X up 3 300 200
sleep 0.5
$X delete
wait $PID
echo "exit $? (trace $TRACE)"
