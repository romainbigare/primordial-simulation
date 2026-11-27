"""The god pictures of life: the Human and Animal categories, whose tools come later (god.py)."""
import numpy as np

import god_fx as F
import god_render as R
from god_scenes_land import course_distance
from god_scenes_fire import EMBER
from god_scenes_weather import fields
from god_world import blur, eroded, smooth


def house(kit, g, x, y, w, ln, wall, roof, turn, roof_role, roof_tint, wall_col):
    """A house on the ground: walls `wall` metres high round a w x ln floor turned `turn` radians, under a gabled
    roof rising `roof` metres to a ridge along its length, its roof painted with texture `roof_role`."""
    r = max(w, ln)
    x0, y0 = g.texel(x - r, y - r)
    x1, y1 = g.texel(x + r, y + r)
    xa, xb, ya, yb = max(int(x0), 0), min(int(x1) + 2, g.nx), max(int(y0), 0), min(int(y1) + 2, g.ny)
    if xa >= xb or ya >= yb:
        return
    X, Y = g.X[ya:yb, xa:xb], g.Y[ya:yb, xa:xb]
    c, s = np.cos(turn), np.sin(turn)
    u = (X - x) * c + (Y - y) * s
    v = -(X - x) * s + (Y - y) * c
    inside = (np.abs(u) < ln / 2) & (np.abs(v) < w / 2)
    base = float(g.at(g.height, np.array([x]), np.array([y]))[0])
    top = base + wall + roof * (1 - np.abs(v) / (w / 2))
    hh = g.height[ya:yb, xa:xb]
    hh[inside] = np.maximum(hh[inside], top[inside])
    t = kit.tex(roof_role)
    col = t.colour(u, v, g.cell) * np.array(roof_tint, np.float32)
    eave = inside & ((np.abs(v) > w / 2 - g.cell * 1.2) | (np.abs(u) > ln / 2 - g.cell * 1.2))
    al = g.albedo[ya:yb, xa:xb]
    al[inside] = col[inside]
    al[eave] = np.array(wall_col, np.float32)
    ring = (~inside) & (np.abs(u) < ln / 2 + 2.5) & (np.abs(v) < w / 2 + 2.5)
    al[ring] *= 0.8


def village(kit, g, cx, cy, n, spread, roofs, rng, size=(5, 9)):
    """`n` houses round (cx, cy), each turned near one of two street lines; returns their (x, y, w, ln, turn)."""
    out = []
    base_turn = rng.uniform(0, np.pi)
    tries = 0
    while len(out) < n and tries < n * 30:
        tries += 1
        r = spread * np.sqrt(rng.random())
        a = rng.uniform(0, 2 * np.pi)
        x, y = cx + r * np.cos(a), cy + r * np.sin(a)
        ln = rng.uniform(*size)
        w = ln * rng.uniform(0.55, 0.75)
        if any(np.hypot(x - o[0], y - o[1]) < (ln + o[3]) * 0.6 + 1.5 for o in out):
            continue
        turn = base_turn + (np.pi / 2 if rng.random() < 0.5 else 0) + rng.normal() * 0.08
        role, tint = roofs[int(rng.integers(len(roofs)))]
        house(kit, g, x, y, w, ln, rng.uniform(2.4, 3.2), w * 0.45, turn, role, tint, (0.42, 0.38, 0.32))
        out.append((x, y, w, ln, turn))
    return out


def paths(kit, g, cx, cy, n, length, width, rng):
    """Worn dirt tracks leading out from (cx, cy), each wandering a little: a field (0..1) of where they run."""
    m = np.zeros_like(g.height)
    for k in range(n):
        a = 2 * np.pi * k / n + rng.normal() * 0.3
        t = np.linspace(0, length, 300)
        wob = np.cumsum(rng.normal(0, 0.02, t.size))
        xs = cx + np.cos(a + wob) * t
        ys = cy + np.sin(a + wob) * t
        fx, fy = g.texel(xs, ys)
        ok = (fx >= 0) & (fx < g.nx) & (fy >= 0) & (fy < g.ny)
        np.add.at(m, (fy[ok].astype(int), fx[ok].astype(int)), 1.0)
    return np.clip(blur(m, width / g.cell) * width / g.cell * 3, 0, 1)


