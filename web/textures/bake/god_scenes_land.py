"""The god pictures of the land: Nature's Create and Edit tools and the plant kinds (god.py)."""
import numpy as np

import god_render as R
from god_world import blur, erode, eroded, fbm, smooth


def slope_of(g, h=None):
    """The ground's slope (rise over run) at each texel."""
    gy, gx = np.gradient(g.height if h is None else h, g.cell)
    return np.hypot(gx, gy)


def carve(kit, g, shape, coarse=512, drops=250_000, **kw):
    """Heights for ground `g`: `shape(X, Y, noise)` (metres) laid on a coarse grid of `coarse` texels across, carved
    by rain (`erode`), then brought up to `g`'s texels. Returns the heights and how much water ran over each texel."""
    from PIL import Image
    cell = g.nx * g.cell / coarse
    nx, ny = coarse, int(g.ny * g.cell / cell) + 1
    X, Y = np.meshgrid(g.x0 + np.arange(nx) * cell, g.y0 + np.arange(ny) * cell)
    h = shape(X, Y, lambda metres, octaves=5, gain=0.5, ridged=False: fbm((ny, nx), metres / cell, kit.rng, octaves,
                                                                         gain, ridged))
    h, flow = erode(h / cell, kit.rng, drops, **kw)
    up = [np.array(Image.fromarray(np.ascontiguousarray(a, np.float32), 'F').resize((g.nx, g.ny), Image.BICUBIC))
          for a in (h * cell, flow)]
    return up[0], up[1] / (drops / (nx * ny))


def mountains(kit):
    """Rugged peaks: one massif of ridged noise, rock on its steep faces, snow on its high gentle ones, grass and
    dark forest in the valleys."""
    v = kit.view(pitch=50, fov=34, dist=9500, ty=300)
    g = R.Ground(v.ground_box(margin=0.12), 2048)
    X, Y = g.X, g.Y

    def shape(X, Y, noise):
        r = np.hypot(X / 1.4, Y - 900) / 4800
        dome = np.exp(-r * r * 1.6)
        ridge = np.clip(1 - np.abs(noise(8000, 4, 0.45)) * 2.2, 0, 1) ** 1.5
        rough = noise(2200, 6, 0.55)
        return dome * (3300 * ridge + 1000 * rough + 1300) + 250 * rough
    h, flow = carve(kit, g, shape)
    rock = kit.tex('rock')
    h += (rock.height(X, Y, g.cell) - 0.5) * 90 * smooth(200, 1200, h)
    h += eroded(h.shape, 700 / g.cell, kit.rng, 5, 0.5, 2.0) * 110 * smooth(0, 900, h)
    g.height[:] = h
    s = slope_of(g)
    kit.paint(g, 'meadow', mix_turn=1.1, sat=0.85)
    kit.paint(g, 'rock', smooth(0.35, 0.8, s) * smooth(400, 900, h) + smooth(1500, 2000, h), mix_turn=0.7,
              cliffs=True)
    forest = smooth(0.45, 0.2, s) * smooth(1300, 900, h) * smooth(-0.1, 0.25, kit.noise(g, 900, 4))
    g.albedo[:] = g.albedo * (1 - forest[..., None] * 0.7) + np.array([0.02, 0.035, 0.015]) * forest[..., None] * 0.7
    snow = smooth(1500, 1800, h + 250 * kit.noise(g, 600, 4)) * smooth(1.4, 0.7, s)
    kit.paint(g, 'snow', snow, tint=(1.15, 1.15, 1.2))
    buf = kit.shoot(v, g, [], haze=(0.3, 0.37, 0.5), haze_far=0.4)
    return R.finish(buf, {'saturation': 1.1, 'contrast': 1.06})


def meander(y, kit, amp, wave):
    """A river's course: its x for each y, a sum of three bends of falling size and random phase."""
    ph = kit.spec.get('phase', [0.3, 1.7, 2.9])
    return (amp * np.sin(y / wave + ph[0]) + amp * 0.45 * np.sin(y / wave * 2.3 + ph[1]) +
            amp * 0.2 * np.sin(y / wave * 5.1 + ph[2]))


