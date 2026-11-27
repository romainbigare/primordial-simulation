"""Effects laid over a rendered god picture (god.py): smoke, flames, glow, streaks, clouds' shadows, heat shimmer.

Each works on the render's buffers (god_render.render): `rgb` (linear colour), `depth` (along the view), `glow`
(light added before the bloom).
"""
import numpy as np
from PIL import Image

import god_render as R
from god_world import bilerp, fbm, smooth, strip_layer

# The game's fire layer (fire.jpg layer 0): two flame tongues above two puffs of smoke, a quarter each.
FIRE_LAYER = 0
_puffs = {}


def puff(kind):
    """A density sprite (0..1) of the fire layer: `flame0`, `flame1`, `smoke0` or `smoke1`."""
    if kind not in _puffs:
        im = np.asarray(strip_layer('fire', FIRE_LAYER).convert('L'), np.float32) / 255
        q = {'flame0': (0, 0), 'flame1': (1, 0), 'smoke0': (0, 1), 'smoke1': (1, 1)}[kind]
        half = im.shape[0] // 2
        d = im[q[1] * half:(q[1] + 1) * half, q[0] * half:(q[0] + 1) * half]
        _puffs[kind] = d / max(float(d.max()), 1e-6)
    return _puffs[kind]


def resize(a, w, h):
    return np.asarray(Image.fromarray(np.ascontiguousarray(a, np.float32), 'F').resize((max(w, 1), max(h, 1)),
                                                                                      Image.BILINEAR))


def paste(dst, src, x0, y0, op):
    """Lay `src` on `dst` with its top left at (x0, y0), clipped; `op(region, part)` returns the new region."""
    h, w = src.shape[:2]
    ya, yb, xa, xb = max(y0, 0), min(y0 + h, dst.shape[0]), max(x0, 0), min(x0 + w, dst.shape[1])
    if ya >= yb or xa >= xb:
        return
    dst[ya:yb, xa:xb] = op(dst[ya:yb, xa:xb], src[ya - y0:yb - y0, xa - x0:xb - x0], (ya, yb, xa, xb))


