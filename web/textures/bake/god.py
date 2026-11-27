"""god.py DL OUT [name...]: makes the god mode's pictures (web/textures/god/, SOURCES.md `## god/`) from the game's
textures and the downloads in DL (`god_fetch.sh DL`), each as god.json says, into OUT: NAME.jpg and its smaller twin.

Each picture is a small scene (god_scenes_*.py): a height field shaped and painted with textures, sprites stood on
it, rendered from above by god_render.py, then effects (god_fx.py) laid over it. With names, only those are made.
"""
import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import numpy as np  # noqa: E402

import god_render as R  # noqa: E402
import god_sprites as P  # noqa: E402
from god_world import HERE, Tex, load_grey, load_rgb, smooth, strip_layer, to_linear  # noqa: E402

# Textures are read at most this many texels a side (enough for a picture 1440 wide).
TEX_SIZE = 1024
# A sky panorama is read at this size (it is only seen blurred, in water).
SKY_SIZE = (1024, 512)


class Kit:
    """What a scene draws with: its spec from god.json, its random draws, its textures and the sprites."""

    def __init__(self, name, spec, dl, sprites, relief):
        self.name, self.spec, self.dl, self.sprites, self.relief = name, spec, dl, sprites, relief
        self.rng = np.random.default_rng(spec.get('seed', 1))
        self._tex = {}

    def tex(self, role):
        """The texture the spec names for `role`: [source, side in metres]. A source `ground:N`, `carpets:N` or
        `fire:N` is layer N of that game strip; any other is a download in DL (its diffuse, and its relief where
        listed under "relief")."""
        if role in self._tex:
            return self._tex[role]
        src, side, *tint = self.spec['textures'][role]
        if ':' in src:
            group, i = src.split(':')
            im = strip_layer(group, int(i)).convert('RGB')
            rgb = to_linear(np.asarray(im, np.float32) / 255)
            t = Tex(rgb, side)
        else:
            rgb = load_rgb(os.path.join(self.dl, f'{src}_diff_2k.jpg'), size=TEX_SIZE)
            disp = os.path.join(self.dl, f'{src}_disp_2k.png')
            t = Tex(rgb, side, load_grey(disp, TEX_SIZE) if src in self.relief else None)
        if tint:
            t.rgb = [m * np.array(tint[0], np.float32) for m in t.rgb]
        self._tex[role] = t
        return t

    def view(self, pitch=52.0, fov=34.0, dist=1000.0, tx=0.0, ty=0.0, small=False):
        """A camera for a picture (`small`: a plant kind's card)."""
        w, h = (R.KIND_SIZE if small else R.FULL)
        return R.View(int(w * R.SUPER), int(h * R.SUPER), pitch, fov, dist, tx, ty)

    def paint(self, g, role, mask=None, turn=0.0, tint=(1, 1, 1), mix_turn=None, sat=1.0, cliffs=False):
        """Lay texture `role` over the ground's colour where `mask` (0..1) says; with `mix_turn`, blended with
        itself turned and shifted by a slow noise, so its repeat does not show; with `cliffs`, also laid on the
        steep faces from the side (triplanar), so it does not smear down them."""
        t = self.tex(role)
        c = t.colour(g.X, g.Y, g.cell, turn)
        if cliffs:
            n = g.normals(g.height)
            w = np.abs(n) ** 4
            w /= w.sum(axis=2, keepdims=True)
            cx = t.colour(g.Y, g.height, g.cell, turn)
            cy = t.colour(g.X, g.height + t.side * 0.5, g.cell, turn)
            c = c * w[..., 2:3] + cx * w[..., 0:1] + cy * w[..., 1:2]
        if mix_turn is not None:
            c2 = t.colour(g.X + t.side * 0.37, g.Y - t.side * 0.61, g.cell, turn + mix_turn)
            m = self.noise(g, t.side * 1.5, octaves=2)
            w = np.clip(m * 1.5 + 0.5, 0, 1)[..., None]
            c = c * (1 - w) + c2 * w
        c = c * np.array(tint, np.float32)
        if sat != 1.0:
            lum = c.mean(axis=-1, keepdims=True)
            c = lum + (c - lum) * sat
        if mask is None:
            g.albedo[:] = c
        else:
            m = np.clip(mask, 0, 1)[..., None]
            g.albedo[:] = g.albedo * (1 - m) + c * m

    def detail(self, g, role, strength=1.0):
        """Fine detail over the ground's colour: its light scaled by texture `role`'s lightness over its mean."""
        t = self.tex(role)
        lum = t.colour(g.X, g.Y, g.cell).mean(axis=2)
        g.albedo[:] *= ((lum / max(float(lum.mean()), 1e-6)) ** strength)[..., None]

    def crack(self, g, size, mask, width=0.06, depth=0.15):
        """Cracks over the ground where `mask` (0..1) says: a network of cells `size` metres across, the gaps between
        them dark and sunk, each plate's colour a little its own."""
        from god_world import cells
        edge, own = cells(g.X, g.Y, size)
        gap = smooth(width * size, 0, edge) * mask
        g.albedo[:] *= ((1 - gap * 0.8) * (1 + (own - 0.5) * 0.2 * mask))[..., None]
        g.height[:] -= gap * depth * size

    def noise(self, g, metres, octaves=5, gain=0.5, ridged=False):
        """Fractal noise over the ground, its largest features `metres` across."""
        from god_world import fbm
        return fbm(g.height.shape, metres / g.cell, self.rng, octaves, gain, ridged)

    def scatter(self, g, density, where, ids, size, tint=None, jitter=0.25):
        """Sprites drawn from the set or list `ids`, about `density` per 100 m x 100 m of ground where the field
        `where` (0..1) allows, each `size` metres high (low, high); `tint` a colour range ((r,g,b), (r,g,b))."""
        ids = self.sprites.ids(ids)
        area = g.nx * g.ny * g.cell * g.cell
        n = self.rng.poisson(density * area / 1e4)
        xs = g.x0 + self.rng.random(n) * g.nx * g.cell
        ys = g.y0 + self.rng.random(n) * g.ny * g.cell
        keep = self.rng.random(n) < g.at(where, xs, ys)
        out = []
        for x, y in zip(xs[keep], ys[keep]):
            it = {'x': float(x), 'y': float(y), 'id': int(self.rng.choice(ids)),
                  'size': float(self.rng.uniform(*size)), 'flip': bool(self.rng.random() < 0.5)}
            if tint:
                a = self.rng.random()
                it['tint'] = tuple(np.array(tint[0]) * (1 - a) + np.array(tint[1]) * a)
            out.append(it)
        return out

    def shoot(self, view, g, items, haze=(0.3, 0.36, 0.46), haze_far=0.35, gain=1.0, exposure=1.0):
        """Light the ground (with the sprites' shadows), render it and stand the sprites on it: the buffers."""
        P.stamp_shadows(g, self.sprites, items)
        g.light()
        buf = R.render(view, g, haze, haze_far, exposure)
        P.draw_sprites(view, g, buf, self.sprites, items, gain)
        return buf


