"""The god pictures of Destruction's Fire tools: meteors, a volcano, a wildfire (god.py)."""
import numpy as np

import god_fx as F
import god_render as R
from god_scenes_land import carve
from god_world import blur, eroded, smooth

# The glow of molten rock and of embers (linear light added to the picture).
LAVA = np.array([6.0, 1.6, 0.25], np.float32)
EMBER = np.array([3.0, 0.9, 0.15], np.float32)


def trails(g, starts, rng, steps=400, wander=0.35):
    """Paths downhill from each start (x, y): each step one texel along the fall of the ground, turned a little at
    random; a field counting how often each texel was passed (the lava's or water's channels)."""
    gy, gx = np.gradient(blur(g.surface(), 2))
    out = np.zeros_like(g.height)
    for x, y in starts:
        fx, fy = g.texel(x, y)
        dx, dy = 0.0, 0.0
        for _ in range(steps):
            ix, iy = int(fx), int(fy)
            if not (1 <= ix < g.nx - 1 and 1 <= iy < g.ny - 1):
                break
            out[iy, ix] += 1
            ddx, ddy = -gx[iy, ix], -gy[iy, ix]
            ln = np.hypot(ddx, ddy) + 1e-6
            dx = dx * 0.6 + ddx / ln * 0.4 + rng.normal() * wander
            dy = dy * 0.6 + ddy / ln * 0.4 + rng.normal() * wander
            ln = np.hypot(dx, dy) + 1e-6
            fx, fy = fx + dx / ln, fy + dy / ln
    return out


def crater(g, x, y, radius, depth, rng):
    """A crater: a bowl `depth` deep, a raised rim, ejecta thrown out in rays. Returns its rays' field (0..1)."""
    r = np.hypot(g.X - x, g.Y - y) / radius
    ang = np.arctan2(g.Y - y, g.X - x)
    g.height[:] += -depth * np.clip(1 - r * r, 0, 1) + depth * 0.35 * np.exp(-((r - 1) / 0.22) ** 2)
    spokes = sum(np.cos(ang * k + rng.uniform(0, 6.3)) for k in (7, 13, 23)) / 3
    return np.clip(spokes * 1.5, 0, 1) * smooth(3.5, 1.0, r) * smooth(0.9, 1.2, r)


def meteor(kit):
    """A meteor shower striking land: fireballs falling from the upper left with glowing trails, fresh craters
    glowing red at their floors, dust and smoke rising."""
    v = kit.view(pitch=48, fov=34, dist=950, ty=30)
    g = R.Ground(v.ground_box(margin=0.1), 3072)
    g.height[:] = eroded(g.height.shape, 600 / g.cell, kit.rng, 7, 0.5, 2.0) * 25 + kit.noise(g, 40, 3) * 1.5
    kit.paint(g, 'ground', mix_turn=0.8)
    kit.paint(g, 'grass', smooth(-0.1, 0.4, kit.noise(g, 300, 3)) * 0.8, mix_turn=1.2)
    hits = kit.spec['hits']
    burnt = np.zeros_like(g.height)
    hot = np.zeros_like(g.height)
    for x, y, rad, dep in hits:
        rays = crater(g, x, y, rad, dep, kit.rng)
        r = np.hypot(g.X - x, g.Y - y) / rad
        burnt = np.maximum(burnt, np.maximum(rays * 0.85, smooth(2.0, 0.9, r)))
        hot = np.maximum(hot, smooth(0.75, 0.15, r) * (0.35 + 0.65 * smooth(-0.3, 0.5, kit.noise(g, rad * 0.3, 3))))
    kit.paint(g, 'scorch', burnt, mix_turn=0.6)
    g.albedo[:] *= 1 - hot[..., None] * 0.6
    g.glow[:] += LAVA * hot[..., None] ** 1.5 * 0.7
    items = kit.scatter(g, 8, smooth(0.5, 0.0, burnt), 'round', (12, 18))
    buf = kit.shoot(v, g, items, haze=(0.28, 0.27, 0.28), haze_far=0.35, exposure=0.9)
    smoke = []
    for x, y, rad, dep in hits:
        for k in range(int(rad / 4)):
            smoke.append({'x': x + k * rad * 0.15 + kit.rng.normal() * rad * 0.2,
                          'y': y + k * rad * 0.1 + kit.rng.normal() * rad * 0.2, 'z': k * rad * 0.3,
                          'r': rad * (0.5 + k * 0.08), 'a': 0.55, 'col': (0.2, 0.17, 0.15),
                          'glow': LAVA * 0.3 / (1 + k)})
    F.smoke(v, buf, smoke, kit.rng)
    for x, y, z, ln in kit.spec['falling']:
        head = np.array([[x, y, z]], np.float32)
        tail = head + np.array([[-0.55, 0.25, 0.8]], np.float32) * ln
        hx, hy, _ = v.project(head)
        tx, ty, _ = v.project(tail)
        F.streak(buf['glow'], (float(tx[0]), float(ty[0])), (float(hx[0]), float(hy[0])), 6.0, (1.2, 0.45, 0.12))
        F.streak(buf['glow'], (float(tx[0]), float(ty[0])), (float(hx[0]), float(hy[0])), 1.6, (4.0, 2.6, 1.3))
        F.glow_dots(v, buf, head, (20.0, 11.0, 5.0), 3.0, z_test=False)
    return R.finish(buf, {'saturation': 1.05, 'bloom': 1.2})


