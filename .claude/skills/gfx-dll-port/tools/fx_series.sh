#!/usr/bin/env bash
# The effect-lane coverage series: `benilla-worldview` with every effect fragment solid magenta
# (`WOW_PARTICLE_FLAT`) and depth-always (`WOW_PARTICLE_NODEPTH`), five window shots 2 s apart from
# 26 s in, and the magenta pixel count and blob widths of each. Particles are not deterministic
# (seeded, but stepped on wall-clock frames), so a backend matches when its counts fall inside
# wgpu's own run-to-run spread.
#
#   OUT=<dir> fx_series.sh <label> <bin>     # WOW_GFX_WINDOW / WOW_GFX_DEVICE pass through
set -u
label="$1"; bin="$2"
out="${OUT:-$(mktemp -d)}"
mkdir -p "$out"
export WOW_UNATTENDED=1 WOW_NOSOUND=1 WOW_BG=0 WOW_CLOCK="${WOW_CLOCK:-720}" WOW_WIN=1280x720
export WOW_CAPTURE=1 WOW_PARTICLE_FLAT=1 WOW_PARTICLE_NODEPTH=1 WOW_WORLDVIEW_CHECK=40
"$bin" > "$out/$label.log" 2>&1 &
pid=$!
id=""
for _ in $(seq 1 300); do
    id="$(xdotool search --pid "$pid" 2>/dev/null | tail -1 || true)"
    [[ -n "$id" ]] && break
    sleep 0.1
done
sleep 26
for k in 1 2 3 4 5; do import -window "$id" "$out/${label}_$k.png"; sleep 2; done
wait "$pid"
python3 - "$out" "$label" <<'PY'
import sys
import numpy as np
from PIL import Image
from scipy.ndimage import label, find_objects
out, name = sys.argv[1], sys.argv[2]
counts = []
for k in range(1, 6):
    a = np.asarray(Image.open(f"{out}/{name}_{k}.png").convert("RGB")).astype(int)
    mag = (a[..., 0] > 200) & (a[..., 1] < 60) & (a[..., 2] > 200)
    lab, _ = label(mag)
    widths = sorted((s[1].start, s[1].stop - s[1].start)
                    for j, s in enumerate(find_objects(lab)) if (lab[s] == j + 1).sum() >= 4)
    counts.append(int(mag.sum()))
    print(f"{name} shot {k}: {mag.sum()} px, widths {[w for _, w in widths]}")
print(f"{name}: mean {np.mean(counts):.0f} px, range {min(counts)}..{max(counts)}")
PY
