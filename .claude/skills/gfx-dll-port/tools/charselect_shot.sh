#!/usr/bin/env bash
# One shot of the character screen: logs <bin> in as the checkout's .probe-identity account with no
# character picked, waits for the roster line, then SETTLE seconds (default 8) for the model and
# the scene, imports the window and stops the client. Extra environment passes through. Run from
# the repo root; the realm must be up.
#
#   SETTLE=8 charselect_shot.sh <bin> <out.png>
set -uo pipefail
bin="$1"; out="$2"
settle="${SETTLE:-8}"
. scripts/probe-identity.sh
probe_identity charselect_shot "$PWD" >/dev/null || exit 1
log="${out%.png}.log"
mkdir -p "$(dirname "$out")"
pkill -x import 2>/dev/null
env WOW_UNATTENDED=1 WOW_NOSOUND=1 WOW_BG=0 WOW_USER="$PROBE_USER" WOW_PASS="$PROBE_PASS" \
    "$bin" >"$log" 2>&1 &
pid=$!
id=""
for _ in $(seq 1 300); do
    id="$(xdotool search --pid "$pid" 2>/dev/null | tail -1 || true)"
    [[ -n "$id" ]] && break
    sleep 0.1
done
seen=""
for _ in $(seq 1 600); do
    grep -q "character select — " "$log" 2>/dev/null && { seen=1; break; }
    kill -0 "$pid" 2>/dev/null || break
    sleep 0.1
done
if [[ -n "$id" && -n "$seen" ]] && kill -0 "$pid" 2>/dev/null; then
    sleep "$settle"
    timeout 20 import -window "$id" "$out" || echo "charselect_shot: capture failed" >&2
else
    echo "charselect_shot: no window or no roster (see $log)" >&2
fi
kill "$pid" 2>/dev/null
sleep 2
kill -9 "$pid" 2>/dev/null
wait "$pid" 2>/dev/null
plain="$(sed -E 's/\x1b\[[0-9;]*m//g' "$log")"
echo "charselect_shot: $(grep -c ' ERROR ' <<<"$plain") ERROR, $(grep -c 'panicked at' <<<"$plain") panic, \
$(grep -o 'character select — [^)]*)' <<<"$plain" | head -1), shot $([ -f "$out" ] && echo yes || echo no)"
