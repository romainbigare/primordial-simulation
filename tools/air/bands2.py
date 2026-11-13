"""Heat light in three bands, each absorber at its own height: fit the band numbers to Prata (1996), clear-sky
outgoing 266 W/m2 and 3.7 W/m2 a doubling; python3 bands2.py LAYERS [CONTINUUM m2/(mol Pa)]"""
import math, random, sys
SIG = 5.67e-8


def column(x, gam, l, hh, L):
    f = 1 - math.exp(-x)
    e = math.exp(-l / hh)
    drop = lambda above: gam * hh * math.log(e + (1 - e) * above)
    out = [[0] * L, [0] * L]
    if x > 0:
        for m in range(L):
            near = -math.log1p(-f * (m + 0.5) / L) / x
            out[0][m] = drop(1 - near)
            out[1][m] = drop(near)
    return out, f


def glow(f, tl, lay):
    return SIG * f / len(lay) * sum((tl + d) ** 4 for d in lay)


AIR = 3.57e5


def depth(a, s, c):
    return a * (math.sqrt(s * c) if s > 0 and c > s else c)


gam, l, T = 6.5e-3, 1e4, 288.0
hh = 8.314 * T / (0.029 * 9.81)
HV = 2000.0
prata = lambda w: 1 - (1 + w) * math.exp(-math.sqrt(1.2 + 3 * w))
SH = [0.283, 0.252, 0.465]
L = int(sys.argv[1])
KC = float(sys.argv[2]) if len(sys.argv) > 2 else 4.4e-7


def run(p, w, co2=400e-6):
    va, vs, ca, cs, vc = p
    cv = w * 555.6
    cc = co2 * AIR
    p0 = cv * 8.314 * T / HV
    tc = KC * cv * p0 / 2
    dv = depth(va, vs, cv)
    dc = depth(ca, cs, cc)
    dvc = vc * dv
    bands = [(tc, HV / 2), (dc + dvc, (dc * hh + dvc * HV) / (dc + dvc)), (dv, HV)]
    down = olr = win = 0
    for sh, (x, h) in zip(SH, bands):
        lay, f = column(x, gam, l, h, L)
        down += sh * glow(f, T, lay[0])
        olr += sh * ((1 - f) * SIG * T ** 4 + glow(f, T, lay[1]))
        win += sh * (1 - f) * SIG * T ** 4
    return down / (SIG * T ** 4), olr, win


ws = [0.5, 0.75, 1, 1.5, 2, 2.5, 3.5, 5]


def err(p):
    if min(p) <= 0:
        return 1e9
    e = sum((run(p, w)[0] - prata(w)) ** 2 for w in ws) / len(ws)
    F = run(p, 2.5)[1] - run(p, 2.5, 800e-6)[1]
    e += ((F - 3.7) / 3.7 * 0.1) ** 2
    e += ((run(p, 2.5)[1] - 266) / 266 * 0.1) ** 2
    return e


best = [0.03, 8.0, 0.026, 5.0, 0.1]
be = err(best)
random.seed(1)
for it in range(30000):
    q = [x * math.exp(random.gauss(0, 0.3 if it < 15000 else 0.06)) for x in best]
    eq = err(q)
    if eq < be:
        best, be = q, eq
p = best
r = lambda w, c=400e-6: run(p, w, c)
print("layers %d continuum %.2e: vapour %.3e sat %.3g; carbon %.3e sat %.3g; vapour in carbon band %.3f; rms %.4f" % (L, KC, *p, math.sqrt(be)))
print(" Prata", [(w, round(r(w)[0], 3), round(prata(w), 3)) for w in (0.5, 1, 2.5, 5)])
print(" clear OLR %.1f window %.1f forcing %.2f; window continuum depth at 2.5 cm %.2f" % (r(2.5)[1], r(2.5)[2], r(2.5)[1] - r(2.5, 800e-6)[1], KC * 1389 * 1389 * 8.314 * 288 / 2000 / 2))
