"""The god pictures of Destruction's Water and Climate tools (god.py)."""
import numpy as np

import god_fx as F
import god_render as R
from god_scenes_land import carve, course_distance, slope_of
from god_world import blur, eroded, smooth


def tsunami(kit):
    """A huge wave reaching a coast: its crest breaking white along its length, its face dark and green where the
    light comes through, the sea drawn back from the beach before it."""
    v = kit.view(pitch=40, fov=34, dist=760, ty=40)
    g = R.Ground(v.ground_box(margin=0.1), 3072)
    X, Y = g.X, g.Y
    coast = -90 + 30 * kit.noise(g, 500, 3) + X * 0.06
    u = Y - coast
    g.height[:] = np.where(u < 0, 2 - u * 0.03 + kit.noise(g, 120, 3) * 3 * smooth(0, -60, u), 2 - u * 0.05)
    g.height[:] += kit.noise(g, 40, 3) * 0.4
    kit.paint(g, 'land', mix_turn=0.9)
    kit.paint(g, 'sand', smooth(-30, -5, u), mix_turn=0.7)
    kit.paint(g, 'seabed', smooth(5, 30, u))
    front = coast + 120 + 25 * kit.noise(g, 350, 3)
    w = Y - front
    crest = 55 * (1 + 0.12 * kit.noise(g, 140, 3))
    wave = crest * np.where(w < 0, np.exp(-(w / 16) ** 2), np.exp(-(w / 110) ** 2))
    sea = -1.0 - 3.0 * smooth(110, 0, w) * smooth(-120, 0, w)
    g.water[:] = np.where(u > 8, sea + wave + kit.noise(g, 30, 3) * 0.5, -1e9)
    gy, gx = np.gradient(blur(g.water, 1.0), g.cell)
    g.wave_n = np.stack([-np.clip(gx, -4, 4), -np.clip(gy, -4, 4)], 2).astype(np.float32)
    g.water_tint = np.array([0.015, 0.06, 0.065], np.float32)
    lip = smooth(-10, -1, w) * smooth(10, 0, w) * smooth(0.55, 0.9, wave / crest)
    face = smooth(-28, -8, w) * smooth(0, -6, w) * 0.4
    churn = smooth(-130, -25, w) * smooth(0.2, 0.8, kit.noise(g, 7, 3))
    streaks = smooth(0.6, 0.9, kit.noise(g, 25, 4)) * 0.15 * smooth(0, 60, w)
    g.foam[:] = np.clip(lip + face * smooth(0.2, 0.7, kit.noise(g, 5, 3)) * 0.35 + churn * 0.5 + streaks, 0, 1)
    g.glow[:] += np.array([0.03, 0.16, 0.15], np.float32) * (face * (1 - g.foam))[..., None]
    land = smooth(-12, -30, u)
    items = (kit.scatter(g, 160, land * smooth(-0.2, 0.3, kit.noise(g, 200, 3)), 'palm', (12, 20)) +
             kit.scatter(g, 60, land * smooth(-60, -120, u), 'round', (12, 18)))
    buf = kit.shoot(v, g, items, haze=(0.32, 0.37, 0.43), haze_far=0.3)
    spray = []
    for x in np.linspace(g.x0, g.x0 + g.nx * g.cell, 90):
        y = 0.0
        for _ in range(5):
            y = float(g.at(front, np.array([x]), np.array([y]))[0])
        top = float(g.at(g.water, np.array([x]), np.array([y]))[0])
        for k in range(2):
            spray.append({'x': x + kit.rng.normal() * 5, 'y': y - 4 + k * 10, 'z': top + 4 + k * 10, 'r': 16 + 10 * k,
                          'a': 0.55, 'col': (0.85, 0.88, 0.9)})
    F.smoke(v, buf, spray, kit.rng)
    return R.finish(buf, {'saturation': 1.08})


def fields(kit, g, roles, size=(70, 110), turn=0.2):
    """A patchwork of fields: the ground cut into rectangles turned by `turn`, each painted with one of `roles`
    and a slight tint of its own. Returns each texel's distance (metres) to its field's edge."""
    c, s = np.cos(turn), np.sin(turn)
    u, w = g.X * c - g.Y * s, g.X * s + g.Y * c
    rows = np.floor(w / size[1])
    shift = (np.sin(rows * 12.9898) * 43758.5453) % 1 * size[0]
    cols = np.floor((u + shift) / size[0])
    ident = (np.sin(cols * 78.233 + rows * 12.9898) * 43758.5453) % 1
    for k, role in enumerate(roles):
        m = (ident * len(roles)).astype(int) == k
        kit.paint(g, role, m.astype(np.float32), turn=turn)
    tone = 0.85 + 0.3 * ((ident * 7.13) % 1)
    g.albedo[:] *= tone[..., None]
    eu = np.minimum((u + shift) % size[0], size[0] - (u + shift) % size[0])
    ew = np.minimum(w % size[1], size[1] - w % size[1])
    return np.minimum(eu, ew)


