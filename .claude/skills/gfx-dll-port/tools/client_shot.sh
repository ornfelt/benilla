#!/usr/bin/env bash
# One shot of a full-client window: runs <bin> (a capture scenario through WOW_CAPTURE), imports its
# window when the log first prints ON (default "capture: scene aged", the moment the wgpu build
# writes its own capture) or AT seconds in (default 60), then stops it. The gfx build has no
# read-back, so its capture harness never writes a PNG; this is how its frame is taken.
#
#   ON="capture: scene aged" AT=60 client_shot.sh <bin> <out.png>
set -uo pipefail
bin="$1"; out="$2"
at="${AT:-60}"
on="${ON:-capture: scene aged}"
export WOW_UNATTENDED=1 WOW_NOSOUND=1 WOW_BG=0
log="${out%.png}.log"
"$bin" >"$log" 2>&1 &
pid=$!
id=""
for _ in $(seq 1 300); do
    id="$(xdotool search --pid "$pid" 2>/dev/null | tail -1 || true)"
    [[ -n "$id" ]] && break
    sleep 0.1
done
if [[ -z "$id" ]]; then
    echo "client_shot.sh: no window (see $log)" >&2
else
    for _ in $(seq 1 $((at * 10))); do
        grep -q "$on" "$log" 2>/dev/null && break
        kill -0 "$pid" 2>/dev/null || break
        sleep 0.1
    done
    if kill -0 "$pid" 2>/dev/null; then
        timeout 20 import -window "$id" "$out" || echo "client_shot.sh: capture failed" >&2
    else
        echo "client_shot.sh: the client exited before the shot (see $log)" >&2
    fi
fi
kill "$pid" 2>/dev/null
sleep 2
kill -9 "$pid" 2>/dev/null
wait "$pid" 2>/dev/null
exit 0
