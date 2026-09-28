#!/usr/bin/env python3
import sys, numpy as np
from PIL import Image
# rectmask.py a.png b.png x0,y0,x1,y1: the diff shares outside a rect (the chat dock a late WM
# resize moves on wgpu, 0,680,600,960 in a 1019x1014 capture) and the bbox of what is left >16.
a=np.asarray(Image.open(sys.argv[1]).convert('RGB')).astype(int); b=np.asarray(Image.open(sys.argv[2]).convert('RGB')).astype(int)
x0,y0,x1,y1=map(int,sys.argv[3].split(','))
d=np.abs(a-b).max(axis=2); m=np.ones(d.shape,bool); m[y0:y1,x0:x1]=False
v=d[m]
print(f"outside {100*m.mean():.1f}% of frame: >1 {100*(v>1).mean():.3f}% >4 {100*(v>4).mean():.3f}% >16 {100*(v>16).mean():.3f}%", end=' ')
ys,xs=np.nonzero((d>16)&m); print("bbox>16", (xs.min(),ys.min(),xs.max(),ys.max()) if len(xs) else None)