def course_distance(g, kit, amp, wave, x_shift=0.0):
    """Each texel's distance (metres) from the river's course, across it."""
    cx = meander(g.Y, kit, amp, wave) + x_shift
    dxdy = np.gradient(cx, g.cell, axis=0)
    return np.abs(g.X - cx) / np.sqrt(1 + dxdy * dxdy)


def flowers(kit, g, role, share, colours):
    """Petals over the ground: the petal texture `role` (grey on black) tinted by a slow noise among `colours`,
    where `share` (0..1) allows."""
    t = kit.tex(role)
    petal = t.colour(g.X, g.Y, g.cell).mean(axis=2)
    pick = kit.noise(g, t.side * 4, 3)
    cols = np.array(colours, np.float32)
    idx = np.clip(((pick + 1) / 2 * len(cols)).astype(int), 0, len(cols) - 1)
    m = np.clip(petal * 3 * share, 0, 1)[..., None]
    g.albedo[:] = g.albedo * (1 - m) + cols[idx] * m


def plants(kit):
    """A lush mixed planting: a meadow in flower with ferns and herbs, shrubs and broad trees of several kinds."""
    v = kit.view(pitch=52, fov=34, dist=42)
    g = R.Ground(v.ground_box(margin=0.1), 2048)
    g.height[:] = kit.noise(g, 30, 4) * 0.8
    kit.paint(g, 'meadow', mix_turn=1.0)
    kit.paint(g, 'herbs', smooth(0.0, 0.3, kit.noise(g, 14, 3)), mix_turn=0.6)
    flowers(kit, g, 'petals', smooth(-0.2, 0.3, kit.noise(g, 10, 3)),
            [(0.75, 0.55, 0.05), (0.8, 0.8, 0.78), (0.5, 0.12, 0.45), (0.75, 0.25, 0.08)])
    one = np.ones_like(g.height)
    edge = smooth(-0.1, 0.4, kit.noise(g, 30, 3))
    items = (kit.scatter(g, 40, edge, 'round', (8, 13)) +
             kit.scatter(g, 40, edge, 'lobed', (6, 10)) +
             kit.scatter(g, 700, one, 'shrub', (1.4, 2.6), tint=((0.9, 1.05, 0.85), (1.1, 1.15, 0.9))) +
             kit.scatter(g, 600, one, 'searsia', (1.0, 1.8)) +
             kit.scatter(g, 900, one, 'othonna', (0.5, 1.0)) +
             kit.scatter(g, 800, 1 - edge, 'fern', (0.8, 1.5)))
    buf = kit.shoot(v, g, items, haze_far=0.2, gain=kit.spec.get('gain', 1.0))
    return R.finish(buf, {'saturation': 1.15})


def rivers(kit):
    """A river winding through green land: its bed carved into gentle hills, gravel bars on its inner bends, trees
    along its banks."""
    v = kit.view(pitch=50, fov=34, dist=1700)
    g = R.Ground(v.ground_box(margin=0.1), 3072)
    d = course_distance(g, kit, 260, 210)
    hills = eroded(g.height.shape, 900 / g.cell, kit.rng, 7, 0.5, 2.0)
    bed = 24
    h = 30 + hills * 40 * smooth(60, 400, d) + 6 * kit.noise(g, 120, 4)
    h = h * smooth(bed * 0.6, bed * 3.5, d) + (h.min() - 4) * (1 - smooth(bed * 0.6, bed * 3.5, d))
    g.height[:] = h - 3.5 * smooth(bed * 1.2, 0, d)
    g.water[:] = np.percentile(g.height[d < bed * 0.5], 70) + 1.4
    kit.paint(g, 'meadow', mix_turn=0.8, sat=0.9)
    kit.paint(g, 'field', smooth(0.1, 0.4, kit.noise(g, 300, 3)) * 0.7, mix_turn=1.3)
    kit.paint(g, 'gravel', smooth(bed * 1.9, bed * 0.9, d))
    g.murk[:] = 0.25
    banks = smooth(bed * 1.3, bed * 1.8, d) * smooth(bed * 6, bed * 2, d)
    woods = smooth(0.15, 0.45, kit.noise(g, 500, 4)) * smooth(bed * 2, bed * 4, d)
    items = (kit.scatter(g, 60, banks, 'round', (14, 22)) + kit.scatter(g, 40, banks, 'lobed', (12, 18)) +
             kit.scatter(g, 45, woods, 'round', (14, 22)) + kit.scatter(g, 20, woods, 'column', (16, 24)))
    buf = kit.shoot(v, g, items, haze_far=0.3)
    return R.finish(buf, {'saturation': 1.1})


