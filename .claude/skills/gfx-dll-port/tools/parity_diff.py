#!/usr/bin/env python3
"""Per-pixel difference of two captures: sizes, max and mean absolute channel difference, the
share of pixels off by more than 1, 4 and 16 levels, the bounding box of the >4 pixels, and an
amplified (x8) difference image."""
import sys
from PIL import Image, ImageChops

a_path, b_path, out = sys.argv[1:4]
a = Image.open(a_path).convert("RGB")
b = Image.open(b_path).convert("RGB")
print(f"{b_path}: {b.size[0]}x{b.size[1]} vs {a.size[0]}x{a.size[1]}")
if a.size != b.size:
    print("  SIZE MISMATCH")
    sys.exit(1)
d = ImageChops.difference(a, b)
px = list(d.getdata())
worst = [max(p) for p in px]
n = len(worst)
print(f"  max {max(worst)}  mean {sum(sum(p) for p in px) / (3 * n):.3f}")
for t in (1, 4, 16):
    print(f"  >{t}: {sum(1 for w in worst if w > t) / n * 100:.3f}%")
bad = d.point(lambda v: 255 if v > 4 else 0).convert("L")
print(f"  >4 bbox {bad.getbbox()}")
d.point(lambda v: min(255, v * 8)).save(out)