def volcano(kit):
    """An erupting cone: lava running down its gullies from a glowing crater, a column of ash rising and leaning
    with the wind, dark rock and ash on its flanks."""
    v = kit.view(pitch=46, fov=34, dist=7000, ty=200)
    g = R.Ground(v.ground_box(margin=0.1), 3072)

    def shape(X, Y, noise):
        r = np.hypot(X, Y - 250) / 2600
        cone = 1500 * np.clip(1 - r, 0, 1) ** 1.6 + 60 * noise(900, 4)
        cone -= 260 * smooth(0.12, 0.05, r)
        return cone + 120 * noise(3000, 4) + 40
    h, flow = carve(kit, g, shape, drops=180_000)
    g.height[:] = h + eroded(h.shape, 250 / g.cell, kit.rng, 5, 0.5, 2.0) * 25
    kit.paint(g, 'rock', mix_turn=0.9, tint=(0.55, 0.52, 0.5))
    kit.paint(g, 'ash', smooth(300, 900, g.height) * 0.75, mix_turn=1.2)
    kit.paint(g, 'land', smooth(260, 60, g.height), mix_turn=1.0)
    r = np.hypot(g.X, g.Y - 250) / 2600
    ang = kit.rng.uniform(np.pi * 0.4, np.pi * 1.6, 9)
    starts = [(np.sin(a) * 2600 * 0.09, 250 + np.cos(a) * 2600 * 0.09) for a in ang]
    lava = blur(np.clip(trails(g, starts, kit.rng, 1400, 0.5), 0, 1), 3)
    lava = np.clip(lava * 5, 0, 1)
    crust = np.clip(blur(lava, 8) * 3, 0, 1)
    g.albedo[:] *= 1 - crust[..., None] * 0.85
    broken = smooth(-0.4, 0.5, kit.noise(g, 50, 4))
    heat = smooth(0.8, 0.05, r)
    g.glow[:] += LAVA * (lava * broken * (0.15 + 0.85 * heat))[..., None] * np.array([1, 0.8, 0.6], np.float32)
    g.glow[:] += LAVA * smooth(0.06, 0.035, r)[..., None] * 0.6
    buf = kit.shoot(v, g, [], haze=(0.3, 0.29, 0.3), haze_far=0.3)
    plume = []
    top = float(g.at(g.height, np.array([0.0]), np.array([250.0]))[0])
    for k in range(90):
        t = k / 90
        plume.append({'x': 600 * t ** 1.5 + kit.rng.normal() * 80 * (0.3 + t),
                      'y': 250 + 1200 * t + kit.rng.normal() * 50,
                      'z': top + 3400 * t, 'r': 260 + 900 * t, 'a': 0.8, 'col': (0.22, 0.21, 0.2),
                      'glow': LAVA * 0.35 * (1 - t) ** 3})
    F.smoke(v, buf, plume, kit.rng)
    F.glow_dots(v, buf, [[0, 250, top]], (2.0, 0.6, 0.12), 90.0)
    return R.finish(buf, {'saturation': 1.05, 'exposure': 0.85, 'bloom': 1.3})


