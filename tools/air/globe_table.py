# Globe probe summary: per latitude row (cells of equal latitude), area-weighted monthly ground temperature; yearly mean,
# warmest and coldest month; ice; vapour. Then each row's surface budget in its darkest three months and over the year.
import csv, sys, collections
D, tree = sys.argv[1], sys.argv[2]
cells = list(csv.DictReader(open(f'{D}/{tree}_cells.csv')))
air = list(csv.DictReader(open(f'{D}/{tree}_air.csv')))
f = lambda r, k: float(r[k])
rows = collections.defaultdict(lambda: collections.defaultdict(list))
for r in cells:
    rows[round(f(r, 'lat'), 1)][int(r['month'])].append(r)
arows = collections.defaultdict(lambda: collections.defaultdict(list))
for r in air:
    arows[round(f(r, 'lat'), 1)][int(r['month'])].append(r)
def wmean(rs, k, wk='adt'):
    s = sum(f(r, wk) for r in rs)
    return sum(f(r, k) * f(r, wk) for r in rs) / s if s > 0 else float('nan')
print(f'{tree}: lat   cells land  T_year  T_warm  T_cold  (land: year warm cold)  ice m max  vapour kg/m2 year')
for lat in sorted(rows, reverse=True):
    ms = rows[lat]
    t = [wmean(ms[m], 't') for m in sorted(ms)]
    land = [[r for r in ms[m] if r['land'] == '1'] for m in sorted(ms)]
    tl = [wmean(l, 't') if l else float('nan') for l in land]
    ice = max(max(f(r, 'ice_m') for r in ms[m]) for m in ms)
    vap = sum(wmean(arows[lat][m], 'vap_kg_m2', 'area') for m in arows[lat]) / max(1, len(arows[lat])) if lat in arows else float('nan')
    print(f'{lat:7.1f} {len(ms[0]):5d} {len(land[0]):4d} {sum(t)/12:7.1f} {max(t):7.1f} {min(t):7.1f}   ({sum(tl)/12:6.1f} {max(tl):6.1f} {min(tl):6.1f})  {ice:8.3f}  {vap:6.1f}')
keys = ['sw', 'up', 'down', 'down_clear', 'cloud', 'sens', 'lat_heat', 'to_space', 'air_sun', 'ts']
print(f'{tree}: surface budget W/m2 (+ into the surface: sw, down; out: up, sens = contact heat to the air, lat_heat), net = sw+down-up-sens-lat')
for lat in sorted(rows, reverse=True):
    if abs(lat) < 50: continue
    ms = rows[lat]
    order = sorted(ms, key=lambda m: wmean(ms[m], 'sw'))[:3]
    for name, sel in (('dark3', order), ('year', sorted(ms))):
        v = {k: sum(wmean(ms[m], k) for m in sel) / len(sel) for k in keys}
        net = v['sw'] + v['down'] - v['up'] - v['sens'] - v['lat_heat']
        sp = sum(sum(f(r, 'space_J') for r in arows[lat][m]) / sum(f(r, 'area') for r in arows[lat][m]) / (30 * 86400) for m in sel) / len(sel)
        toa = v['sw'] + v['air_sun'] - v['to_space'] - sp
        print(f'{lat:6.1f} {name:5s} months {sel if name=="dark3" else "all"}: sw {v["sw"]:6.1f} up {v["up"]:6.1f} down {v["down"]:6.1f} clear {v["down_clear"]:6.1f} cloud {v["cloud"]:.2f} sens {v["sens"]:6.1f} latent {v["lat_heat"]:6.1f} net {net:6.1f} | ts {v["ts"]:6.1f} | air sun {v["air_sun"]:5.1f} OLR {v["to_space"]+sp:6.1f} TOA net {toa:6.1f}')