def settled(kit):
    """The settlement pictures' ground: rolling fields, a village of houses at the middle, tracks between."""
    v = kit.view(pitch=52, fov=34, dist=kit.spec.get('dist', 300), ty=5)
    g = R.Ground(v.ground_box(margin=0.1), 3072)
    g.height[:] = eroded(g.height.shape, 600 / g.cell, kit.rng, 5, 0.5, 2.0) * 6
    edge = fields(kit, g, ['crop', 'pasture', 'soil'], (40, 60), 0.3)
    kit.paint(g, 'green', smooth(45, 20, np.hypot(g.X, g.Y)), mix_turn=0.8)
    tracks = paths(kit, g, 0, 0, 5, 400, 1.6, kit.rng)
    kit.paint(g, 'track', tracks, mix_turn=0.5)
    return v, g, edge


def population(kit):
    """A village: thatched houses gathered along two lanes, fields round it, smoke from a few hearths."""
    v, g, edge = settled(kit)
    homes = village(kit, g, 0, 0, 34, 42, [('roof', (1, 1, 1)), ('roof', (0.8, 0.75, 0.7))], kit.rng)
    items = (kit.scatter(g, 60, smooth(3, 1, edge), 'round', (8, 12)) +
             kit.scatter(g, 20, smooth(40, 60, np.hypot(g.X, g.Y)), 'lobed', (8, 11)))
    buf = kit.shoot(v, g, items, haze_far=0.25)
    puffs = []
    for x, y, w, ln, turn in homes[::5]:
        for k in range(6):
            puffs.append({'x': x + k * 1.6, 'y': y + k * 1.0, 'z': 6 + k * 2.4, 'r': 1.6 + k * 0.7, 'a': 0.25,
                          'col': (0.55, 0.55, 0.56)})
    F.smoke(v, buf, puffs, kit.rng)
    return R.finish(buf, {'saturation': 1.08})


def war(kit):
    """A settlement burning: houses alight inside a broken stockade, black smoke climbing and leaning, the
    ground scorched."""
    v, g, edge = settled(kit)
    homes = village(kit, g, 0, 0, 30, 40, [('roof', (1, 1, 1)), ('roof', (0.7, 0.66, 0.62))], kit.rng)
    r = np.hypot(g.X, g.Y)
    ring = smooth(1.0, 0.3, np.abs(r - 52)) * (np.sin(np.arctan2(g.Y, g.X) * 160) > -0.2)
    broken = smooth(-0.2, 0.2, kit.noise(g, 40, 3))
    g.height[:] += ring * 3.5 * broken
    g.albedo[:] = g.albedo * (1 - ring[..., None] * broken[..., None]) + np.array([0.12, 0.08, 0.05]) * (
        ring * broken)[..., None]
    scorch = smooth(45, 25, r) * smooth(-0.3, 0.3, kit.noise(g, 25, 3))
    kit.paint(g, 'burnt', scorch, mix_turn=0.6)
    burning = [h for h in homes if kit.rng.random() < 0.45]
    for x, y, w, ln, turn in burning:
        m = smooth(6, 2, np.hypot(g.X - x, g.Y - y))
        g.albedo[:] *= 1 - m[..., None] * 0.6
        g.glow[:] += EMBER * 0.5 * m[..., None]
    buf = kit.shoot(v, g, kit.scatter(g, 40, smooth(3, 1, edge), 'round', (8, 12)), haze=(0.26, 0.24, 0.23),
                    haze_far=0.35, exposure=0.9)
    spots, puffs = [], []
    for x, y, w, ln, turn in burning:
        for _ in range(6):
            spots.append({'x': x + kit.rng.normal() * ln * 0.25, 'y': y + kit.rng.normal() * w * 0.25, 'z': 3.0,
                          'h': kit.rng.uniform(4, 8)})
        for k in range(14):
            puffs.append({'x': x + k * 2.6 + kit.rng.normal() * 1.5, 'y': y + k * 2.2, 'z': 7 + k * 5,
                          'r': 3.5 + k * 1.3, 'a': 0.6, 'col': (0.1, 0.09, 0.09), 'glow': EMBER * (1.2 / (1 + k))})
    F.smoke(v, buf, puffs, kit.rng)
    F.flames(v, buf, spots, kit.rng)
    return R.finish(buf, {'saturation': 1.0, 'exposure': 0.9, 'bloom': 1.1})