def lakes(kit):
    """A lake among wooded hills: a deep clear basin, a pale shore of sand and pebbles, reeds in its shallows."""
    v = kit.view(pitch=50, fov=34, dist=1500, ty=100)
    g = R.Ground(v.ground_box(margin=0.1), 3072)
    X, Y = g.X, g.Y
    warp = kit.noise(g, 900, 3) * 260
    r = np.hypot((X + warp) / 1.35, Y - 120 + warp * 0.6) / 420
    hills = eroded(g.height.shape, 1100 / g.cell, kit.rng, 7, 0.5, 2.0) * 0.5 + 0.5
    h = 40 + 170 * hills * smooth(0.7, 2.2, r) + 25 * kit.noise(g, 200, 4)
    h = h - 70 * smooth(1.15, 0.2, r)
    g.height[:] = h
    level = 22.0
    g.water[:] = level
    depth = level - h
    kit.paint(g, 'meadow', mix_turn=0.8, sat=0.9)
    kit.paint(g, 'shore', smooth(-4.0, -0.5, -depth) * smooth(9.0, 1.0, -depth) + smooth(0, 3, depth))
    kit.paint(g, 'bed', smooth(2, 10, depth))
    land = smooth(-1.0, -6.0, depth)
    woods = land * smooth(0.0, 0.3, kit.noise(g, 700, 4)) * smooth(-14, -30, depth)
    items = (kit.scatter(g, 70, woods, 'conifer', (18, 28)) + kit.scatter(g, 50, woods, 'round', (14, 20)) +
             kit.scatter(g, 25, land * 0.5, 'round', (12, 18)) +
             kit.scatter(g, 400, smooth(0.2, 0.6, depth) * smooth(1.4, 0.8, depth), 'reed', (2.5, 3.5)))
    buf = kit.shoot(v, g, items, haze_far=0.35)
    return R.finish(buf, {'saturation': 1.1})


def meadow_ground(kit, g, relief=1.5):
    """Gently rolling grassland: the base of the Edit tools' pictures, its colour varied by a slow noise."""
    g.height[:] = kit.noise(g, 120, 4) * relief * 3 + kit.noise(g, 25, 3) * relief * 0.4
    kit.paint(g, 'meadow', mix_turn=0.9)
    kit.detail(g, 'grass', 0.8)
    g.albedo[:] *= (1 + 0.25 * kit.noise(g, 60, 3))[..., None]


def torn_earth(kit, g, slope, start=0.45, full=0.8):
    """Bare earth where the ground was pulled steep (the turf torn off it): the `earth` texture over slopes past
    `start`, darker clods in it."""
    m = smooth(start, full, slope + 0.15 * kit.noise(g, 6, 3))
    kit.paint(g, 'earth', m, mix_turn=0.8)
    return m


