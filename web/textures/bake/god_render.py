"""The god pictures' camera and renderer (god.py): a height field ray-marched from a camera looking north and down,
lit by a warm sun from the north-west (upper left on the picture) and a blue sky, with cast shadows, ambient
occlusion, water, sprites standing on it and haze; then graded, vignetted and saved."""
import io

import numpy as np
from PIL import Image, ImageFilter

from god_world import bilerp, blur, mips, sample_mips, to_srgb, smooth

# The sun: its height over the horizon (degrees) and its direction, from the north-west.
SUN_ELEVATION = 38.0
SUN = np.array([-np.cos(np.radians(SUN_ELEVATION)) * 0.7071, np.cos(np.radians(SUN_ELEVATION)) * 0.7071,
                np.sin(np.radians(SUN_ELEVATION))], np.float32)
SUN_LIGHT = np.array([1.7, 1.55, 1.36], np.float32)
SKY_LIGHT = np.array([0.26, 0.32, 0.44], np.float32)
# Ray marching: at most this many steps, each this share of the safe step (the ray's height over the ground over the
# steepest climb it can meet).
MARCH_STEPS = 400
MARCH_SAFETY = 0.8
# The pictures are rendered this many times larger than they are saved, then shrunk (anti-aliasing).
SUPER = 1.5
# The size a tool's picture is drawn at, and the size it is saved at (the page's thumbnail) with its file's byte budget.
FULL = (960, 540)
SMALL, SMALL_BYTES = (480, 270), 50_000
# The same for a plant kind's picture.
KIND_SIZE = (320, 240)
KIND_SMALL, KIND_SMALL_BYTES = (160, 120), 15_000
JPEG_QUALITY = 82


class View:
    """A pinhole camera looking north, `pitch` degrees down, its picture `w` x `h`, `fov` degrees high, `dist` metres
    from the ground point (tx, ty) its centre sees."""

    def __init__(self, w, h, pitch, fov, dist, tx=0.0, ty=0.0):
        self.w, self.h = w, h
        p = np.radians(pitch)
        self.fwd = np.array([0, np.cos(p), -np.sin(p)], np.float32)
        self.right = np.array([1, 0, 0], np.float32)
        self.up = np.array([0, np.sin(p), np.cos(p)], np.float32)
        self.cam = np.array([tx, ty, 0], np.float32) - self.fwd * dist
        self.f = (h / 2) / np.tan(np.radians(fov) / 2)

    def rays(self):
        """Each pixel's unit direction, (h, w, 3)."""
        j = (np.arange(self.w, dtype=np.float32) + 0.5 - self.w / 2) / self.f
        i = (self.h / 2 - np.arange(self.h, dtype=np.float32) - 0.5) / self.f
        d = self.fwd[None, None] + j[None, :, None] * self.right + i[:, None, None] * self.up
        return d / np.linalg.norm(d, axis=2, keepdims=True)

    def project(self, p):
        """World points (n, 3) to pixels: (sx, sy, depth along the view)."""
        r = p - self.cam
        z = r @ self.fwd
        return self.w / 2 + self.f * (r @ self.right) / z, self.h / 2 - self.f * (r @ self.up) / z, z

    def ground_box(self, z=0.0, margin=0.08):
        """The box (x0, x1, y0, y1) of the plane at height z the picture sees, grown by `margin` of its size."""
        pts = []
        for i in (0, self.h - 1):
            for j in (0, self.w - 1):
                yv = (self.h / 2 - i - 0.5) / self.f
                xv = (j + 0.5 - self.w / 2) / self.f
                d = self.fwd + xv * self.right + yv * self.up
                t = (z - self.cam[2]) / d[2]
                pts.append(self.cam + t * d)
        pts = np.array(pts)
        x0, x1, y0, y1 = pts[:, 0].min(), pts[:, 0].max(), pts[:, 1].min(), pts[:, 1].max()
        mx, my = (x1 - x0) * margin, (y1 - y0) * margin
        return x0 - mx, x1 + mx, y0 - my, y1 + my