def load_sky(path):
    """A sky panorama (Poly Haven's tone-mapped JPG) as linear colour, shrunk to `SKY_SIZE`."""
    from PIL import Image
    Image.MAX_IMAGE_PIXELS = None
    im = Image.open(path)
    im.draft('RGB', (SKY_SIZE[0] * 2, SKY_SIZE[1] * 2))
    im = im.convert('RGB').resize(SKY_SIZE, Image.LANCZOS)
    return to_linear(np.asarray(im, np.float32) / 255)


def main():
    dl, out = sys.argv[1], sys.argv[2]
    only = sys.argv[3:]
    spec = json.load(open(os.path.join(HERE, 'god.json')))
    import god_scenes_fire
    import god_scenes_land
    import god_scenes_life
    import god_scenes_weather
    scenes = {**god_scenes_land.SCENES, **god_scenes_fire.SCENES, **god_scenes_weather.SCENES,
              **god_scenes_life.SCENES}
    sprites = P.Sprites()
    os.makedirs(out, exist_ok=True)
    for name, ps in spec['pictures'].items():
        if only and name not in only:
            continue
        kit = Kit(name, ps, dl, sprites, spec['relief'])
        R.SKY_MAP = load_sky(os.path.join(dl, ps['sky'] + '.jpg')) if 'sky' in ps else None
        srgb = scenes[ps['scene']](kit)
        small = name.startswith('kind-')
        sizes = R.save(srgb, os.path.join(out, name), *((R.KIND_OUT, R.KIND_SMALL, R.KIND_BYTES, R.KIND_SMALL_BYTES)
                                                       if small else ()))
        print(name, *sizes)


if __name__ == '__main__':
    main()
