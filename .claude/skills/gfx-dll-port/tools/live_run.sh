#!/usr/bin/env bash
# One live login through <bin> as the checkout's .probe-identity account: a WOW_LIVE_SHOT at AT
# seconds (default 25), exit at EXIT seconds (default AT+5), the log beside the PNG. Extra
# environment (WOW_GFX_WINDOW, WOW_GFX_DEVICE, WOW_BOOTH_LOG, ...) passes through. Run from the
# repo root; the realm must be up.
#
#   AT=25 live_run.sh <bin> <out.png>
set -uo pipefail
bin="$1"; out="$2"
at="${AT:-25}"
exit_at="${EXIT:-$((at + 5))}"
. scripts/probe-identity.sh
probe_identity live_run "$PWD" >/dev/null || exit 1
log="${out%.png}.log"
mkdir -p "$(dirname "$out")"
env WOW_UNATTENDED=1 WOW_NOSOUND=1 WOW_BG=0 \
    WOW_USER="$PROBE_USER" WOW_PASS="$PROBE_PASS" WOW_CHAR="$PROBE_CHAR" \
    WOW_LIVE_SHOT="$out" WOW_LIVE_SHOT_AT="$at" WOW_PROBE_EXIT_AT="$exit_at" \
    timeout -s KILL $((exit_at + 60)) "$bin" >"$log" 2>&1
code=$?
plain="$(sed -E 's/\x1b\[[0-9;]*m//g' "$log")"
echo "live_run: exit $code, $(grep -c ' ERROR ' <<<"$plain") ERROR, $(grep -c ' WARN ' <<<"$plain") WARN, \
$(grep -c 'panicked at' <<<"$plain") panic, $(grep -o 'preflight: [^,]*' <<<"$plain" | head -1 || echo 'NO preflight'), \
shot $([ -f "$out" ] && echo yes || echo no)"