class Ground:
    """A height field over a box, `n` texels along its longer side: the ground's height, the water's level, the
    ground's colour (albedo), its glow, the water's foam and colour, and the light it gets (shadow, occlusion)."""

    def __init__(self, box, n):
        x0, x1, y0, y1 = box
        self.cell = max(x1 - x0, y1 - y0) / n
        self.nx, self.ny = int((x1 - x0) / self.cell) + 1, int((y1 - y0) / self.cell) + 1
        self.x0, self.y0 = x0, y0
        xs = x0 + np.arange(self.nx, dtype=np.float32) * self.cell
        ys = y0 + np.arange(self.ny, dtype=np.float32) * self.cell
        self.X, self.Y = np.meshgrid(xs, ys)
        shape = (self.ny, self.nx)
        self.height = np.zeros(shape, np.float32)
        self.water = np.full(shape, -1e9, np.float32)
        self.albedo = np.full(shape + (3,), 0.2, np.float32)
        self.glow = np.zeros(shape + (3,), np.float32)
        self.foam = np.zeros(shape, np.float32)
        self.murk = np.zeros(shape, np.float32)
        self.shade = np.ones(shape, np.float32)
        self.ripple = 1.0
        self.water_tint = np.array([0.012, 0.045, 0.06], np.float32)
        self.water_absorb = np.array([0.45, 0.16, 0.12], np.float32)

    def texel(self, x, y):
        """World (x, y) to fractional texels."""
        return (x - self.x0) / self.cell, (y - self.y0) / self.cell

    def at(self, a, x, y):
        """Field `a` at world points, held at the edges."""
        fx, fy = self.texel(x, y)
        return bilerp(a, fx, fy, False)

    def surface(self):
        return np.maximum(self.height, self.water)

    def light(self):
        """The cast shadows (soft, by marching each texel towards the sun) and the ambient occlusion."""
        s = self.surface()
        half = np.ascontiguousarray(s[::2, ::2])
        step = self.cell * 2
        sx, sy = SUN[0] / np.hypot(SUN[0], SUN[1]), SUN[1] / np.hypot(SUN[0], SUN[1])
        rise = SUN[2] / np.hypot(SUN[0], SUN[1])
        lit = np.ones_like(half)
        span = max(half.shape) * step * 0.35
        hi = half.max()
        fy, fx = np.mgrid[0:half.shape[0], 0:half.shape[1]].astype(np.float32)
        d = step
        while d < span:
            ray = half + d * rise
            if ray.min() > hi:
                break
            g = bilerp(half, fx + sx * d / step, fy + sy * d / step, False)
            lit = np.minimum(lit, np.clip((ray - g) / (d * 0.12) + 0.5, 0, 1))
            d += step * (1 + d / span * 6)
        lit = Image.fromarray(np.ascontiguousarray(lit, np.float32), 'F')
        lit = np.asarray(lit.resize((self.nx, self.ny), Image.BILINEAR))
        ao = np.zeros_like(s)
        for r in (2, 8, 32):
            ao += np.clip((blur(s, r) - s) / (r * self.cell), 0, 1)
        self.sun_lit = lit * self.shade
        self.occlusion = np.clip(1 - ao * 0.9, 0.25, 1)

    def normals(self, s):
        gy, gx = np.gradient(s, self.cell)
        n = np.stack([-gx, -gy, np.ones_like(s)], axis=2)
        return n / np.linalg.norm(n, axis=2, keepdims=True)


# The sky panorama the water reflects (linear, equirectangular), set by god.py for each picture; None: a gradient.
SKY_MAP = None
# The panorama's brightness against the sun's light.
SKY_MAP_GAIN = 0.9


def sky(d, haze):
    """The sky's colour seen along unit directions `d` (n, 3): the panorama (or a blue zenith over a pale horizon)
    and the sun."""
    if SKY_MAP is not None:
        hh, ww = SKY_MAP.shape[:2]
        u = (np.arctan2(d[:, 0], d[:, 1]) / (2 * np.pi) + 0.5) * ww
        v = np.arccos(np.clip(d[:, 2], -1, 1)) / np.pi * hh
        col = bilerp(SKY_MAP, u, np.clip(v, 0, hh - 1), True) * SKY_MAP_GAIN
    else:
        up = np.clip(d[:, 2], 0, 1)[:, None]
        zen = np.array([0.18, 0.32, 0.62], np.float32)
        col = haze * (1 - up) ** 3 + zen * (1 - (1 - up) ** 3)
    sun = np.clip(d @ SUN, 0, 1)[:, None]
    return col + SUN_LIGHT * (sun ** 600 * 40 + sun ** 12 * 0.15)