def raise_(kit):
    """Land pushed up into a hill: its turf torn on its steep flanks, the earth showing, rocks rolled to its foot."""
    v = kit.view(pitch=50, fov=34, dist=230, ty=10)
    g = R.Ground(v.ground_box(margin=0.1), 2560)
    meadow_ground(kit, g)
    X, Y = g.X, g.Y
    warp = kit.noise(g, 60, 3) * 9
    r = np.hypot(X + warp, (Y - 20 + warp) * 1.1) / 44
    hill = 34 * np.exp(-r ** 2.6 * 1.3) * (1 + 0.12 * kit.noise(g, 30, 4))
    g.height[:] += hill
    s = slope_of(g)
    earth = torn_earth(kit, g, s, 0.28, 0.55)
    crack = (smooth(0.06, 0.0, np.abs(r - 1.05 - 0.08 * kit.noise(g, 15, 3))) *
             smooth(0.2, 0.6, kit.noise(g, 9, 3) + 0.4))
    g.height[:] -= crack * 0.8
    g.albedo[:] *= 1 - crack[..., None] * 0.7
    top = smooth(0.35, 0.1, s) * smooth(20, 28, hill)
    foot = smooth(0.7, 1.0, r) * smooth(1.5, 1.1, r)
    items = (kit.scatter(g, 60, foot * earth, 'rock', (0.8, 2.2)) +
             kit.scatter(g, 200, earth * 0.5, 'cobble', (0.4, 0.9)) +
             kit.scatter(g, 80, top, 'round', (8, 12)) +
             kit.scatter(g, 6, smooth(1.6, 2.2, r), 'round', (9, 13)))
    buf = kit.shoot(v, g, items, haze_far=0.2)
    import god_fx as F
    dust = [{'x': float(44 * 1.1 * np.cos(a)), 'y': float(20 + 40 * np.sin(a)), 'z': 3.0, 'r': 9.0, 'a': 0.25,
             'col': (0.33, 0.27, 0.2)} for a in kit.rng.uniform(0, 2 * np.pi, 14)]
    F.smoke(v, buf, dust, kit.rng)
    return R.finish(buf, {'saturation': 1.1})


def lower(kit):
    """Land pressed down into a hollow: steep earthen walls under a torn rim of turf, a damp floor."""
    v = kit.view(pitch=52, fov=34, dist=220, ty=10)
    g = R.Ground(v.ground_box(margin=0.1), 2560)
    meadow_ground(kit, g)
    X, Y = g.X, g.Y
    warp = kit.noise(g, 60, 3) * 8
    r = np.hypot(X + warp, (Y - 15 + warp) * 1.1) / 42
    pit = 26 * (1 - smooth(0.55, 1.05, r)) * (1 + 0.1 * kit.noise(g, 25, 4))
    g.height[:] -= pit
    s = slope_of(g)
    torn_earth(kit, g, s, 0.25, 0.5)
    floor = smooth(0.6, 0.3, r)
    kit.paint(g, 'floor', floor, mix_turn=0.5)
    crack = (smooth(0.05, 0.0, np.abs(r - 1.15 - 0.08 * kit.noise(g, 12, 3))) *
             smooth(0.1, 0.5, kit.noise(g, 8, 3) + 0.4))
    g.height[:] -= crack * 0.6
    g.albedo[:] *= 1 - crack[..., None] * 0.7
    items = (kit.scatter(g, 300, smooth(0.5, 0.8, s) * 0.5, 'rock', (0.6, 1.8)) +
             kit.scatter(g, 8, smooth(1.5, 2.0, r), 'round', (9, 13)) +
             kit.scatter(g, 200, smooth(1.2, 1.5, r) * 0.3, 'shrub', (1.2, 2.0)))
    buf = kit.shoot(v, g, items, haze_far=0.2)
    return R.finish(buf, {'saturation': 1.1})