def culture(kit):
    """A circle of standing stones on a grassy rise, a fire burning at its heart, a ring worn into the turf by
    those who walk round it."""
    v = kit.view(pitch=52, fov=34, dist=110, ty=0)
    g = R.Ground(v.ground_box(margin=0.1), 2560)
    r = np.hypot(g.X, g.Y)
    g.height[:] = 4 * np.exp(-(r / 60) ** 2) + kit.noise(g, 40, 4) * 0.6
    kit.paint(g, 'meadow', mix_turn=0.9)
    kit.paint(g, 'track', smooth(1.6, 0.4, np.abs(r - 15.5 + 0.4 * kit.noise(g, 8, 2))) * 0.9, mix_turn=0.5)
    kit.paint(g, 'track', smooth(5, 2.5, r), mix_turn=0.5)
    kit.paint(g, 'burnt', smooth(2.2, 1.2, r), mix_turn=0.5)
    n = 22
    for k in range(n):
        a = 2 * np.pi * k / n + kit.rng.normal() * 0.03
        x, y = np.cos(a) * 13, np.sin(a) * 13
        stone(kit, g, x, y, kit.rng.uniform(1.4, 2.0), kit.rng.uniform(0.7, 1.0), kit.rng.uniform(3.0, 4.6),
              a + np.pi / 2 + kit.rng.normal() * 0.1)
    for k in range(5):
        a = kit.rng.uniform(0, 2 * np.pi)
        stone(kit, g, np.cos(a) * 26, np.sin(a) * 26, 1.6, 0.9, 1.0, kit.rng.uniform(0, 3))
    g.glow[:] += EMBER * smooth(1.2, 0.3, r)[..., None] * 1.5
    items = kit.scatter(g, 60, smooth(35, 50, r), 'round', (8, 12))
    buf = kit.shoot(v, g, items, haze_far=0.2)
    F.flames(v, buf, [{'x': kit.rng.normal() * 0.4, 'y': kit.rng.normal() * 0.4, 'z': float(g.height.max()) - 0.2,
                       'h': kit.rng.uniform(2.2, 3.4)} for _ in range(9)], kit.rng)
    F.smoke(v, buf, [{'x': k * 0.9, 'y': k * 0.7, 'z': 4 + k * 1.6, 'r': 1.4 + k * 0.5, 'a': 0.3,
                      'col': (0.5, 0.5, 0.5), 'glow': EMBER * (0.8 / (1 + k))} for k in range(10)], kit.rng)
    return R.finish(buf, {'saturation': 1.08, 'bloom': 1.2})


def stone(kit, g, x, y, w, d, h, turn):
    """A standing stone: a block w x d metres, h high, its top rounded, painted with the `stone` texture."""
    r = max(w, d)
    sl = np.s_[max(int(g.texel(0, y - r)[1]), 0):int(g.texel(0, y + r)[1]) + 2,
               max(int(g.texel(x - r, 0)[0]), 0):int(g.texel(x + r, 0)[0]) + 2]
    X, Y = g.X[sl], g.Y[sl]
    c, s = np.cos(turn), np.sin(turn)
    u = ((X - x) * c + (Y - y) * s) / (w / 2)
    v = (-(X - x) * s + (Y - y) * c) / (d / 2)
    e = u ** 4 + v ** 4
    inside = e < 1
    base = float(g.at(g.height, np.array([x]), np.array([y]))[0])
    top = base + h * (1 - e ** 3 * 0.15)
    g.height[sl][inside] = top[inside]
    col = kit.tex('stone').colour(X * 1.0, Y + h, g.cell)
    g.albedo[sl][inside] = col[inside]