def flood(kit):
    """A river spilling over its banks across fields: brown water spread wide, hedges and trees standing in it,
    only the highest fields still dry."""
    v = kit.view(pitch=50, fov=34, dist=650, ty=20)
    g = R.Ground(v.ground_box(margin=0.1), 3072)
    edge = fields(kit, g, ['crop', 'pasture', 'soil', 'pasture2'])
    d = course_distance(g, kit, 110, 150, x_shift=-40)
    g.height[:] = 4 + kit.noise(g, 350, 4) * 2.0 + smooth(100, 500, d) * 2.5 - 3 * smooth(30, 0, d)
    g.water[:] = 5.6
    g.murk[:] = 1.0
    g.water_absorb = np.array([2.2, 2.6, 3.2], np.float32)
    g.water_tint = np.array([0.06, 0.05, 0.03], np.float32)
    g.ripple = 0.5
    hedges = smooth(3.0, 0.5, edge)
    items = (kit.scatter(g, 200, hedges, 'round', (8, 13)) + kit.scatter(g, 60, hedges, 'column', (12, 18)) +
             kit.scatter(g, 30, smooth(60, 20, d), 'lobed', (10, 14)))
    for it in items:
        it['z'] = min(5.6 - 0.3, float(g.at(g.height, np.array([it['x']]), np.array([it['y']]))[0]))
    buf = kit.shoot(v, g, items, haze_far=0.3)
    return R.finish(buf, {'saturation': 1.05})


def water_drought(kit):
    """A dried-out lake: its floor cracked mud, rings of old shorelines down its sides, a last pool shrinking at its
    middle, dead trees on its old shore."""
    v = kit.view(pitch=50, fov=34, dist=430, ty=20)
    g = R.Ground(v.ground_box(margin=0.1), 3072)
    warp = kit.noise(g, 300, 3) * 50
    r = np.hypot((g.X + warp) / 1.4, g.Y - 20 + warp * 0.5) / 170
    g.height[:] = 14 * smooth(0.15, 1.3, r) + 8 * smooth(1.0, 2.5, r) * kit.noise(g, 400, 3) + kit.noise(g, 20, 3) * 0.3
    kit.paint(g, 'land', mix_turn=0.9)
    bed = smooth(1.15, 0.95, r)
    kit.paint(g, 'mud', bed, mix_turn=0.6)
    kit.paint(g, 'cracks', smooth(0.85, 0.45, r), mix_turn=0.4)
    kit.crack(g, 5.0, smooth(0.95, 0.6, r), 0.07, 0.06)
    rings = (np.sin(g.height * 2.6) * 0.5 + 0.5) * smooth(1.05, 0.5, r) * smooth(0.15, 0.4, r)
    g.albedo[:] *= 1 + rings[..., None] * 0.2
    g.water[:] = float(np.percentile(g.height[r < 0.12], 60))
    g.murk[:] = 0.8
    g.water_absorb = np.array([1.6, 1.8, 2.2], np.float32)
    g.water_tint = np.array([0.03, 0.035, 0.02], np.float32)
    g.albedo[:] *= 1 - smooth(1.5, 0.0, g.height - g.water)[..., None] * 0.35
    shore = smooth(0.95, 1.05, r) * smooth(1.35, 1.1, r)
    items = (kit.scatter(g, 40, shore, list(range(157, 175)), (9, 14)) +
             kit.scatter(g, 60, smooth(1.1, 1.6, r), 'dead_bush', (1.5, 3)))
    buf = kit.shoot(v, g, items, haze=(0.3, 0.28, 0.25), haze_far=0.3, exposure=0.9)
    return R.finish(buf, {'saturation': 1.0, 'tint': (1.04, 1.0, 0.94), 'contrast': 1.1})


