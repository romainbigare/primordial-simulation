"""cmp.py MAIN_DIR SORTED_DIR: per year, the rows table of both runs side by side (lat; kg/m2, lead kind and share,
kinds over 1% of the row's cover, dry matter a year kg/m2, frost gene (cover-weighted), coldest month K)."""
import sys, re
def load(d):
    out, y = {}, None
    for line in open(f'{d}/rows.txt'):
        m = re.match(r'== year (\d+)', line)
        if m:
            y = int(m.group(1)); out[y] = {'head': line.strip(), 'rows': {}}; continue
        f = line.split()
        if y is not None and len(f) == 13 and re.match(r'^-?\d+\.\d$', f[0]):
            out[y]['rows'][f[0]] = f
    return out
a, b = load(sys.argv[1]), load(sys.argv[2])
for y in sorted(a):
    if y not in b:
        continue
    print(f'year {y}\n  main:   {a[y]["head"][3:]}\n  sorted: {b[y]["head"][3:]}')
    print(f'{"lat":>5} | {"kg/m2":>6} {"lead":>5} {"%":>3} {"kinds":>5} {"npp":>5} {"frost":>5} {"tmin":>5} | {"kg/m2":>6} {"lead":>5} {"%":>3} {"kinds":>5} {"npp":>5} {"frost":>5} {"tmin":>5}')
    lats = sorted(set(a[y]['rows']) | set(b[y]['rows']), key=float, reverse=True)
    def part(r):
        if r is None:
            return ' ' * 41
        lat, n, kg, lead, lp, kinds, npp, frost, tmin = r[:9]
        return f'{float(kg):6.2f} {lead:>5} {lp:>3} {kinds:>5} {float(npp):5.2f} {float(frost):5.0f} {float(tmin):5.0f}'
    for lat in lats:
        print(f'{lat:>5} | {part(a[y]["rows"].get(lat))} | {part(b[y]["rows"].get(lat))}')
    print()
