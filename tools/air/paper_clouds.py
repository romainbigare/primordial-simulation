# Paper check of a layered cloud cover from the tip globe's states (never part of the repository): per node and month
# the humidity profile from the column's ratio, the vapour's scale height and the temperature's fall; per height the
# critical humidity of Quaas (2012); Sundqvist's saturated share per height; overlap by a decorrelation depth of 2 km
# (Hogan and Illingworth 2000). The node's cloud water kept (S3b's path over the new cover); the sunlight absorbed by
# a crude two-stream (reflectance x / (1 + x), x = 0.13 tau, Lacis and Hansen 1974) against the present cover.
import csv, sys, math, collections
D = sys.argv[1]
Z0, DZ, TOP = 2000.0, 50.0, 10000.0
def esat(t): return 611.2 * math.exp(17.67 * (t - 273.15) / (t - 29.65))
def rc(z): return 0.74 - (0.74 - 0.375) * min(z / 3000.0, 1.0)
def sat(r, r0): return 0.0 if r <= r0 else 1.0 if r >= 1 else 1 - math.sqrt((1 - r) / (1 - r0))
def cover(r, t0, hk, gam):
    rho0 = r * esat(t0) / t0
    c, z = 0.0, 0.0
    while z < TOP:
        t = t0 - gam * z
        if t < 180: break
        rh = rho0 * math.exp(-z / hk) / (esat(t) / t)
        b = sat(rh, rc(z))
        c = max(c, b)
        c += DZ / Z0 * min(c, b) * (1 - max(c, b))
        z += DZ
    return c
def R(tau): x = 0.13 * tau; return x / (1 + x)
cells = list(csv.DictReader(open(f'{D}/tip3_cells.csv'))); air = list(csv.DictReader(open(f'{D}/tip3_air.csv')))
sw = collections.defaultdict(list)
for r in cells: sw[(round(float(r['lat']), 1), int(r['month']))].append(float(r['sw']))
rows = collections.defaultdict(list)
for r in air: rows[(round(float(r['lat']), 1), int(r['month']))].append(r)
print('lat   season  cover now -> layered  | tau now -> new | surface sun absorbed now -> new (W/m2)')
tot = collections.defaultdict(lambda: [0, 0, 0, 0, 0])
for (lat, m) in sorted(rows, key=lambda k: (-k[0], k[1])):
    rs = rows[(lat, m)]
    c_old = sum(float(r['cloud']) for r in rs) / len(rs)
    tau = sum(float(r['tau']) for r in rs) / len(rs)
    c_new = sum(cover(float(r['ratio']), float(r['t_low']), float(r['hk']), float(r['gam'])) for r in rs) / len(rs)
    tau_new = tau * c_old / c_new if c_new > 0 else 0.0
    s = sum(sw[(lat, m)]) / len(sw[(lat, m)])
    clear = s / (1 - c_old * R(tau)) if c_old * R(tau) < 1 else s
    s_new = clear * (1 - c_new * R(tau_new))
    t = tot[lat]; t[0] += c_old / 12; t[1] += c_new / 12; t[2] += s / 12; t[3] += s_new / 12
    if m in (0, 6): print(f'{lat:6.1f} month {m:2d}: {c_old:.2f} -> {c_new:.2f} | {tau:5.1f} -> {tau_new:5.1f} | {s:6.1f} -> {s_new:6.1f}')
print('year:')
for lat in sorted(tot, reverse=True):
    t = tot[lat]; print(f'{lat:6.1f}: cover {t[0]:.2f} -> {t[1]:.2f}; surface sun {t[2]:6.1f} -> {t[3]:6.1f} (change {t[3]-t[2]:+.1f})')
