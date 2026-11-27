"""water.py FOAM001_DIR FOAM002_DIR OUT_PNG: the water's strip (textures/water.png, SOURCES.md), two 512 x 512 layers
that hold numbers, not colours (web/src/detail.ts `NUMBER_LAYERS`), each repeating.

Layer 0, the surface: a sea of small waves made from a spectrum, its slope east and south in red and green (0.5 is
flat, each a share of `SLOPE_RANGE` either way), and in blue the light it focuses on a bed under it (its caustics, a
share of `CAUSTIC_RANGE`, 1 on the mean). The waves' heights are Gaussian with random phases over a spectrum with a
peak (`PEAK` cycles a tile; Pierson and Moskowitz 1964 in the wave number) falling as k^-`FALL` past it, down to
`SHORTEST` texels, each crest sharpened by moving the surface toward it (choppy waves, Tessendorf 2001) by `CHOP`. The caustics are the
sunlight refracted through that surface (index 1.33, the sun overhead) onto a flat bed `BED_DEPTH` texels per unit of
slope deep, gathered texel by texel.

Layer 1, foam: ambientCG's Foam001 and Foam002 (CC0, their 1K opacity maps: sea foam's lace) shrunk to 512, each made
even (its texels ranked, so a share s of them lies over 1 - s: the shader's threshold covers just the share it asks
for), in red and green; and in blue Foam001's colour's lightness over its mean, halved (the foam's own light and shade).

Run with `python3 -I` (numpy and Pillow); the downloads are data only.
"""
import sys

import numpy as np
from PIL import Image

SIDE = 512
# The waves' peak, cycles a tile, how steeply their elevation falls past it (as a power of the wave number), and the
# shortest, texels; the seed of the random phases.
PEAK = 5.0
FALL = 3.3
SHORTEST = 3.0
SEED = 7
# How far each crest is sharpened (a share of the displacement that would fold the surface on itself).
CHOP = 0.8
# The slope stored either way of flat, in units of the slopes' spread (the shader's `WAVE_RANGE` times it).
SLOPE_RANGE = 3.0
# The depth of the bed the caustics fall on, texels per unit of the slopes' spread, and the caustics' range stored.
BED_DEPTH = 14.0
CAUSTIC_RANGE = 3.0
N_WATER = 1.33


def wave_layer():
    """Layer 0: slopes and caustics (module doc)."""
    rng = np.random.default_rng(SEED)
    f = np.fft.fftfreq(SIDE) * SIDE
    kx, ky = np.meshgrid(f, f)
    k = np.hypot(kx, ky)
    k[0, 0] = 1.0
    shortest = SIDE / SHORTEST
    # Elevation amplitude ~ k^-FALL past a peak (Pierson and Moskowitz 1964 in the wave number), rolled off at the short end.
    amp = k ** -FALL * np.exp(-0.625 * (PEAK / k) ** 2) * np.exp(-(k / shortest) ** 2)
    amp[0, 0] = 0.0
    h = amp * (rng.normal(size=k.shape) + 1j * rng.normal(size=k.shape))
    sx = np.real(np.fft.ifft2(1j * kx * h))
    sy = np.real(np.fft.ifft2(1j * ky * h))
    dx = np.real(np.fft.ifft2(-1j * kx / k * h))
    dy = np.real(np.fft.ifft2(-1j * ky / k * h))
    # The displacement that just folds the surface: the most of its divergence.
    div = np.real(np.fft.ifft2(k * h))
    lam = CHOP / max(div.max(), 1e-9)
    # Each texel's slope moved to its displaced place, gathered back on the grid (bilinear, wrapping round).
    sx, sy = gathered([sx, sy], lam * dx, lam * dy)
    spread = np.sqrt(0.5 * (sx.var() + sy.var()))
    sx, sy = sx / spread, sy / spread
    # Caustics: each texel's light lands where its ray, bent by the slope, meets the bed.
    bend = BED_DEPTH * (1.0 - 1.0 / N_WATER)
    (light,) = gathered([np.ones_like(sx)], -bend * sx, -bend * sy, normalise=False)
    light = blurred(light) / light.mean()
    out = np.zeros((SIDE, SIDE, 3))
    out[:, :, 0] = 0.5 + 0.5 * np.clip(sx / SLOPE_RANGE, -1, 1)
    out[:, :, 1] = 0.5 + 0.5 * np.clip(sy / SLOPE_RANGE, -1, 1)
    c = np.clip(light / CAUSTIC_RANGE, 0, 1)
    out[:, :, 2] = c / (c.mean() * CAUSTIC_RANGE)
    print('caustics: spread', round(float(light.std()), 3), 'clipped', round(float((light > CAUSTIC_RANGE).mean()), 4))
    return out


def gathered(fields, ox, oy, normalise=True):
    """Each of `fields` moved texel by texel by (ox, oy) texels and gathered on the grid (bilinear, wrapping round);
    with `normalise`, over the weight each texel gathered (a texel none reached takes its own value)."""
    y, x = np.mgrid[0:SIDE, 0:SIDE].astype(float)
    tx, ty = x + ox, y + oy
    x0, y0 = np.floor(tx).astype(int), np.floor(ty).astype(int)
    fx, fy = tx - x0, ty - y0
    sums = [np.zeros((SIDE, SIDE)) for _ in fields]
    weight = np.zeros((SIDE, SIDE))
    for ddx, ddy, w in ((0, 0, (1 - fx) * (1 - fy)), (1, 0, fx * (1 - fy)), (0, 1, (1 - fx) * fy), (1, 1, fx * fy)):
        ix, iy = (x0 + ddx) % SIDE, (y0 + ddy) % SIDE
        np.add.at(weight, (iy, ix), w)
        for s, v in zip(sums, fields):
            np.add.at(s, (iy, ix), w * v)
    if not normalise:
        return sums
    return [np.where(weight > 1e-3, s / np.maximum(weight, 1e-3), v) for s, v in zip(sums, fields)]


def blurred(a):
    """`a` blurred by a texel (a Gaussian, wrapping round)."""
    f = np.fft.fftfreq(SIDE)
    fx, fy = np.meshgrid(f, f)
    return np.real(np.fft.ifft2(np.fft.fft2(a) * np.exp(-2 * (np.pi ** 2) * (fx ** 2 + fy ** 2))))


def even(a):
    """`a`'s texels ranked: 0..1, evenly."""
    r = np.empty(a.size)
    r[np.argsort(a.ravel(), kind='stable')] = (np.arange(a.size) + 0.5) / a.size
    return r.reshape(a.shape)


def grey(path):
    """The image at `path` shrunk to the layer's side, its lightness 0..1."""
    return np.asarray(Image.open(path).convert('L').resize((SIDE, SIDE), Image.LANCZOS)).astype(float) / 255


def foam_layer(d1, d2):
    """Layer 1: foam (module doc)."""
    out = np.zeros((SIDE, SIDE, 3))
    out[:, :, 0] = even(grey(f'{d1}/Foam001_1K-JPG_Opacity.jpg'))
    out[:, :, 1] = even(grey(f'{d2}/Foam002_1K-JPG_Opacity.jpg'))
    tone = grey(f'{d1}/Foam001_1K-JPG_Color.jpg')
    out[:, :, 2] = np.clip(0.5 * tone / tone.mean(), 0, 1)
    return out


def main():
    d1, d2, out = sys.argv[1:4]
    layers = np.concatenate([wave_layer(), foam_layer(d1, d2)], axis=0)
    Image.fromarray(np.floor(255 * np.clip(layers, 0, 1) + 0.5).astype(np.uint8), 'RGB').save(out, optimize=True)


main()
