# Annual by latitude row: sunlight absorbed at the surface and in the air, light to space, net at the top, cloud share, down light.
import csv, sys, collections
D, tree = sys.argv[1], sys.argv[2]
f = float
cells = list(csv.DictReader(open(f'{D}/{tree}_cells.csv'))); air = list(csv.DictReader(open(f'{D}/{tree}_air.csv')))
by = collections.defaultdict(list); ab = collections.defaultdict(list)
for r in cells: by[round(f(r['lat']), 1)].append(r)
for r in air: ab[round(f(r['lat']), 1)].append(r)
print(f'{tree}: lat   sw_surf air_sun absorbed  OLR  TOA_net cloud down  up')
for lat in sorted(by, reverse=True):
    rs = by[lat]; s = sum(f(r['adt']) for r in rs); wm = lambda k: sum(f(r[k]) * f(r['adt']) for r in rs) / s
    sp = sum(f(r['space_J']) for r in ab[lat]) / (sum(f(r['area']) for r in ab[lat]) / 12) / (360 * 86400)
    olr = wm('to_space') + sp; absd = wm('sw') + wm('air_sun')
    print(f'{lat:7.1f} {wm("sw"):7.1f} {wm("air_sun"):6.1f} {absd:7.1f} {olr:6.1f} {absd-olr:7.1f} {wm("cloud"):5.2f} {wm("down"):5.1f} {wm("up"):5.1f}')