def technology(kit):
    """Land worked by hand and skill: a hillside cut into terraces, each a field of its own crop, water led along
    their edges in channels to a mill."""
    v = kit.view(pitch=50, fov=34, dist=380, ty=10)
    g = R.Ground(v.ground_box(margin=0.1), 3072)
    hill = 60 * np.exp(-((g.X + 30) ** 2 + (g.Y - 90) ** 2 * 0.6) / 230 ** 2) + kit.noise(g, 200, 3) * 4
    step = 4.0
    lvl = np.floor(hill / step)
    frac = hill / step - lvl
    g.height[:] = (lvl + smooth(0.82, 1.0, frac)) * step
    ident = (np.sin(lvl * 12.9898 + np.floor(g.X / 90) * 78.233) * 43758.5453) % 1
    kit.paint(g, 'crop', (ident < 0.4).astype(np.float32), turn=0.2)
    kit.paint(g, 'gold', ((ident >= 0.4) & (ident < 0.7)).astype(np.float32), turn=0.2)
    kit.paint(g, 'soil', (ident >= 0.7).astype(np.float32), turn=0.3)
    rows = (np.sin((g.X * 0.6 + g.Y * 0.2) * 2.0) * 0.5 + 0.5)
    g.albedo[:] *= (0.85 + 0.3 * rows)[..., None]
    wall = smooth(0.8, 0.95, frac) * smooth(1.0, 0.97, frac)
    kit.paint(g, 'stone', wall, mix_turn=0.5)
    canal = smooth(0.06, 0.0, np.abs(frac - 0.07)) * (hill > step)
    g.height[:] -= canal * 0.6
    g.water[:] = np.where(canal > 0.3, g.height + 0.45, -1e9)
    d = course_distance(g, kit, 30, 160, x_shift=170)
    g.height[:] -= 3 * smooth(5, 1, d)
    g.water[:] = np.maximum(g.water, np.where(d < 4, g.height + 1.6, -1e9))
    g.water_tint = np.array([0.02, 0.05, 0.06], np.float32)
    for k in range(3):
        house(kit, g, 150 + 6 * k, 30 - 10 * k, 6, 9, 3, 2.7, 0.3, 'roof', (1, 1, 1), (0.45, 0.4, 0.33))
    items = kit.scatter(g, 50, smooth(40, 10, hill) * smooth(3, 8, d), 'round', (9, 13))
    buf = kit.shoot(v, g, items, haze_far=0.25)
    return R.finish(buf, {'saturation': 1.1})


def built(kit):
    """A walled town: roofs packed tight inside a stone wall with towers, streets running to its gates, a great
    hall at its middle, fields outside."""
    v = kit.view(pitch=52, fov=34, dist=520, ty=10)
    g = R.Ground(v.ground_box(margin=0.1), 3072)
    g.height[:] = eroded(g.height.shape, 800 / g.cell, kit.rng, 5, 0.5, 2.0) * 5
    edge = fields(kit, g, ['crop', 'pasture', 'soil'], (50, 80), 0.25)
    r = np.hypot(g.X / 1.15, g.Y)
    kit.paint(g, 'street', smooth(96, 90, r), mix_turn=0.6)
    tracks = paths(kit, g, 0, 0, 4, 500, 2.5, kit.rng)
    kit.paint(g, 'street', tracks * smooth(80, 100, r), mix_turn=0.6)
    village(kit, g, 0, 0, 160, 84, [('tiles', (1, 1, 1)), ('tiles', (0.8, 0.7, 0.62)), ('tiles', (1.1, 0.95, 0.85))],
            kit.rng, (7, 12))
    house(kit, g, 0, 0, 16, 30, 9, 8, 0.4, 'tiles', (0.7, 0.65, 0.6), (0.5, 0.46, 0.4))
    wall = smooth(2.2, 1.2, np.abs(r - 95))
    g.height[:] = np.maximum(g.height, wall * 9)
    kit.paint(g, 'stone', wall, mix_turn=0.5)
    for k in range(14):
        a = 2 * np.pi * k / 14
        tx, ty = np.cos(a) * 95 * 1.15, np.sin(a) * 95
        tower = smooth(4.5, 3.5, np.hypot(g.X - tx, g.Y - ty))
        g.height[:] = np.maximum(g.height, tower * 13)
        kit.paint(g, 'stone', tower, mix_turn=0.5)
    items = kit.scatter(g, 50, smooth(3, 1, edge) * smooth(110, 130, r), 'round', (8, 12))
    buf = kit.shoot(v, g, items, haze_far=0.28)
    return R.finish(buf, {'saturation': 1.08})


