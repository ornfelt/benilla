#!/usr/bin/env python3
"""Diffs two captures only where the second one drew something: its pixels whose every channel is
below `--white` (default 250) and whose `--erode` neighbourhood is too, so a blended edge against a
backdrop only one path draws (the gfx path's unported white terrain) stays out.

    masked_diff.py <a.png> <b.png> [--white 250] [--erode 1] [--out diff.png]

Prints the masked share of the frame, the max, mean and percentiles of the per-pixel max channel
difference, and the share over 1, 4 and 16 levels; `--out` writes the x8 diff with the mask."""
import argparse
import numpy as np
from PIL import Image

p = argparse.ArgumentParser()
p.add_argument("a")
p.add_argument("b")
p.add_argument("--white", type=int, default=250)
# Pixels to erode the mask by: past the glow's reach (the quarter-res Gauss4 x2, ~16 px), a
# backdrop only one path draws no longer bleeds in.
p.add_argument("--erode", type=int, default=1)
p.add_argument("--out")
args = p.parse_args()
a = np.asarray(Image.open(args.a).convert("RGB")).astype(np.int16)
b = np.asarray(Image.open(args.b).convert("RGB")).astype(np.int16)
drawn = (b < args.white).any(axis=2)
# Erode: a pixel counts only if every pixel within `--erode` (Chebyshev) is drawn too.
m = drawn.copy()
for _ in range(args.erode):
    step = m.copy()
    for dy in (-1, 0, 1):
        for dx in (-1, 0, 1):
            step &= np.roll(np.roll(m, dy, 0), dx, 1)
    m = step
d = np.abs(a - b).max(axis=2)
v = d[m]
print(f"{args.b}: mask {100 * m.mean():.2f}% of {m.size} px")
if v.size:
    print(f"  max {v.max()}  mean {v.mean():.3f}  p50 {np.percentile(v, 50):.0f}  "
          f"p90 {np.percentile(v, 90):.0f}  p99 {np.percentile(v, 99):.0f}")
    for t in (1, 4, 16):
        print(f"  >{t}: {100 * (v > t).mean():.3f}%")
if args.out:
    img = np.where(m[..., None], np.clip(np.abs(a - b) * 8, 0, 255), 64).astype(np.uint8)
    Image.fromarray(img).save(args.out)
