#!/usr/bin/env bash
# type_run.sh <bin> <out-prefix> [env...]: a ui-bag capture that types
# `/console renderScale 1` at ~6 s and `/console renderScale 2` at ~9 s into its own window only
# (xsend: XSendEvent to that window, nothing else on the desktop). The keys are this machine's
# Swedish layout (`/` is shift+7); give both builds the same WOW_CAPTURE_AGE (1200) so the shot
# comes after the lines at one sim age. Edit `line` for another command.
set -u
S=$(cd "$(dirname "$0")" && pwd)
[ -x "$S/xsend" ] || cc "$S/xsend.c" -lX11 -o "$S/xsend" || exit 1
bin=$1 out=$2; shift 2
env WOW_UNATTENDED=1 WOW_NOSOUND=1 WOW_CAPTURE=ui-bag WOW_CAPTURE_OUT="$out.png" "$@" timeout -s KILL 240 "$bin" >"$out.log" 2>&1 &
pid=$!
wid=
for _ in $(seq 1 600); do wid=$(xdotool search --name '^benilla$' 2>/dev/null | tail -1); [ -n "$wid" ] && break; sleep 0.1; done
[ -z "$wid" ] && { echo "no window"; wait $pid; exit 1; }
X="$S/xsend $wid"
type_line() {
  wid=$(xdotool search --onlyvisible --name '^benilla$' 2>/dev/null | tail -1); X="$S/xsend $wid"
  $X key Return; sleep 0.3
  for k in "$@"; do case $k in SLASH) $X key 7 shift ;; [A-Z]) $X key "${k,,}" shift ;; *) $X key "$k" ;; esac; sleep 0.03; done
  sleep 0.2; $X key Return
}
line() { local ks=(SLASH c o n s o l e space r e n d e r S c a l e space "$1"); type_line "${ks[@]}"; }
sleep 6; $X focus in; sleep 0.3
line 1; sleep 3; line 2
wait $pid; echo "exit $?"
