"""The pieces the god pictures are made of (god.py): noise, blur, sampling, the textures and the sprites.

Colours are linear (the photographs' sRGB undone); a world point is (x east, y north, z up) in metres.
"""
import os

import numpy as np
from PIL import Image

HERE = os.path.dirname(os.path.abspath(__file__))
TEXTURES = os.path.dirname(HERE)
LAYER = 512


def to_linear(srgb):
    """sRGB values 0..1 to linear light."""
    return np.where(srgb <= 0.04045, srgb / 12.92, ((srgb + 0.055) / 1.055) ** 2.4)


def to_srgb(lin):
    """Linear light to sRGB values 0..1."""
    lin = np.clip(lin, 0, 1)
    return np.where(lin <= 0.0031308, lin * 12.92, 1.055 * lin ** (1 / 2.4) - 0.055)


def smooth(e0, e1, x):
    """Hermite step from 0 at e0 to 1 at e1."""
    t = np.clip((x - e0) / (e1 - e0), 0, 1)
    return t * t * (3 - 2 * t)


def box1(a, r, axis):
    """A running mean of 2r+1 values along `axis`, the edges held."""
    if r < 1:
        return a
    pad = [(0, 0)] * a.ndim
    pad[axis] = (r + 1, r)
    c = np.cumsum(np.pad(a, pad, mode='edge'), axis=axis, dtype=np.float64)
    n = a.shape[axis]
    hi = np.take(c, np.arange(2 * r + 1, 2 * r + 1 + n), axis=axis)
    lo = np.take(c, np.arange(0, n), axis=axis)
    return ((hi - lo) / (2 * r + 1)).astype(np.float32)


def blur(a, sigma):
    """A Gaussian blur of `sigma` texels over the first two axes: three running means (Wells 1986)."""
    if sigma < 0.5:
        return a
    r = max(1, int(round(np.sqrt(12 * sigma * sigma / 3 + 1) / 2 - 0.5)))
    for _ in range(3):
        a = box1(box1(a, r, 0), r, 1)
    return a


def lattice(shape, cell, rng):
    """Smooth noise of about zero mean and unit spread: random values every `cell` texels, bicubic between."""
    h, w = shape
    gh, gw = int(h / cell) + 4, int(w / cell) + 4
    g = rng.standard_normal((gh, gw)).astype(np.float32)
    big = Image.fromarray(g, 'F').resize((int(gw * cell), int(gh * cell)), Image.BICUBIC)
    a = np.asarray(big)
    oy, ox = int(cell * 1.5), int(cell * 1.5)
    return a[oy:oy + h, ox:ox + w]


def fbm(shape, cell, rng, octaves=5, gain=0.5, ridged=False):
    """Fractal noise: octaves of `lattice` from `cell` texels down, each `gain` as strong; ridged folds each octave
    (1 - |n|), the sharp crests of eroded ranges (Musgrave 1989)."""
    out = np.zeros(shape, np.float32)
    amp, total = 1.0, 0.0
    for _ in range(octaves):
        n = lattice(shape, max(cell, 1.5), rng)
        if ridged:
            n = 1 - np.abs(n) * 0.9
            n = n * n
        out += amp * n
        total += amp
        amp *= gain
        cell /= 2
    return out / total


def eroded(shape, cell, rng, octaves=8, gain=0.5, damp=1.5):
    """Noise with the look of eroded land: each octave weakened where the octaves above it already slope steeply,
    so slopes stay smooth and the detail gathers on crests and in gullies (Quilez 2008, "noise derivatives")."""
    out = np.zeros(shape, np.float32)
    gx = np.zeros(shape, np.float32)
    gy = np.zeros(shape, np.float32)
    amp, total, first = 1.0, 0.0, cell
    for _ in range(octaves):
        n = lattice(shape, max(cell, 1.5), rng)
        dy, dx = np.gradient(n)
        dx, dy = dx * first, dy * first
        gx += amp * dx
        gy += amp * dy
        out += amp * n / (1 + damp * (gx * gx + gy * gy))
        total += amp
        amp *= gain
        cell /= 2
    return out / total


# Rain's erosion works on heights this many times a texel's side (gentle slopes, as Beyer's method is tuned for).
ERODE_SCALE = 1 / 40
# Each drop wears the ground within this many texels of it (its brush).
ERODE_BRUSH = 2


