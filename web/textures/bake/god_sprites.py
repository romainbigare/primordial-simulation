"""The page's sprites in the god pictures (god.py): read from the game's strips, stood on the ground with their
shadows laid away from the sun."""
import re

import numpy as np
from PIL import Image

from god_render import SUN, SUN_ELEVATION, fog_at
from god_world import LAYER, TEXTURES, strip_layer, to_linear

SPRITE_TABLE = f'{TEXTURES}/../shaders/lib/sprites.wgsl'
# A sprite's foot stands this many texels above its cell's bottom (bake/sprite.html).
FOOT_TEXELS = 2


class Sprites:
    """The page's sprites (trees*.png, models*.png; their table web/shaders/lib/sprites.wgsl): each cut to its content,
    linear colour and cover, its foot at the bottom middle."""

    def __init__(self):
        src = open(SPRITE_TABLE).read()
        self.trees = int(re.search(r'const TREES = (\d+);', src).group(1))
        self.sets = {m.group(1).lower(): (int(m.group(2)), int(m.group(3))) for m in
                     re.finditer(r'const (\w+)_FROM = (\d+); const \w+_N = (\d+);', src)}
        self.w = self._table(src, 'MODEL_W_TABLE')
        self.h = self._table(src, 'MODEL_H_TABLE')
        self.cache = {}

    @staticmethod
    def _table(src, name):
        body = re.search(name + r'[^=]*= array<f32, \d+>\(([^)]*)\)', src).group(1)
        return [float(v) for v in body.split(',')]

    def ids(self, name):
        """The sprites of set `name` (`round`, `conifer`, ...), or a list given as it is."""
        if isinstance(name, list):
            return name
        a, n = self.sets[name]
        return list(range(a, a + n))

    def image(self, s):
        """Sprite `s` as (rgba floats, linear colour), cut to its content: a tree a whole layer, a model a quarter."""
        if s in self.cache:
            return self.cache[s]
        if s < self.trees:
            im, cell = strip_layer('trees', s), LAYER
        else:
            q = s - self.trees
            layer = strip_layer('models', q // 4)
            cell = LAYER // 2
            ox, oy = (q % 4) % 2 * cell, (q % 4) // 2 * cell
            im = layer.crop((ox, oy, ox + cell, oy + cell))
        w = max(2, int(round(self.w[s] * cell)))
        h = max(2, int(round(self.h[s] * cell)))
        x0 = (cell - w) // 2
        y1 = cell - FOOT_TEXELS
        im = im.crop((x0, max(0, y1 - h), x0 + w, y1))
        a = np.asarray(im.convert('RGBA'), np.float32) / 255
        rgba = np.concatenate([to_linear(a[..., :3]), a[..., 3:]], axis=2)
        self.cache[s] = rgba
        return rgba


def sprite_rgba(sprites, it):
    """An item's picture: its own `img` (rgba, linear) where it has one, else its sprite's."""
    return it['img'] if 'img' in it else sprites.image(it['id'])


def draw_sprites(view, g, buf, sprites, items, gain=1.0):
    """Stand the sprites `items` (dicts: x, y, id or img, size in metres, and optional tint, flip, z or sink) on the
    ground, far ones first, each hidden where the ground stands in front of it, dimmed where the ground's shadow
    falls on its foot."""
    if not items:
        return
    xs = np.array([i['x'] for i in items], np.float32)
    ys = np.array([i['y'] for i in items], np.float32)
    ground = g.at(g.height, xs, ys)
    zs = np.array([i.get('z', z - i.get('sink', 0.0)) for i, z in zip(items, ground)], np.float32)
    sx, sy, dz = view.project(np.stack([xs, ys, zs], 1))
    lit = g.at(g.sun_lit, xs, ys)
    rgb, depth = buf['rgb'], buf['depth']
    hz = buf['haze']
    for k in np.argsort(-dz):
        it = items[k]
        img = sprite_rgba(sprites, it)
        ph = it['size'] * view.f / dz[k] * 0.9
        if ph < 2:
            continue
        pw = ph * img.shape[1] / img.shape[0]
        sp = resized(img, it.get('id', id(img)), int(round(pw)), int(round(ph)), it.get('flip', False))
        h, w = sp.shape[:2]
        x0, y0 = int(round(sx[k] - w / 2)), int(round(sy[k] - h))
        ya, yb, xa, xb = max(y0, 0), min(y0 + h, view.h), max(x0, 0), min(x0 + w, view.w)
        if ya >= yb or xa >= xb:
            continue
        s = sp[ya - y0:yb - y0, xa - x0:xb - x0]
        a = s[..., 3:4] * (depth[ya:yb, xa:xb, None] > dz[k] - it['size'] * 0.6)
        light = (0.45 + 0.55 * lit[k]) * gain
        col = s[..., :3] * light * np.array(it.get('tint', (1, 1, 1)), np.float32)
        fog = fog_at(dz[k], view, buf['haze_far'])
        col = col * (1 - fog) + hz * fog
        rgb[ya:yb, xa:xb] = rgb[ya:yb, xa:xb] * (1 - a) + col * a
        solid = a[..., 0] > 0.5
        depth[ya:yb, xa:xb][solid] = dz[k]


_resized = {}


def resized(img, key, w, h, flip):
    """The picture `img` (rgba) resized to w x h (premultiplied while resized, so its edges do not darken), mirrored
    if asked; kept by `key`."""
    key = (key, w, h, flip)
    if key in _resized:
        return _resized[key]
    a = img[..., 3]
    chans = [img[..., c] * a for c in range(3)] + [a]
    out = np.stack([np.asarray(Image.fromarray(np.ascontiguousarray(c, np.float32), 'F').resize(
        (max(w, 1), max(h, 1)), Image.LANCZOS)) for c in chans], 2)
    out = np.clip(out, 0, None)
    al = np.clip(out[..., 3:4], 0, 1)
    out = np.concatenate([out[..., :3] / np.maximum(al, 1e-4), al], 2)
    if flip:
        out = out[:, ::-1]
    _resized[key] = out
    return out


def stamp_shadows(g, sprites, items, strength=0.75):
    """Each sprite's shadow on the ground: its cover laid flat away from the sun, as long as its height over the sun's
    rise, multiplied into the ground's light."""
    lenf = 1 / np.tan(np.radians(SUN_ELEVATION))
    away = -SUN[:2] / np.hypot(SUN[0], SUN[1])
    ang = np.degrees(np.arctan2(away[0], away[1]))
    for it in items:
        img = sprite_rgba(sprites, it)
        hgt = it['size'] * lenf
        wid = it['size'] * img.shape[1] / img.shape[0]
        ph, pw = int(hgt / g.cell) + 1, int(wid / g.cell) + 1
        if ph < 2 or pw < 2 or ph > 400:
            continue
        a = Image.fromarray(np.ascontiguousarray(img[::-1, :, 3]), 'F').resize((pw, ph), Image.BILINEAR)
        a = np.asarray(a.rotate(-ang, Image.BILINEAR, expand=True))
        cx, cy = g.texel(it['x'], it['y'])
        oy = cy - a.shape[0] / 2 + away[1] * ph / 2
        ox = cx - a.shape[1] / 2 + away[0] * ph / 2
        x0, y0 = int(round(ox)), int(round(oy))
        ya, yb, xa, xb = max(y0, 0), min(y0 + a.shape[0], g.ny), max(x0, 0), min(x0 + a.shape[1], g.nx)
        if ya >= yb or xa >= xb:
            continue
        sub = a[ya - y0:yb - y0, xa - x0:xb - x0]
        g.shade[ya:yb, xa:xb] *= 1 - np.clip(sub, 0, 1) * strength