def march(view, g, rays):
    """Each ray's distance to the ground or water (inf where it misses)."""
    s = g.surface()
    zmax, zmin = float(s.max()) + 0.01, float(s.min()) - 0.01
    gy, gx = np.gradient(s, g.cell)
    climb = float(np.percentile(np.hypot(gx, gy), 99.8)) * 1.3 + 0.05
    d = rays.reshape(-1, 3)
    c = view.cam
    t = np.maximum((c[2] - zmax) / -d[:, 2], 0)
    tend = (c[2] - zmin) / -d[:, 2]
    k = MARCH_SAFETY / (np.abs(d[:, 2]) + climb * np.hypot(d[:, 0], d[:, 1]))
    hit = np.full(d.shape[0], np.inf, np.float32)
    live = np.arange(d.shape[0])
    tl, kl = t, k
    for _ in range(MARCH_STEPS):
        p = c + tl[:, None] * d[live]
        fx, fy = g.texel(p[:, 0], p[:, 1])
        dh = p[:, 2] - bilerp(s, fx, fy, False)
        done = dh < g.cell * 0.02
        out = tl > tend[live]
        if done.any():
            back = np.maximum(tl[done] - np.maximum(dh[done], -g.cell) * kl[done] * 0.5, 0)
            hit[live[done]] = back
        keep = ~(done | out)
        live, tl, kl, dh = live[keep], tl[keep], kl[keep], dh[keep]
        if live.size == 0:
            break
        tl = tl + np.maximum(dh * kl, g.cell * 0.25)
    hit[live] = tl
    return hit.reshape(rays.shape[:2])


def render(view, g, haze=(0.3, 0.36, 0.46), haze_far=0.35, exposure=1.0):
    """The picture of ground `g` as linear colour, with its buffers: depth (inf on sky), world points, glow."""
    rays = view.rays()
    t = march(view, g, rays)
    hitm = np.isfinite(t)
    d = rays[hitm]
    p = view.cam + t[hitm][:, None] * d
    fx, fy = g.texel(p[:, 0], p[:, 1])
    hz = np.array(haze, np.float32)
    under = (bilerp(g.water, fx, fy, False) - bilerp(g.height, fx, fy, False))
    wet = under > g.cell * 0.01
    n = bilerp(g.normals(g.height), fx, fy, False)
    n /= np.linalg.norm(n, axis=1, keepdims=True)
    foot = t[hitm] / view.f / np.sqrt(np.maximum(-d[:, 2], 0.3))
    lod = np.log2(np.maximum(foot / g.cell, 1.0))
    if not hasattr(g, 'pyr'):
        g.pyr = mips(g.albedo)
        g.glow_pyr = mips(g.glow)
    alb = sample_mips(g.pyr, fx, fy, lod, False)
    lit = bilerp(g.sun_lit, fx, fy, False)[:, None]
    occ = bilerp(g.occlusion, fx, fy, False)[:, None]
    ndl = np.clip(n @ SUN, 0, 1)[:, None]
    light = SUN_LIGHT * ndl * lit + SKY_LIGHT * occ * (0.55 + 0.45 * n[:, 2:3])
    col = alb * light + sample_mips(g.glow_pyr, fx, fy, lod, False)
    if wet.any():
        col[wet] = shade_water(g, d[wet], p[wet], fx[wet], fy[wet], under[wet], col[wet], lit[wet], hz)
    dist = t[hitm][:, None]
    fog = fog_at(dist, view, haze_far)
    col = col * (1 - fog) + hz * fog
    out = np.zeros(rays.shape, np.float32)
    out[hitm] = col * exposure
    miss = ~hitm
    out[miss] = sky(rays[miss], hz)
    world = np.full(rays.shape, np.nan, np.float32)
    world[hitm] = p
    depth = np.where(hitm, t * np.einsum('ijk,k->ij', rays, view.fwd), np.inf).astype(np.float32)
    return {'rgb': out, 'depth': depth, 'world': world, 'glow': np.zeros(rays.shape, np.float32),
            'haze': hz, 'haze_far': haze_far}


def fog_at(dist, view, haze_far):
    """The share of haze over a point `dist` metres away: none nearer than the picture's middle, then rising to
    `haze_far` (aerial perspective)."""
    return haze_far * (1 - np.exp(-np.maximum(dist / view_scale(view) - 0.8, 0) * 2.2))


def view_scale(view):
    """The distance from the camera to the ground point at the picture's centre."""
    return float(np.linalg.norm(view.cam - np.array([view.cam[0], view.cam[1] + (-view.cam[2] / view.fwd[2]) *
                                                     view.fwd[1], 0])))


