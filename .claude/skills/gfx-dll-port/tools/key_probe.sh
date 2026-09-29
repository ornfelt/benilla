#!/usr/bin/env bash
# One key into benilla's own window mid-game, and whether it opened the game menu: logs <bin> in
# as the checkout's .probe-identity account, sends <key> KEY_AT seconds after the window appears
# (default 18; xsend, so nothing else on the desktop gets it), and at 24 s logs
# `probe-log: menu=<GameMenuFrame:IsShown()>`. <key> is an xsend key: a keysym name (`Escape`)
# or `#<n>` for X keycode n (`#66`, Caps Lock, whatever the layout makes it). Extra environment
# passes through. Run from the repo root; the realm must be up.
#
#   key_probe.sh <bin> <key> <out.log>
set -uo pipefail
bin="$1"; key="$2"; log="$3"
at="${KEY_AT:-18}"
tools="$(cd "$(dirname "$0")" && pwd)"
xsend="$(dirname "$log")/xsend"
[ -x "$xsend" ] || cc "$tools/xsend.c" -lX11 -o "$xsend" || exit 1
. scripts/probe-identity.sh
probe_identity key_probe "$PWD" >/dev/null || exit 1
env WOW_UNATTENDED=1 WOW_NOSOUND=1 WOW_BG=0 \
    WOW_USER="$PROBE_USER" WOW_PASS="$PROBE_PASS" WOW_CHAR="$PROBE_CHAR" \
    WOW_PROBE_LUA='ProbeLog("menu=" .. tostring(GameMenuFrame:IsShown()))' WOW_PROBE_LUA_AT=24 \
    WOW_PROBE_EXIT_AT=28 timeout -s KILL 90 "$bin" >"$log" 2>&1 &
pid=$!  # timeout's; the window is its child's
id=""
for _ in $(seq 1 300); do
    win_pid="$(pgrep -P "$pid" | head -1)"; id="$(xdotool search --pid "${win_pid:-$pid}" 2>/dev/null | tail -1 || true)"
    [ -n "$id" ] && break
    sleep 0.1
done
if [ -n "$id" ]; then
    sleep "$at"
    "$xsend" "$id" focus in
    "$xsend" "$id" key "$key"
fi
wait "$pid"
plain="$(sed -E 's/\x1b\[[0-9;]*m//g' "$log")"
echo "key_probe: $key -> $(grep -o 'probe-log: .*' <<<"$plain" | tail -1), \
$(grep -o 'preflight: [^,]*' <<<"$plain" | head -1), $(grep -c ' ERROR ' <<<"$plain") ERROR"
