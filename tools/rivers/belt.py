# The belt's walk on a straight valley, to tune its numbers by eye: python3 belt.py OUT.png [key=value ...]
import math, random, sys
from PIL import Image, ImageDraw
P = dict(STEPS=24, HEADING_MOST=1.9, R0=1.8, R1=5.0, REACH0=0.35, REACH1=1.0, SWELL_BENDS=5, SWELL_LEAST=0.08, ARC0=1.5, ARC1=24.0, BELT=4.0, RPOW=2.0, SWPOW=1.0, CURVE_SWELL=0.5, WANDER=0.5, WANDER_BENDS=7, FAR_LEAST=0.3, ADV=2.0, rows=6, seed=1)
for a in sys.argv[2:]:
    k, v = a.split('='); P[k] = float(v)
STEPS = int(P['STEPS'])
def heading(arc, c, x): return arc * x * (1 - x) * (c[0] * (1 - x) + c[1] * x)
def across(arc, c): return sum(arc / STEPS * math.sin(heading(arc, c, (i + .5) / STEPS)) for i in range(STEPS))
def advance(arc, c): return sum(arc / STEPS * math.cos(heading(arc, c, (i + .5) / STEPS)) for i in range(STEPS))
def walk(seed, width, n):
    rnd = random.Random(seed)
    sw = [rnd.random() for _ in range(n // int(P['SWELL_BENDS']) + 3)]
    wa = [rnd.random() for _ in range(n // int(P['WANDER_BENDS']) + 3)]
    side = 1 if rnd.random() < .5 else -1
    u = v = 0.0; curve = [0.0, 0.0]; pts = [(0, 0)]; belt = P['BELT'] * width
    for half in range(n):
        i, f = divmod(half, int(P['SWELL_BENDS'])); f /= P['SWELL_BENDS']; t = f * f * (3 - 2 * f)
        swell = sw[i] * (1 - t) + sw[i + 1] * t
        swell = P['SWELL_LEAST'] + (1 - P['SWELL_LEAST']) * (1 - (1 - swell) ** P['SWPOW'])
        reach = belt * swell * (P['REACH0'] + (P['REACH1'] - P['REACH0']) * rnd.random())
        side = -side
        i2, f2 = divmod(half, int(P['WANDER_BENDS'])); f2 /= P['WANDER_BENDS']; t2 = f2 * f2 * (3 - 2 * f2)
        mid = belt * swell * P['WANDER'] * (2 * (wa[i2] * (1 - t2) + wa[i2 + 1] * t2) - 1)
        reach *= 1 - abs(mid) / belt
        radius = width * (P['R0'] + (P['R1'] - P['R0']) * rnd.random() ** P['RPOW'])
        curve = [curve[1], swell ** P['CURVE_SWELL'] / radius]
        far = max(side * (mid + side * reach - v), P['FAR_LEAST'] * reach)
        apex = max(-belt, min(belt, v + side * far)); far = apex - v; swing = -1 if far < 0 else 1
        s = curve[0] + curve[1]
        lo, hi = P['ARC0'] * width, min(8 * P['HEADING_MOST'] / s, P['ARC1'] * width); lo = min(lo, hi)
        for _ in range(14):
            mid = .5 * (lo + hi)
            if across(mid, curve) < abs(far) and advance(mid, curve) > P['ADV'] * width: lo = mid
            else: hi = mid
        arc = .5 * (lo + hi); gain = abs(far) / max(across(arc, curve), 1e-9)
        for k in range(STEPS):
            th = heading(arc, curve, (k + .5) / STEPS); ds = arc / STEPS
            v += swing * gain * ds * math.sin(th); u += ds * math.cos(th); pts.append((u, v))
        v = apex
    return pts
rows = int(P['rows']); W, H, width = 1800, 260 * rows, 5.0
im = Image.new('RGB', (W, H), (120, 180, 110)); d = ImageDraw.Draw(im)
for r in range(rows):
    pts = walk(P['seed'] * 100 + r, width, 140); k = 1.5
    line = [(20 + p[0] * k, 130 + 260 * r + p[1] * k) for p in pts if 20 + p[0] * k < W]
    d.line(line, fill=(40, 90, 200), width=int(width * k), joint='curve')
    length = sum(math.hypot(pts[i + 1][0] - pts[i][0], pts[i + 1][1] - pts[i][1]) for i in range(len(pts) - 1))
    d.text((10, 260 * r + 5), 'sinuosity %.2f' % (length / pts[-1][0]), fill=(0, 0, 0))
im.save(sys.argv[1])
