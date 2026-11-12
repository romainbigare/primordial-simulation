# Work per pixel from the count pictures: texture reads and random words per map pixel, land + cover, by view.
# python3 summ.py BUILD PROFILE SEED
import sys, os, glob
import numpy as np
from PIL import Image
F = os.path.dirname(os.path.abspath(__file__)) + '/' + os.environ.get('CD', 'counts2')
b, prof, seed = sys.argv[1], sys.argv[2], sys.argv[3]
def load(p):
    a = np.asarray(Image.open(p).convert('RGBA')).astype(np.int64)
    return a[..., 0] + 256 * (a[..., 2] % 16), a[..., 1] + 256 * (a[..., 2] // 16)
rows = {}
for lp in sorted(glob.glob(f'{F}/{b}-{prof}-{seed}-land-*.png')):
    view = lp.rsplit('-', 1)[1][:-4]
    r, w = load(lp)
    m = (r > 0) & (r < 1000)
    cp = lp.replace('-land-', '-cover-')
    cr, cw = load(cp) if os.path.exists(cp) else (r * 0, w * 0)
    cr = np.where(cr < 4000, cr, 0)
    rows[view] = (r[m].mean(), w[m].mean(), cr[m].mean(), cw[m].mean(), np.percentile(r[m] + cr[m], 99), m.sum())
zooms = {}
for v, x in rows.items():
    z = v.rstrip('ab')
    zooms.setdefault(z, []).append(x)
order = ['fit', 'k2', 'k1', 'm200', 'm50', 'm10']
print(f'{b} {prof} seed {seed}: per map pixel, land reads / words, cover reads / words, total reads / words, p99 reads')
for z in order:
    if z not in zooms: continue
    a = np.array(zooms[z]).mean(axis=0)
    print(f'{z:5s} land {a[0]:6.1f} {a[1]:6.1f}  cover {a[2]:6.1f} {a[3]:6.1f}  total {a[0]+a[2]:6.1f} {a[1]+a[3]:6.1f}  p99 {a[4]:6.0f}')