def beast(rng, tone):
    """A grazing animal seen from above and behind, as a small picture (rgba, linear): a dark body, a lighter back
    where the sun falls, a head lowered to one side, short legs."""
    h, w = 24, 40
    yy, xx = np.mgrid[0:h, 0:w].astype(np.float32)
    body = ((xx - 21) / 13) ** 2 + ((yy - 11) / 6.5) ** 2 < 1
    head = ((xx - 6) / 5) ** 2 + ((yy - 14) / 3.6) ** 2 < 1
    neck = (xx > 7) & (xx < 12) & (np.abs(yy - 12 - (xx - 9) * -0.2) < 3)
    legs = np.zeros_like(body)
    for lx in (12, 16, 27, 31):
        legs |= (np.abs(xx - lx) < 1.3) & (yy > 14) & (yy < 22)
    a = (body | head | neck | legs).astype(np.float32)
    a = blur(a, 0.7)
    shade = np.clip(1.25 - (yy - 4) / 16, 0.45, 1.25)[..., None]
    col = np.array(tone, np.float32) * shade
    return np.concatenate([np.broadcast_to(col, (h, w, 3)), a[..., None]], 2).astype(np.float32)


def herd(kit):
    """A herd on a plain: hundreds of grazing animals spread over a golden grassland, a trail of dust behind the
    moving front, flat-topped trees standing apart."""
    v = kit.view(pitch=50, fov=34, dist=260, ty=0)
    g = R.Ground(v.ground_box(margin=0.1), 3072)
    g.height[:] = kit.noise(g, 300, 4) * 4
    kit.paint(g, 'grass', mix_turn=0.9)
    kit.paint(g, 'dry', smooth(-0.1, 0.5, kit.noise(g, 120, 3)) * 0.8, mix_turn=1.1)
    lane = np.exp(-((g.Y - 10 - 0.3 * g.X - 25 * kit.noise(g, 150, 2)) / 40) ** 2)
    kit.paint(g, 'trampled', lane * 0.7, mix_turn=0.7)
    tones = [(0.05, 0.035, 0.025), (0.08, 0.055, 0.035), (0.035, 0.03, 0.028), (0.1, 0.08, 0.06)]
    pics = [beast(kit.rng, t) for t in tones]
    items = []
    for it in kit.scatter(g, 2400, lane * smooth(-120, 60, g.X), [0], (1.25, 1.55)):
        k = int(kit.rng.integers(len(pics)))
        it.update(img=pics[k], id=f'beast{k}', flip=bool(kit.rng.random() < 0.15))
        items.append(it)
    items += kit.scatter(g, 8, 1 - lane, 'umbrella', (9, 13))
    buf = kit.shoot(v, g, items, haze=(0.36, 0.34, 0.3), haze_far=0.3)
    dust = [{'x': float(x), 'y': float(10 + 0.3 * x + kit.rng.normal() * 20), 'z': 2.0 + kit.rng.random() * 4,
             'r': 14.0, 'a': 0.22, 'col': (0.42, 0.34, 0.24)} for x in kit.rng.uniform(60, 220, 28)]
    F.smoke(v, buf, dust, kit.rng)
    return R.finish(buf, {'saturation': 1.05, 'tint': (1.03, 1.0, 0.95)})


SCENES = {'human-population': population, 'human-war': war, 'human-culture': culture,
          'human-technology': technology, 'human-built': built, 'animal-population': herd}
