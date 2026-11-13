# Compare the key probe's land cells of two runs: island means, and the cover by classes of wetness and of height.
import csv, sys
def load(p): return [r for r in csv.DictReader(open(p)) if r['land'] == '1' and float(r['depth']) <= 1.0]
a, b = load(sys.argv[1]), load(sys.argv[2])
names = sys.argv[3:5] if len(sys.argv) > 4 else ['A', 'B']
def mean(rows, k): return sum(float(r[k]) for r in rows) / len(rows)
print(f'land cells {len(a)} / {len(b)}')
for k in ['light', 'wet', 'tmin', 'tmean', 'rain', 'evap', 'kg', 'leaf']:
    print(f'{k:>6}: {names[0]} {mean(a,k):10.4g}  {names[1]} {mean(b,k):10.4g}  ratio {mean(b,k)/max(mean(a,k),1e-30):.3f}')
def classes(key, edges):
    print(f'cover kg/m2 by {key} (of {names[0]}\'s cells; same cells in both: the same island)')
    ia = {r['i']: r for r in a}; ib = {r['i']: r for r in b}
    common = [i for i in ia if i in ib]
    for lo, hi in zip(edges, edges[1:]):
        cs = [i for i in common if lo <= float(ia[i][key]) < hi]
        if not cs: continue
        ma = sum(float(ia[i]['kg']) for i in cs) / len(cs); mb = sum(float(ib[i]['kg']) for i in cs) / len(cs)
        wa = sum(float(ia[i]['wet']) for i in cs) / len(cs); wb = sum(float(ib[i]['wet']) for i in cs) / len(cs)
        la = sum(float(ia[i]['light']) for i in cs) / len(cs); lb = sum(float(ib[i]['light']) for i in cs) / len(cs)
        print(f'  {key} {lo:>7.3g}..{hi:<7.3g} cells {len(cs):4d}: kg {ma:6.3f} -> {mb:6.3f} ({mb/max(ma,1e-9):.2f}x); wet {wa:.3f} -> {wb:.3f}; light {la:6.1f} -> {lb:6.1f}')
classes('wet', [0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.7, 1.01])
zs = sorted(float(r['z']) for r in a); q = [zs[int(len(zs) * f)] for f in (0, .2, .4, .6, .8)] + [zs[-1] + 1]
classes('z', q)
