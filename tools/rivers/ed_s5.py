def edit(p, pairs):
    t = open(p).read()
    for a, b in pairs:
        assert t.count(a) == 1, (p, a[:70], t.count(a))
        t = t.replace(a, b)
    open(p, 'w').write(t)
edit('web/shaders/lib/rock.wgsl', [
 ("""// Value noise along a channel's line at `s` m along it, in cells `cell` m long, of stream `k`: -1..1, the same again
// after the length the place along is kept modulo (a whole number of cells).
fn noise_along(s: f32, cell: f32, k: i32) -> f32 {
  let n = i32(round(land.reach.x / cell));
  let p = s / cell;
  let i = i32(floor(p));
  let f = p - floor(p);
  let v = vec2f(f32(lowbias(vec2i(((i % n) + n) % n, 0), k)), f32(lowbias(vec2i((((i + 1) % n) + n) % n, 0), k))) * (2.0 / 4294967296.0) - 1.0;
  return mix(v.x, v.y, f * f * (3.0 - 2.0 * f));
}
""", """// Value noise over a channel's reach, of stream `k`, -1..1: at `along` m along its line, in cells `cell` m long (a whole
// number of them in the length the place along is kept modulo, after which it comes again), and `across` cells
// across it.
fn noise_reach(along: f32, cell: f32, across: f32, k: i32) -> f32 {
  let n = i32(round(land.reach.x / cell));
  let p = vec2f(along / cell, across);
  let i = vec2i(floor(p));
  let f = p - floor(p);
  let u = f * f * (3.0 - 2.0 * f);
  let x = vec2i(((i.x % n) + n) % n, (((i.x + 1) % n) + n) % n);
  let v = vec4f(f32(lowbias(vec2i(x.x, i.y), k)), f32(lowbias(vec2i(x.y, i.y), k)), f32(lowbias(vec2i(x.x, i.y + 1), k)), f32(lowbias(vec2i(x.y, i.y + 1), k))) * (2.0 / 4294967296.0) - 1.0;
  return mix(mix(v.x, v.y, u.x), mix(v.z, v.w, u.x), u.y);
}
"""),
 ("""  let vary = 1.0 + BED_VARY * (noise_along(g.along, BED_VARY_M.x, 68) + 0.5 * noise_along(g.along, BED_VARY_M.y, 69)) / 1.5;""",
  """  let vary = 1.0 + BED_VARY * (noise_reach(g.along, BED_VARY_M.x, 0.0, 68) + 0.5 * noise_reach(g.along, BED_VARY_M.y, 0.0, 69)) / 1.5;"""),
])
edit('web/shaders/lib/river.wgsl', [
 ("""// channel is kept modulo (rock.wgsl `noise_along`).""", """// channel is kept modulo (rock.wgsl `noise_reach`)."""),
 ("""    let row = floor(sd / STREAK_W_M);
    let lines = ease(0.3, 0.9, abs(noise(vec2f(sd / STREAK_W_M, 0.5), 73).x));
    let dashes = ease(0.0, 0.5, noise_along(along + 3.7 * row, STREAK_M, 74 + i32(row) % 7));
    streak = STREAK * shown * lines * dashes * (1.0 - across * across);""", """    let dashes = ease(0.35, 0.75, noise_reach(along, STREAK_M, sd / STREAK_W_M, 74));
    streak = STREAK * shown * dashes * (1.0 - across * across);"""),
 ("""    let row = floor(sd / (0.35 * WHITE_M));
    let w = noise_along(along + 1.9 * row, WHITE_M, 75 + i32(row) % 5) + 0.5 * noise_along(along + 0.7 * row, 0.5 * WHITE_M, 78);""",
  """    let w = noise_reach(along, WHITE_M, sd / (0.35 * WHITE_M), 75) + 0.5 * noise_reach(along, 0.5 * WHITE_M, sd / (0.15 * WHITE_M), 78);"""),
])
edit('web/shaders/land.wgsl', [
 ("""  let resolved = ease(e, 2.0 * e, ch.water);
  let x = r.across;
  out.wet.depth = ch.deep * mix(ch.mean, BED_MIDDLE * ch.mean * (1.0 - x * x), resolved);""", """  // Where the bed is drawn apart from the water and the water takes a few pixels, its depth shows: its pools and
  // riffles and its bed's shape across it; else its mean depth.
  let resolved = ease(e, 2.0 * e, ch.water) * open;
  let x = r.across;
  out.wet.depth = mix(ch.mean, ch.deep * BED_MIDDLE * ch.mean * (1.0 - x * x), resolved);"""),
])