def climate_drought(kit):
    """Parched land under a hot sun: grass dead and pale, a stream bed run dry and cracked, trees standing bare."""
    v = kit.view(pitch=50, fov=34, dist=170, ty=5)
    g = R.Ground(v.ground_box(margin=0.1), 3072)
    g.height[:] = kit.noise(g, 200, 4) * 4
    d = course_distance(g, kit, 30, 45)
    g.height[:] -= 1.5 * smooth(7, 2, d)
    kit.paint(g, 'grass', mix_turn=0.9)
    kit.paint(g, 'soil', smooth(-0.2, 0.5, kit.noise(g, 40, 3)) * 0.7, mix_turn=1.1)
    kit.paint(g, 'bed', smooth(6, 3, d), mix_turn=0.5)
    kit.crack(g, 1.4, smooth(6, 3, d), 0.08, 0.05)
    one = np.ones_like(g.height)
    items = (kit.scatter(g, 60, smooth(5, 9, d), list(range(157, 175)), (9, 14)) +
             kit.scatter(g, 300, smooth(4, 7, d), 'dead_bush', (1.0, 2.0)) +
             kit.scatter(g, 400, smooth(4, 7, d), 'tuft', (0.4, 0.7), tint=((1.6, 1.2, 0.7), (1.8, 1.5, 0.9))) +
             kit.scatter(g, 300, smooth(5, 2, d) * one, 'cobble', (0.3, 0.7)))
    buf = kit.shoot(v, g, items, haze=(0.36, 0.32, 0.26), haze_far=0.15, exposure=0.9)
    sun_glare(buf, (0.8, 0.62, 0.38))
    return R.finish(buf, {'saturation': 0.95, 'tint': (1.05, 1.0, 0.9), 'contrast': 1.12})


def heatwave(kit):
    """Heat shimmering over scorched land: the air itself wavering and orange, the ground red, baked and cracked."""
    v = kit.view(pitch=50, fov=34, dist=70, ty=0)
    g = R.Ground(v.ground_box(margin=0.1), 3072)
    g.height[:] = kit.noise(g, 80, 4) * 1.5
    kit.paint(g, 'soil', mix_turn=0.9)
    kit.crack(g, 1.6, np.ones_like(g.height), 0.07, 0.06)
    kit.paint(g, 'grass', smooth(0.1, 0.6, kit.noise(g, 20, 3)) * 0.8, mix_turn=1.2)
    one = np.ones_like(g.height)
    items = (kit.scatter(g, 40, one, list(range(157, 175)), (7, 10)) +
             kit.scatter(g, 300, one, 'dead_bush', (0.8, 1.6)) + kit.scatter(g, 300, one, 'rock', (0.3, 0.9)))
    buf = kit.shoot(v, g, items, haze=(0.45, 0.24, 0.1), haze_far=0.22, exposure=0.9)
    sun_glare(buf, (2.2, 1.0, 0.3))
    buf['rgb'][:] = F.shimmer(buf['rgb'], 9.0, kit.rng)
    buf['rgb'][:] = buf['rgb'] * np.array([1.08, 0.88, 0.68], np.float32)
    return R.finish(buf, {'saturation': 1.1, 'tint': (1.05, 0.95, 0.85), 'contrast': 1.12})


def sun_glare(buf, colour):
    """The sun's glare beyond the picture's upper left corner: a broad soft light falling off across it."""
    h, w = buf['rgb'].shape[:2]
    yy, xx = np.mgrid[0:h, 0:w].astype(np.float32)
    d = np.hypot(xx + w * 0.05, yy + h * 0.15) / w
    buf['glow'][:] += np.exp(-d * 3.2)[..., None] * np.array(colour, np.float32) * 0.5


def glacial(kit):
    """Ice and snow spreading over the land: glaciers filling the valleys of a range, streaked along their flow,
    snow over all but the steepest rock, its ragged edge creeping down into green valleys."""
    v = kit.view(pitch=50, fov=34, dist=9000, ty=300)
    g = R.Ground(v.ground_box(margin=0.12), 3072)

    def shape(X, Y, noise):
        r = np.hypot(X / 1.4, Y - 1200) / 5000
        dome = np.exp(-r * r * 1.6)
        ridge = np.clip(1 - np.abs(noise(8000, 4, 0.45)) * 2.2, 0, 1) ** 1.5
        rough = noise(2200, 6, 0.55)
        return dome * (3300 * ridge + 1000 * rough + 1300) + 250 * rough
    h, flow = carve(kit, g, shape)
    wet = blur(flow, 4)
    ice = smooth(np.percentile(wet, 70), np.percentile(wet, 88), wet) * smooth(350, 800, h)
    g.height[:] = h * (1 - ice) + (blur(h, 5) + 20) * ice
    s = slope_of(g)
    kit.paint(g, 'land', mix_turn=0.9)
    kit.paint(g, 'rock', smooth(0.45, 0.9, s) * smooth(300, 700, g.height), mix_turn=0.8, cliffs=True)
    reach = g.height + (g.Y - g.y0) * 0.12 + 200 * kit.noise(g, 700, 4)
    snow = smooth(650, 850, reach) * smooth(1.3, 0.75, s)
    kit.paint(g, 'snow', snow, mix_turn=0.9, tint=(1.1, 1.12, 1.18))
    streak = kit.noise(g, 160, 3, ridged=True)
    blue = np.array([0.5, 0.62, 0.74], np.float32) * (0.8 + 0.25 * streak)[..., None]
    g.albedo[:] = g.albedo * (1 - ice[..., None]) + blue * ice[..., None]
    moraine = smooth(0.15, 0.4, ice) * smooth(0.65, 0.4, ice)
    g.albedo[:] *= 1 - moraine[..., None] * 0.45
    woods = smooth(0.4, 0.2, s) * (1 - snow) * smooth(800, 400, g.height)
    g.albedo[:] = g.albedo * (1 - woods[..., None] * 0.6) + np.array([0.02, 0.03, 0.02]) * woods[..., None] * 0.6
    buf = kit.shoot(v, g, [], haze=(0.4, 0.48, 0.6), haze_far=0.4)
    return R.finish(buf, {'saturation': 0.95, 'tint': (0.95, 1.0, 1.06)})


