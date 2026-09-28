#!/usr/bin/env bash
# scripts/smoke.sh on the gfx build: the same script with every `cargo build|run -p benilla` given
# `--features gfx`, run from the repo root. smoke.sh unsets inherited WOW_* variables, so the
# pair is the platform default (WOW_GFX_WINDOW / WOW_GFX_DEVICE unset). WOW_SMOKE_KEEP=1 keeps
# the logs.
#
#   smoke_gfx.sh
set -uo pipefail
root="$(git rev-parse --show-toplevel)" || exit 1
tmp="$(mktemp "${TMPDIR:-/tmp}/smoke-gfx.XXXXXX.sh")"
trap 'rm -f "$tmp"' EXIT
sed -E 's/cargo (build|run) -q -p benilla /cargo \1 -q -p benilla --features gfx /' \
    "$root/scripts/smoke.sh" >"$tmp"
[ "$(grep -c -- '--features gfx' "$tmp")" -eq 3 ] || { echo "smoke_gfx: smoke.sh changed shape"; exit 1; }
cd "$root" && CARGO_INCREMENTAL=0 bash "$tmp"
