# Pixel difference of two builds' pictures: mean and largest difference (0..255, per channel), share of pixels that
# differ, and a difference image (x8). python3 pdiff.py A B PROFILE SEED
import sys, glob, os
import numpy as np
from PIL import Image
a, b, prof, seed = sys.argv[1:5]
F = os.path.dirname(os.path.abspath(__file__)) + '/pics'
for pa in sorted(glob.glob(f'{F}/{a}-{prof}-{seed}-*.png')):
    v = pa.rsplit('-', 1)[1][:-4]
    pb = f'{F}/{b}-{prof}-{seed}-{v}.png'
    if not os.path.exists(pb):
        continue
    x = np.asarray(Image.open(pa).convert('RGB')).astype(int)
    y = np.asarray(Image.open(pb).convert('RGB')).astype(int)
    d = np.abs(x - y)
    print(f'{v:8s} mean {d.mean():.4f} max {d.max():3d} differ {100 * (d.max(axis=2) > 0).mean():.3f}%  >2: {100 * (d.max(axis=2) > 2).mean():.3f}%')
    Image.fromarray(np.clip(d * 8, 0, 255).astype(np.uint8)).save(f'{F}/diff-{a}-{b}-{prof}-{seed}-{v}.png')