def storm(kit):
    """A tornado and storm clouds over land: dark clouds rolling over the fields, the funnel hanging from them to
    a whirl of dust on the ground, rain falling in grey curtains, lightning."""
    v = kit.view(pitch=44, fov=34, dist=1300, ty=60)
    g = R.Ground(v.ground_box(margin=0.1), 3072)
    g.height[:] = eroded(g.height.shape, 900 / g.cell, kit.rng, 6, 0.5, 2.0) * 20
    edge = fields(kit, g, ['crop', 'pasture', 'soil'], (120, 180), 0.35)
    items = kit.scatter(g, 50, smooth(4, 1, edge), 'round', (12, 18))
    deck = 300.0
    F.cloud_shadow(g, deck, 0.4)
    g.shade *= 0.6
    buf = kit.shoot(v, g, items, haze=(0.17, 0.19, 0.21), haze_far=0.5, exposure=0.85)
    clouds = []
    for _ in range(160):
        y = kit.rng.uniform(100, 650)
        clouds.append({'x': kit.rng.uniform(-900, 900), 'y': y, 'z': deck + kit.rng.uniform(-30, 50) + y * 0.15,
                       'r': kit.rng.uniform(90, 180), 'a': 0.85 * smooth(100, 300, y), 'col': (0.13, 0.135, 0.145)})
    base = (-80.0, 20.0)
    funnel = []
    for k in range(110):
        t = k / 109
        funnel.append({'x': base[0] + 60 * np.sin(t * 2.0) + kit.rng.normal() * 2, 'y': base[1] + 80 * t,
                       'z': deck * t, 'r': 14 + 80 * t ** 2.6, 'a': 0.95, 'col': (0.07, 0.068, 0.066)})
    for k in range(40):
        a = kit.rng.uniform(0, 2 * np.pi)
        rr = kit.rng.uniform(15, 60)
        funnel.append({'x': base[0] + np.cos(a) * rr, 'y': base[1] + np.sin(a) * rr, 'z': 6 + kit.rng.random() * 30,
                       'r': 35, 'a': 0.5, 'col': (0.26, 0.21, 0.16)})
    F.smoke(v, buf, funnel + clouds, kit.rng)
    F.rain(buf['rgb'], kit.rng, 900, 22, 12, (0.06, 0.07, 0.08))
    bolt = lightning(kit.rng, (v.w * 0.74, v.h * 0.2), (v.w * 0.79, v.h * 0.62))
    for p0, p1 in zip(bolt[:-1], bolt[1:]):
        F.streak(buf['glow'], p0, p1, 2.5, (0.4, 0.5, 0.8), fade=False)
        F.streak(buf['glow'], p0, p1, 0.7, (4, 4.4, 5.5), fade=False)
    return R.finish(buf, {'saturation': 0.85, 'tint': (0.95, 1.0, 1.04), 'vignette': 0.6})


def lightning(rng, a, b, n=18):
    """A lightning bolt's points from a to b, each knocked aside at random, the kinks sharp."""
    pts = [a]
    x = a[0]
    for k in range(1, n):
        t = k / n
        x = x + rng.normal() * 9 + (b[0] - x) / (n - k + 1)
        pts.append((x, a[1] + (b[1] - a[1]) * t))
    pts.append(b)
    return pts


SCENES = {'tsunami': tsunami, 'flood': flood, 'water-drought': water_drought,
          'climate-drought': climate_drought, 'heatwave': heatwave, 'glacial': glacial, 'storm': storm}
