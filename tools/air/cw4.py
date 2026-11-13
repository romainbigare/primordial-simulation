"""Three bands, each split into a core and wings of half its share, the core's depth rho times the band's, the wings'
1/rho times: fit the band numbers and rho to Prata, 266 and 3.7. python3 cw.py"""
import math, random
exec(open('/tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/bands2.py').read().split("ws = [0.5")[0].replace("L = int(sys.argv[1])", "L = 2").replace("KC = float(sys.argv[2]) if len(sys.argv) > 2 else 4.4e-7", "KC = 4.4e-7"))


def run(p, w, co2=400e-6):
    va, vs, ca, cs, rho = p
    cv = w * 555.6
    cc = co2 * AIR
    p0 = cv * 8.314 * T / HV
    tc = KC * cv * p0 / 2
    PB = lambda h: h * hh / (h + hh)
    bands = [(0.0, 1.0), (depth(ca, cs, cc), PB(hh)), (depth(va, vs, cv), PB(HV))]
    down = olr = win = 0
    for sh, (d, hl) in zip(SH, bands):
        for part in (rho, 1 / rho):
            x = d * part + tc
            h = (d * part * hl + tc * HV / 2) / x
            lay, f = column(x, gam, l, h, L)
            down += sh / 2 * glow(f, T, lay[0])
            olr += sh / 2 * ((1 - f) * SIG * T ** 4 + glow(f, T, lay[1]))
            win += sh / 2 * (1 - f) * SIG * T ** 4
    return down / (SIG * T ** 4), olr, win


ws = [0.5, 0.75, 1, 1.5, 2, 2.5, 3.5, 5]


def err(p):
    if min(p) <= 0 or p[4] < 1:
        return 1e9
    e = sum((run(p, w)[0] - prata(w)) ** 2 for w in ws) / len(ws)
    F = run(p, 2.5)[1] - run(p, 2.5, 800e-6)[1]
    e += ((F - 3.7) / 3.7 * 0.05) ** 2
    e += ((run(p, 2.5)[1] - 266) / 266 * 0.05) ** 2
    return e


best = [0.1, 30, 0.1, 10, 3.0]
be = err(best)
random.seed(3)
for it in range(60000):
    q = [x * math.exp(random.gauss(0, 0.3 if it < 30000 else 0.06)) for x in best]
    eq = err(q)
    if eq < be:
        best, be = q, eq
p = best
r = lambda w, c=400e-6: run(p, w, c)
print("GREY continuum: vapour %.3e sat %.3g; carbon %.3e sat %.3g; rho %.2f; rms %.4f" % (*p, math.sqrt(be)))
print(" Prata", [(w, round(r(w)[0], 3), round(prata(w), 3)) for w in (0.5, 1, 2.5, 5)])
print(" clear OLR %.1f window %.1f forcing %.2f" % (r(2.5)[1], r(2.5)[2], r(2.5)[1] - r(2.5, 800e-6)[1]))