def shade_water(g, d, p, fx, fy, depth, bottom, lit, hz):
    """Water: the ground under it seen through (dimmed by depth, murk and the water's own colour), the sky reflected
    off a rippled surface by Fresnel's law (Schlick 1994), the sun's glint, and foam."""
    rip = g.ripple
    nx = np.sin(p[:, 0] * 0.9 / max(g.cell, 1e-3) * 0.25 + p[:, 1] * 0.31) * 0.03 * rip
    ny = np.sin(p[:, 1] * 1.3 / max(g.cell, 1e-3) * 0.25 - p[:, 0] * 0.2) * 0.03 * rip
    if hasattr(g, 'wave_n'):
        wn = bilerp(g.wave_n, fx, fy, False)
        nx, ny = nx + wn[:, 0], ny + wn[:, 1]
    n = np.stack([nx, ny, np.ones_like(nx)], axis=1)
    n /= np.linalg.norm(n, axis=1, keepdims=True)
    cos = np.clip(-(d * n).sum(1), 0.02, 1)[:, None]
    fres = 0.02 + 0.98 * (1 - cos) ** 5
    r = d - 2 * (d * n).sum(1)[:, None] * n
    r[:, 2] = np.abs(r[:, 2])
    refl = sky(r, hz) * (0.35 + 0.65 * lit)
    murk = bilerp(g.murk, fx, fy, False)[:, None]
    trans = np.exp(-depth[:, None] * g.water_absorb * (1 + murk * 6))
    scatter = g.water_tint * (SUN_LIGHT * 0.5 * lit + SKY_LIGHT) * (1 - trans)
    mud = np.array([0.16, 0.11, 0.06], np.float32) * (SUN_LIGHT * 0.5 * lit + SKY_LIGHT) * 0.5
    scatter = scatter * (1 - murk) + mud * murk * (1 - trans)
    col = bottom * trans + scatter
    col = col * (1 - fres) + refl * fres
    foam = bilerp(g.foam, fx, fy, False)[:, None]
    white = np.array([0.8, 0.82, 0.84], np.float32) * (SUN_LIGHT * 0.35 * lit + SKY_LIGHT * 0.9)
    return col * (1 - foam) + white * foam


def finish(buf, grade):
    """The linear picture to sRGB: bloom from its glow and brightest light, a filmic curve (Narkowicz 2015), the
    grade (saturation, contrast, warmth) and a soft darker vignette."""
    rgb = buf['rgb'] + buf['glow']
    bright = np.clip(rgb - grade.get('bloom_from', 1.4), 0, None) + buf['glow'] * 0.6
    h = rgb.shape[0]
    bloom = blur(bright, h * 0.006) * 0.5 + blur(bright, h * 0.03) * 0.35 + blur(bright, h * 0.09) * 0.25
    rgb = rgb + bloom * grade.get('bloom', 1.0)
    rgb = rgb * grade.get('exposure', 1.0)
    x = rgb
    rgb = np.clip((x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14), 0, 1)
    lum = (rgb @ np.array([0.2126, 0.7152, 0.0722], np.float32))[..., None]
    rgb = lum + (rgb - lum) * grade.get('saturation', 1.1)
    rgb = rgb * np.array(grade.get('tint', (1.0, 1.0, 1.0)), np.float32)
    s = to_srgb(np.clip(rgb, 0, 1))
    c = grade.get('contrast', 1.08)
    s = np.clip(0.5 + (s - 0.5) * c, 0, 1)
    hh, ww = s.shape[:2]
    yy, xx = np.mgrid[0:hh, 0:ww].astype(np.float32)
    r = np.sqrt(((xx - ww / 2) / (ww / 2)) ** 2 * 0.85 + ((yy - hh * 0.47) / (hh / 2)) ** 2)
    v = 1 - grade.get('vignette', 0.55) * smooth(0.55, 1.45, r)
    return np.clip(s * v[..., None], 0, 1)


def save(srgb, path_stem, small=SMALL, small_bytes=SMALL_BYTES):
    """The picture as `stem@<w>.jpg` at `small`, a JPEG at `JPEG_QUALITY`, lowered a step at a time where its file
    would pass its byte budget. Returns the file's size."""
    im = Image.fromarray((srgb * 255 + 0.5).astype(np.uint8), 'RGB')
    out = im.resize(small, Image.LANCZOS).filter(ImageFilter.UnsharpMask(radius=1.0, percent=40, threshold=2))
    q = JPEG_QUALITY
    while True:
        b = io.BytesIO()
        out.save(b, 'JPEG', quality=q, optimize=True, progressive=True)
        if b.tell() <= small_bytes or q <= 60:
            break
        q -= 3
    open(f'{path_stem}@{small[0]}.jpg', 'wb').write(b.getvalue())
    return b.tell()
