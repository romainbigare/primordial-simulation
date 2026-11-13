"""herb.py DIR YEAR [DIR2]: per map row: lat, kg/m2, share of cells with herb-design cover > 0.01 kg/m2, mean herb kg/m2, lead hue."""
import csv, sys, collections
def load(d, y):
    by = collections.defaultdict(list)
    for r in csv.DictReader(open(f'{d}/cells_{y:03d}.csv')):
        by[int(r['y'])].append(r)
    return by
y = int(sys.argv[2])
ds = [load(d, y) for d in [sys.argv[1]] + sys.argv[3:]]
keys = sorted(set().union(*[set(b) for b in ds]), reverse=True)
print('  lat |' + '|'.join(f'{"kg":>6} {"herbc%":>6} {"herbkg":>6} {"hue":>4} {"hcov":>5} {"tLAI":>5}' for _ in ds))
for k in keys:
    out = []
    lat = None
    for b in ds:
        rs = b.get(k, [])
        if not rs:
            out.append(' ' * 39); continue
        lat = sum(float(r['lat']) for r in rs) / len(rs)
        n = len(rs)
        kg = sum(float(r['kgm2']) for r in rs) / n
        hc = sum(1 for r in rs if float(r['herbkg']) > 0.01) / n
        hk = sum(float(r['herbkg']) for r in rs) / n
        hs = [float(r['hue']) for r in rs if r['hue'] != 'NaN']
        cov = sum(float(r.get('herbcov', 'nan')) for r in rs) / n
        tl = sum(float(r.get('treeleaf', 'nan')) for r in rs) / n
        out.append(f'{kg:6.2f} {100*hc:6.0f} {hk:6.3f} {sum(hs)/max(1,len(hs)):4.0f} {cov:5.2f} {tl:5.1f}')
    print(f'{lat:5.1f} |' + '|'.join(out))