def erode(h, rng, drops=200_000, steps=48, batch=2_500, rate=0.3, depo=0.3, carry=4.0, evap=0.02):
    """Carve the height field `h` (in texels' units: a rise of 1 is one texel's side) with rain drops that run
    downhill, each taking up ground (from a small brush round it) while it runs faster than its load allows and
    dropping it as it slows (Beyer 2015, "Implementation of a method for hydraulic erosion"). Returns the carved
    field and how much water ran over each texel (the streams)."""
    h = h.astype(np.float64) * ERODE_SCALE
    ny, nx = h.shape
    flow = np.zeros_like(h)
    offs = np.arange(-ERODE_BRUSH + 1, ERODE_BRUSH + 1)
    ox, oy = [a.ravel() for a in np.meshgrid(offs, offs)]
    for _ in range(drops // batch):
        x = rng.random(batch) * (nx - 2)
        y = rng.random(batch) * (ny - 2)
        dx, dy = np.zeros(batch), np.zeros(batch)
        speed, water, load = np.ones(batch), np.ones(batch), np.zeros(batch)
        alive = np.ones(batch, bool)
        for _ in range(steps):
            ix, iy = x.astype(np.int64), y.astype(np.int64)
            u, v = x - ix, y - iy
            a, b, c, d = h[iy, ix], h[iy, ix + 1], h[iy + 1, ix], h[iy + 1, ix + 1]
            gx = (b - a) * (1 - v) + (d - c) * v
            gy = (c - a) * (1 - u) + (d - b) * u
            here = a * (1 - u) * (1 - v) + b * u * (1 - v) + c * (1 - u) * v + d * u * v
            dx, dy = dx * 0.1 - gx * 0.9, dy * 0.1 - gy * 0.9
            ln = np.hypot(dx, dy)
            alive &= ln > 1e-12
            dx, dy = dx / np.maximum(ln, 1e-12), dy / np.maximum(ln, 1e-12)
            x2, y2 = x + dx, y + dy
            alive &= (x2 >= 1) & (x2 < nx - 3) & (y2 >= 1) & (y2 < ny - 3)
            x2, y2 = np.clip(x2, 1, nx - 3.001), np.clip(y2, 1, ny - 3.001)
            jx, jy = x2.astype(np.int64), y2.astype(np.int64)
            s, t = x2 - jx, y2 - jy
            there = (h[jy, jx] * (1 - s) * (1 - t) + h[jy, jx + 1] * s * (1 - t) + h[jy + 1, jx] * (1 - s) * t +
                     h[jy + 1, jx + 1] * s * t)
            dh = there - here
            cap = np.maximum(-dh, 0.001) * speed * water * carry
            drop = np.where(dh > 0, np.minimum(dh, load), np.maximum(load - cap, 0) * depo) * alive
            take = np.where((dh <= 0) & (load <= cap), np.minimum((cap - load) * rate, -dh), 0) * alive
            load += take - drop
            for wx, wy, w in ((0, 0, (1 - u) * (1 - v)), (1, 0, u * (1 - v)), (0, 1, (1 - u) * v), (1, 1, u * v)):
                np.add.at(h, (iy + wy, ix + wx), drop * w)
            bx = np.clip(ix[:, None] + ox[None], 0, nx - 1)
            by = np.clip(iy[:, None] + oy[None], 0, ny - 1)
            bw = np.maximum(ERODE_BRUSH - np.hypot(bx - x[:, None], by - y[:, None]), 0)
            bw /= np.maximum(bw.sum(1, keepdims=True), 1e-9)
            np.add.at(h, (by.ravel(), bx.ravel()), -(bw * take[:, None]).ravel())
            np.add.at(flow, (iy, ix), water * alive)
            speed = np.sqrt(np.maximum(speed * speed - dh * 4, 0.0))
            water *= 1 - evap
            x, y = np.where(alive, x2, x), np.where(alive, y2, y)
            if not alive.any():
                break
    return (h / ERODE_SCALE).astype(np.float32), flow.astype(np.float32)


def cells(x, y, size, seed=0.0):
    """Cells of a jittered grid `size` metres apart, as cracked mud or a dried crust breaks (Worley 1996): each
    point's distance to the nearest edge between two cells (metres) and a number 0..1 of its own cell."""
    gx, gy = np.floor(x / size), np.floor(y / size)
    d1 = np.full(x.shape, 1e9, np.float32)
    d2 = np.full(x.shape, 1e9, np.float32)
    own = np.zeros(x.shape, np.float32)
    for oy in (-1, 0, 1):
        for ox in (-1, 0, 1):
            cx, cy = gx + ox, gy + oy
            hx = (np.sin(cx * 127.1 + cy * 311.7 + seed) * 43758.5453) % 1
            hy = (np.sin(cx * 269.5 + cy * 183.3 + seed) * 43758.5453) % 1
            d = np.hypot(x - (cx + hx) * size, y - (cy + hy) * size)
            closer = d < d1
            d2 = np.where(closer, d1, np.minimum(d2, d))
            own = np.where(closer, hx, own)
            d1 = np.minimum(d1, d)
    return (d2 - d1) * 0.5, own


def bilerp(img, fx, fy, wrap):
    """`img` (h, w[, c]) at fractional texels (fx, fy); wrapping round or held at the edges."""
    h, w = img.shape[:2]
    x0f, y0f = np.floor(fx), np.floor(fy)
    tx, ty = (fx - x0f).astype(np.float32), (fy - y0f).astype(np.float32)
    x0, y0 = x0f.astype(np.int64), y0f.astype(np.int64)
    if wrap:
        x0, y0 = x0 % w, y0 % h
        x1, y1 = (x0 + 1) % w, (y0 + 1) % h
    else:
        x0, y0 = np.clip(x0, 0, w - 1), np.clip(y0, 0, h - 1)
        x1, y1 = np.minimum(x0 + 1, w - 1), np.minimum(y0 + 1, h - 1)
    if img.ndim == 3:
        tx, ty = tx[..., None], ty[..., None]
    top = img[y0, x0] * (1 - tx) + img[y0, x1] * tx
    bot = img[y1, x0] * (1 - tx) + img[y1, x1] * tx
    return top * (1 - ty) + bot * ty


def mips(img):
    """`img` and its halvings (the mean of four texels each) down to 4 texels a side."""
    out = [img.astype(np.float32)]
    while min(out[-1].shape[:2]) >= 8:
        a = out[-1]
        h, w = a.shape[0] // 2 * 2, a.shape[1] // 2 * 2
        a = a[:h, :w]
        out.append((a[0::2, 0::2] + a[1::2, 0::2] + a[0::2, 1::2] + a[1::2, 1::2]) * 0.25)
    return out


def sample_mips(pyr, fx, fy, lod, wrap):
    """The pyramid `pyr` at level-0 texels (fx, fy), blended between the two levels round `lod` (trilinear)."""
    lod = np.clip(lod, 0, len(pyr) - 1.001)
    base = np.floor(lod).astype(np.int64)
    frac = (lod - base).astype(np.float32)
    shape = fx.shape + pyr[0].shape[2:]
    out = np.zeros(shape, np.float32)
    for lv in np.unique(base):
        m = base == lv
        s = 0.5 ** lv
        a = bilerp(pyr[lv], fx[m] * s - 0.5 + 0.5 * s, fy[m] * s - 0.5 + 0.5 * s, wrap)
        b = bilerp(pyr[lv + 1], fx[m] * s * 0.5 - 0.5 + 0.25 * s, fy[m] * s * 0.5 - 0.5 + 0.25 * s, wrap)
        f = frac[m] if a.ndim == 1 else frac[m][:, None]
        out[m] = a * (1 - f) + b * f
    return out


class Tex:
    """A repeating ground texture `side` metres across: its colour (linear) and, where it has one, its relief (0..1)."""

    def __init__(self, rgb, side, relief=None):
        self.side = side
        self.size = rgb.shape[0]
        self.rgb = mips(rgb)
        self.relief = mips(relief) if relief is not None else None

    def _at(self, pyr, x, y, cell, turn):
        c, s = np.cos(turn), np.sin(turn)
        u, v = (x * c - y * s) / self.side * self.size, (x * s + y * c) / self.side * self.size
        lod = np.full(x.shape, np.log2(max(cell / (self.side / self.size), 1.0)), np.float32)
        return sample_mips(pyr, u, v, lod, True)

    def colour(self, x, y, cell, turn=0.0):
        """The colour at world points (x, y) seen at `cell` metres a texel, the texture turned `turn` radians."""
        return self._at(self.rgb, x, y, cell, turn)

    def height(self, x, y, cell, turn=0.0):
        """The relief at (x, y), 0..1 (a flat 0.5 where the texture has none)."""
        if self.relief is None:
            return np.full(x.shape, 0.5, np.float32)
        return self._at(self.relief, x, y, cell, turn)


def load_rgb(path, box=None, size=None):
    """An image (or the part `box` of it) as linear RGB floats, shrunk to `size` texels a side where asked."""
    im = Image.open(path).convert('RGB')
    if box:
        im = im.crop(box)
    if size and im.size[0] != size:
        im = im.resize((size, size), Image.LANCZOS)
    return to_linear(np.asarray(im, np.float32) / 255)


def load_grey(path, size=None):
    """A greyscale image (a relief map) as floats 0..1."""
    im = Image.open(path)
    a = np.asarray(im)
    if a.ndim == 3:
        a = a[..., 0]
    a = a.astype(np.float32) / (65535.0 if a.dtype == np.uint16 else 255.0)
    if size and a.shape[0] != size:
        a = np.asarray(Image.fromarray(a, 'F').resize((size, size), Image.BILINEAR))
    return a


def strip_layer(group, index):
    """Layer `index` (0 at the top) of the game's strip `group` (strips.json), as an image."""
    import json
    files = json.load(open(os.path.join(TEXTURES, 'strips.json')))[group]
    for f in files:
        im = Image.open(os.path.join(TEXTURES, f))
        n = im.size[1] // LAYER
        if index < n:
            return im.crop((0, index * LAYER, LAYER, (index + 1) * LAYER))
        index -= n
    raise IndexError(f'{group} has no layer {index}')