def smoke(view, buf, puffs, rng):
    """Puffs of smoke or dust (dicts: x, y, z, r metres, a opacity, col linear colour, and optional glow under it),
    far ones first: each a smoke sprite turned at random, lit by the sun on its upper left side and shadowed on
    the other, hidden behind nearer ground."""
    if not puffs:
        return
    pts = np.array([[p['x'], p['y'], p['z']] for p in puffs], np.float32)
    sx, sy, dz = view.project(pts)
    for k in np.argsort(-dz):
        p = puffs[k]
        size = int(2 * p['r'] * view.f / dz[k])
        if size < 3 or dz[k] <= 0:
            continue
        d = puff('smoke0' if rng.random() < 0.5 else 'smoke1')
        d = np.rot90(d, rng.integers(4))
        d = resize(d, size, size)
        sh = max(1, size // 10)
        shadow = np.zeros_like(d)
        shadow[sh:, sh:] = d[:-sh, :-sh]
        lit = np.clip(1 - shadow * 0.9 + (d - shadow) * 0.6, 0.25, 1.3)
        col = np.array(p['col'], np.float32)
        light = (R.SUN_LIGHT * 0.55 * lit[..., None] + R.SKY_LIGHT * 0.9) * col
        if 'glow' in p:
            light = light + np.array(p['glow'], np.float32) * (1 - np.clip(lit, 0, 1))[..., None] ** 2
        a = np.clip(d * p['a'], 0, 1)
        x0, y0 = int(sx[k] - size / 2), int(sy[k] - size / 2)

        def lay(region, part, box, light=light, a=a, z=dz[k]):
            ya, yb, xa, xb = box
            al = a[ya - y0:yb - y0, xa - x0:xb - x0][..., None]
            al = al * (buf['depth'][ya:yb, xa:xb, None] > z - p['r'])
            return region * (1 - al) + light[ya - y0:yb - y0, xa - x0:xb - x0] * al
        paste(buf['rgb'], light, x0, y0, lay)


def flames(view, buf, spots, rng, colour=(9.0, 3.2, 0.7)):
    """Tongues of flame (dicts: x, y, z, h metres) added as light: each a flame sprite, its colour hotter at its
    root."""
    if not spots:
        return
    pts = np.array([[s['x'], s['y'], s['z']] for s in spots], np.float32)
    sx, sy, dz = view.project(pts)
    for k in np.argsort(-dz):
        s = spots[k]
        hpx = int(s['h'] * view.f / dz[k])
        if hpx < 3:
            continue
        d = puff('flame0' if rng.random() < 0.5 else 'flame1')
        if rng.random() < 0.5:
            d = d[:, ::-1]
        wpx = max(2, int(hpx * s.get('wide', 0.8)))
        d = resize(d, wpx, hpx)
        root = np.linspace(0.45, 1.0, hpx)[:, None]
        light = d[..., None] * np.array(colour, np.float32) * (0.35 + root[..., None] * 0.65) * s.get('a', 1.0)
        light[..., 2] *= root ** 3
        x0, y0 = int(sx[k] - wpx / 2), int(sy[k] - hpx * 0.92)

        def add(region, part, box, z=dz[k]):
            ya, yb, xa, xb = box
            vis = buf['depth'][ya:yb, xa:xb, None] > z - s['h']
            return region + part * vis
        paste(buf['glow'], light, x0, y0, add)


def glow_dots(view, buf, pts, colour, radius_m, z_test=True):
    """Soft round lights at world points `pts` (n, 3), each `radius_m` metres across its core."""
    sx, sy, dz = view.project(np.asarray(pts, np.float32))
    for x, y, z in zip(sx, sy, dz):
        r = max(1.5, radius_m * view.f / z)
        n = int(r * 4) + 1
        yy, xx = np.mgrid[-n:n + 1, -n:n + 1].astype(np.float32)
        blob = np.exp(-(xx * xx + yy * yy) / (2 * r * r))[..., None] * np.array(colour, np.float32)

        def add(region, part, box, z=z):
            ya, yb, xa, xb = box
            vis = (buf['depth'][ya:yb, xa:xb, None] > z - radius_m * 4) if z_test else 1
            return region + part * vis
        paste(buf['glow'], blob, int(x) - n, int(y) - n, add)


def streak(img, p0, p1, width, colour, fade=True):
    """A soft line from pixel p0 to p1 added to `img`, `width` pixels thick, fading from p1 back to p0."""
    (x0, y0), (x1, y1) = p0, p1
    xa, xb = int(max(min(x0, x1) - width * 3, 0)), int(min(max(x0, x1) + width * 3, img.shape[1]))
    ya, yb = int(max(min(y0, y1) - width * 3, 0)), int(min(max(y0, y1) + width * 3, img.shape[0]))
    if xa >= xb or ya >= yb:
        return
    yy, xx = np.mgrid[ya:yb, xa:xb].astype(np.float32)
    dx, dy = x1 - x0, y1 - y0
    ln2 = max(dx * dx + dy * dy, 1e-6)
    t = np.clip(((xx - x0) * dx + (yy - y0) * dy) / ln2, 0, 1)
    dist = np.hypot(xx - (x0 + t * dx), yy - (y0 + t * dy))
    a = np.exp(-dist * dist / (2 * width * width)) * (t ** 2 if fade else 1)
    img[ya:yb, xa:xb] += a[..., None] * np.array(colour, np.float32)


def cloud_field(g, x, y, cover, metres):
    """The cloud deck's density and its light at world points (x, y): density from the deck's noise (`g.clouds`),
    light from how the density falls towards the sun."""
    if not hasattr(g, 'clouds'):
        rng = np.random.default_rng(7)
        n = fbm((512, 512), 512 * 0.25, rng, 6, 0.55)
        g.clouds = n
        g.cloud_metres = metres
    s = 512 / g.cloud_metres
    fx, fy = x * s, y * s
    n = bilerp(g.clouds, fx, fy, True)
    up = bilerp(g.clouds, fx - 4, fy + 4, True)
    dens = smooth(0.55 - cover * 0.8, 0.85 - cover * 0.7, n * 0.5 + 0.5)
    lit = np.clip(0.7 + (n - up) * 3.0, 0.15, 1.3)
    return dens, lit


def cloud_shadow(g, z, cover, metres=2500.0):
    """The cloud deck's shadow on the ground, multiplied into its light."""
    away = R.SUN[:2] / R.SUN[2] * z
    dens, _ = cloud_field(g, g.X + away[0], g.Y + away[1], cover, metres)
    g.shade *= 1 - np.clip(dens, 0, 1) * 0.75


def shimmer(rgb, strength, rng, cell=18.0):
    """Heat shimmer: the picture shifted by a fine noise, more near its bottom (hot ground close to the eye)."""
    h, w = rgb.shape[:2]
    nx = fbm((h, w), cell, rng, 3, 0.5)
    ny = fbm((h, w), cell * 1.6, rng, 3, 0.5)
    k = strength * (0.4 + 0.6 * np.linspace(0, 1, h)[:, None])
    yy, xx = np.mgrid[0:h, 0:w].astype(np.float32)
    out = bilerp(rgb, xx + nx * k * 0.5, yy + ny * k, False)
    return out


def rain(rgb, rng, n, length, angle, colour, width=0.8):
    """Rain: `n` thin streaks `length` pixels long at `angle` degrees from the vertical."""
    h, w = rgb.shape[:2]
    a = np.radians(angle)
    for _ in range(n):
        x, y = rng.random() * w, rng.random() * h
        ln = length * (0.6 + rng.random() * 0.8)
        streak(rgb, (x, y), (x + np.sin(a) * ln, y + np.cos(a) * ln), width, np.array(colour) * rng.uniform(0.5, 1),
               fade=False)