def fire(kit):
    """A wildfire front through dry grass and trees: flames along a ragged line, black ground and charred snags
    behind it, a pall of smoke lit orange from below."""
    v = kit.view(pitch=50, fov=34, dist=420, ty=10)
    g = R.Ground(v.ground_box(margin=0.1), 3072)
    g.height[:] = kit.noise(g, 300, 4) * 10
    kit.paint(g, 'dry', mix_turn=0.9)
    kit.paint(g, 'grass', smooth(-0.1, 0.4, kit.noise(g, 90, 3)) * 0.7, mix_turn=1.3)
    wob = kit.noise(g, 120, 3)
    line = g.Y - (40 - g.X * 0.25 + 25 * wob)
    burnt = smooth(-4, 6, line)
    kit.paint(g, 'burnt', burnt, mix_turn=0.6)
    g.albedo[:] *= 1 - burnt[..., None] * 0.45
    g.glow[:] += EMBER * (smooth(9, 0, line) * smooth(-6, 0, line) * smooth(-0.2, 0.5, kit.noise(g, 6, 3)))[..., None]
    g.glow[:] += EMBER * 0.35 * (burnt * smooth(30, 5, line) * smooth(0.5, 0.85, kit.noise(g, 4, 3) * 0.5 + 0.5))[
        ..., None]
    alive = 1 - smooth(-6, 2, line)
    snags = kit.scatter(g, 70, burnt, [0], (7, 11))
    for it in snags:
        it['img'] = snag(kit, it)
    items = (kit.scatter(g, 90, alive, 'round', (9, 14), tint=((1.15, 1.0, 0.7), (1.0, 0.95, 0.75))) +
             kit.scatter(g, 300, alive, 'dead_bush', (1.0, 2.0)) + snags)
    buf = kit.shoot(v, g, items, haze=(0.26, 0.24, 0.22), haze_far=0.3)
    xs = np.linspace(g.x0, g.x0 + g.nx * g.cell, 110)
    spots, puffs = [], []
    for x in xs:
        y = 40 - x * 0.25
        for _ in range(4):
            y = float(40 - x * 0.25 + 25 * g.at(wob, np.array([x]), np.array([y]))[0])
        for _ in range(2):
            fx, fy = x + kit.rng.normal() * 2, y + kit.rng.normal() * 2.5
            z = float(g.at(g.height, np.array([fx]), np.array([fy]))[0])
            spots.append({'x': fx, 'y': fy, 'z': z, 'h': kit.rng.uniform(3, 10), 'a': 0.8})
        if kit.rng.random() < 0.35:
            z0 = float(g.at(g.height, np.array([x]), np.array([y]))[0])
            for k in range(10):
                puffs.append({'x': x + 10 * k + kit.rng.normal() * 6, 'y': y + 14 * k, 'z': z0 + 8 + 16 * k,
                              'r': 12 + 6 * k, 'a': 0.75, 'col': (0.075, 0.068, 0.062),
                              'glow': EMBER * (1.2 / (1 + k * 1.5))})
    F.smoke(v, buf, puffs, kit.rng)
    F.flames(v, buf, spots, kit.rng)
    return R.finish(buf, {'saturation': 1.05, 'exposure': 0.95, 'bloom': 1.1})


# The charred trunks of snags.png: its first layers, a whole trunk each.
SNAG_TRUNKS = 6
_snags = {}


def snag(kit, it):
    """A charred trunk sprite (snags.png) for a tree that burned (rgba, linear, cut to its content); names it in
    `it`."""
    from god_world import strip_layer, to_linear
    k = int(kit.rng.integers(0, SNAG_TRUNKS))
    if k not in _snags:
        im = np.asarray(strip_layer('snags', k).convert('RGBA'), np.float32) / 255
        ys, xs = np.nonzero(im[..., 3] > 0.05)
        im = im[ys.min():ys.max() + 1, xs.min():xs.max() + 1]
        _snags[k] = np.concatenate([to_linear(im[..., :3]) * 1.5, im[..., 3:]], 2)
    it['id'] = f'snag{k}'
    return _snags[k]


SCENES = {'meteor': meteor, 'volcano': volcano, 'fire': fire}
