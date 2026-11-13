# Month by month for one latitude row: sun absorbed, cloud share, back light all/clear, up, contact, latent, net, surface T, air T, vapour, ice.
import csv, sys
D, tree, lat = sys.argv[1], sys.argv[2], float(sys.argv[3])
cells = [r for r in csv.DictReader(open(f'{D}/{tree}_cells.csv')) if abs(float(r['lat']) - lat) < 0.5]
air = [r for r in csv.DictReader(open(f'{D}/{tree}_air.csv')) if abs(float(r['lat']) - lat) < 0.5]
f = float
def wm(rs, k, w='adt'):
    s = sum(f(r[w]) for r in rs); return sum(f(r[k]) * f(r[w]) for r in rs) / s
print(f'{tree} lat {lat}: m   sw  cloud  down clear   up  sens latent   net   T_gr   ts  T_air vap  ice_m')
for m in range(12):
    rs = [r for r in cells if r['month'] == str(m)]; ar = [r for r in air if r['month'] == str(m)]
    v = {k: wm(rs, k) for k in ['sw', 'cloud', 'down', 'down_clear', 'up', 'sens', 'lat_heat', 'ts', 't']}
    net = v['sw'] + v['down'] - v['up'] - v['sens'] - v['lat_heat']
    print(f'  {m:2d} {v["sw"]:6.1f} {v["cloud"]:5.2f} {v["down"]:5.1f} {v["down_clear"]:5.1f} {v["up"]:5.1f} {v["sens"]:5.1f} {v["lat_heat"]:5.1f} {net:6.1f} {v["t"]:6.1f} {v["ts"]:6.1f} {v["ts"]-v["sens"]/10:6.1f} {wm(ar, "vap_kg_m2", "area"):5.1f} {max(f(r["ice_m"]) for r in rs):6.3f}')