def lake_brush(kit):
    """A freshly dug basin filling with water: raw earth, a ring of wet mud, muddy water spreading over its floor,
    a stream running in."""
    v = kit.view(pitch=52, fov=34, dist=200, ty=5)
    g = R.Ground(v.ground_box(margin=0.1), 2560)
    meadow_ground(kit, g, 1.0)
    X, Y = g.X, g.Y
    warp = kit.noise(g, 50, 3) * 7
    r = np.hypot((X + warp) / 1.25, Y - 5 + warp) / 40
    basin = 9 * (1 - smooth(0.0, 1.0, r) ** 1.6) + 0.6 * kit.noise(g, 10, 3)
    g.height[:] -= basin * smooth(1.1, 0.95, r)
    stream_d = course_distance(g, kit, 6, 18, x_shift=8) * (Y > 25)
    g.height[:] -= 1.6 * smooth(2.2, 0.3, stream_d) * (Y > 25)
    dug = smooth(1.08, 0.95, r)
    kit.paint(g, 'earth', dug, mix_turn=0.7)
    level = float(np.percentile(g.height[r < 0.2], 50)) + 3.2
    g.water[:] = np.where((r < 1.0) | ((stream_d < 1.6) & (Y > 25)), level, -1e9)
    g.water[(stream_d < 1.6) & (Y > 25)] = g.height[(stream_d < 1.6) & (Y > 25)] + 0.35
    wet = smooth(2.5, 0.0, g.height - level) * dug
    g.albedo[:] *= 1 - wet[..., None] * 0.55
    g.murk[:] = 0.6
    g.water_absorb = np.array([1.2, 1.4, 1.7], np.float32)
    spoil = kit.scatter(g, 500, smooth(1.0, 1.15, r) * smooth(1.35, 1.15, r), 'cobble', (0.4, 1.0))
    items = spoil + kit.scatter(g, 8, smooth(1.6, 2.0, r), 'round', (8, 12))
    buf = kit.shoot(v, g, items, haze_far=0.2)
    return R.finish(buf, {'saturation': 1.1})


def smooth_tool(kit):
    """Rough land softened into gentle slopes: broken rocky ground on the left eased into smooth green rolls on the
    right, along a soft edge like a brush's."""
    v = kit.view(pitch=50, fov=34, dist=420)
    g = R.Ground(v.ground_box(margin=0.1), 3072)
    X, Y = g.X, g.Y
    rough = (eroded(g.height.shape, 160 / g.cell, kit.rng, 8, 0.6, 0.6) * 38 +
             np.clip(1 - np.abs(kit.noise(g, 250, 3)) * 2.5, 0, 1) ** 2 * 30 + kit.noise(g, 12, 3) * 1.5)
    soft = blur(rough, 45 / g.cell)
    m = smooth(-40, 40, X + Y * 0.3 + 25 * kit.noise(g, 120, 3))
    g.height[:] = rough * (1 - m) + soft * m
    s = slope_of(g)
    kit.paint(g, 'meadow', mix_turn=0.9)
    kit.paint(g, 'rock', (1 - m) * smooth(0.15, 0.5, s), mix_turn=0.7, cliffs=True)
    items = kit.scatter(g, 400, (1 - m) * smooth(0.3, 0.6, s), 'rock', (1.5, 4))
    buf = kit.shoot(v, g, items, haze_far=0.3)
    return R.finish(buf, {'saturation': 1.1})


def kind(kit):
    """A plant kind's card: a close look down on a patch of it (the spec's layers of ground and sprites)."""
    sp = kit.spec
    v = kit.view(pitch=sp.get('pitch', 55), fov=34, dist=sp['dist'], small=True)
    g = R.Ground(v.ground_box(margin=0.12), 1024)
    g.height[:] = kit.noise(g, sp['dist'], 3) * sp['dist'] * 0.01
    kit.paint(g, 'ground', mix_turn=0.9)
    if 'cover' in sp['textures']:
        kit.paint(g, 'cover', smooth(-0.3, 0.4, kit.noise(g, sp['dist'] * 0.4, 3)), mix_turn=1.2)
    if 'water' in sp:
        g.water[:] = g.height.max() + sp['water']
        g.murk[:] = 0.2
        g.water_absorb = np.array([2.0, 1.6, 1.8], np.float32)
        g.water_tint = np.array([0.008, 0.016, 0.01], np.float32)
    one = np.ones_like(g.height)
    items = []
    for layer in sp['sprites']:
        z = {'z': float(g.water.max())} if layer.get('afloat') else {}
        got = kit.scatter(g, layer['density'], one, layer['set'], layer['size'], layer.get('tint'))
        for it in got:
            it.update(z)
        items += got
    buf = kit.shoot(v, g, items, haze_far=0.1, gain=sp.get('gain', 1.0))
    return R.finish(buf, {'saturation': 1.12, 'vignette': 0.5})


SCENES = {'mountains': mountains, 'plants': plants, 'rivers': rivers, 'lakes': lakes, 'raise': raise_,
          'lower': lower, 'lake-brush': lake_brush, 'smooth': smooth_tool, 'kind': kind}
