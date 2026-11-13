def edit(p, pairs):
    t = open(p).read()
    for a, b in pairs:
        assert t.count(a) == 1, (p, a[:70], t.count(a))
        t = t.replace(a, b)
    open(p, 'w').write(t)

edit('web/shaders/lib/stone.wgsl', [
 ("""// What makes the stones of a place: the share of the cells that hold one, the largest's size, m, how worn they are
// (0 angular .. 1 rounded), how deep their feet are sunk (0..1 of their height), and the snow on their ground, 0..1.
struct Stony { density: f32, size: f32, worn: f32, sunk: f32, snow: f32 }""", """// What makes the stones of a place: the share of the cells that hold one; the largest's size, m, which sets where
// they lie (so it changes only in steps from place to place: a size that changed smoothly would move every stone);
// what each stone is of that size there, 0..1 (this may change smoothly: finer up a bar); how worn they are (0 angular
// .. 1 rounded), how deep their feet are sunk (0..1 of their height), and the snow on their ground, 0..1.
struct Stony { density: f32, size: f32, small: f32, worn: f32, sunk: f32, snow: f32 }"""),
 ("""    let stands = (1.0 - s.snow * (1.0 - ease(SNOW_BURIES_M, 2.0 * SNOW_BURIES_M, size))) * ease(least, 1.5 * least, size);""",
  """    var stands = 1.0 - s.snow * (1.0 - ease(SNOW_BURIES_M, 2.0 * SNOW_BURIES_M, size));
    if (least > 0.0) { stands *= ease(least, 1.5 * least, s.small * size); }"""),
 ("""    var near = 0.0;
    if (!unseen(STONE_MOST_M / STONE_PX)) { near = ease(STONE_PX * e, 2.0 * STONE_PX * e, size); }
    let mean = 2.0 * density * stands * PI * 0.25 * 0.7 / (STONE_GAP * STONE_GAP) * (1.0 - near);""", """    var near = 0.0;
    if (!unseen(STONE_MOST_M / STONE_PX)) { near = ease(STONE_PX * e, 2.0 * STONE_PX * e, s.small * size); }
    let mean = 2.0 * density * stands * PI * 0.25 * 0.7 * s.small * s.small / (STONE_GAP * STONE_GAP) * (1.0 - near);"""),
 ("""        let p = placed(at, size, density, o, l, k);
        if (!p.held) { continue; }
        let v = p.v / p.r;
        let w = p.x * 2654435761u;
        let st = stone_at(v, e / p.r, w, s.worn, s.sunk);""", """        let p = placed(at, size, density, o, l, k);
        if (!p.held) { continue; }
        let r = s.small * p.r;
        let v = p.v / r;
        let w = p.x * 2654435761u;
        let st = stone_at(v, e / r, w, s.worn, s.sunk);"""),
 ("""        let bands = 1.0 + STONE_BANDS * sin(dot(v, dir) * p.r / STONE_BAND_M * 2.0 * PI + f32((w >> 16u) & 255u));""",
  """        let bands = 1.0 + STONE_BANDS * sin(dot(v, dir) * r / STONE_BAND_M * 2.0 * PI + f32((w >> 16u) & 255u));"""),
 ("""    let stands = ease(least, 1.5 * least, size) * ease(STONE_PX * e, 2.0 * STONE_PX * e, size);
    if (stands > 0.0) {""", """    let stands = ease(least, 1.5 * least, s.small * size) * ease(STONE_PX * e, 2.0 * STONE_PX * e, s.small * size);
    if (stands > 0.0) {"""),
 ("""        let v = p.v + back;
        let along = dot(v, down) / p.r;
        let across = abs(dot(v, vec2f(-down.y, down.x))) / p.r;""", """        let r = s.small * p.r;
        let v = p.v + back;
        let along = dot(v, down) / r;
        let across = abs(dot(v, vec2f(-down.y, down.x))) / r;"""),
 ("""        let rim = (1.0 - smoothstep(1.0, 1.35, length(v) / p.r)) * smoothstep(-0.2, -0.9, along);""", """        let rim = (1.0 - smoothstep(1.0, 1.35, length(v) / r)) * smoothstep(-0.2, -0.9, along);"""),
 ("""// in a flow that runs `down` (a unit vector), 0..1: each stone's the same stone `stones` draws, found from a point up
// the flow.
fn wake(at: vec2f, e: f32, s: Stony, least: f32, down: vec2f, k: i32) -> f32 {
  var white = 0.0;""", """// in a flow that runs `down` (a unit vector), 0..1: each stone's the same stone `stones` draws, found from a point up
// the flow.
fn wake(at: vec2f, e: f32, s: Stony, least: f32, down: vec2f, k: i32) -> f32 {
  var white = 0.0;
  if (least <= 0.0) { return white; }"""),
])
edit('web/shaders/lib/colour.wgsl', [
 ("""  // ground, m; and of its channel there: the largest stone it moves, m, the share of the places that hold one of its
  // stones (0: none), and how round it has worn them, 0..1.
  bed: vec3f, rock: f32, loose: f32, stone: f32, stony: f32, worn: f32,""", """  // ground, m; and of its channel's stones there (stone.wgsl `Stony`): the largest's size as drawn, m, what each is
  // of that size, the share of the places that hold one (0: none), and how round the channel has worn them, 0..1.
  bed: vec3f, rock: f32, loose: f32, stone: f32, small: f32, stony: f32, worn: f32,"""),
 ("""      var s = Stony(scree * (1.0 - plants), SCREE_STONE_M, 0.0, sunk, snow);
      if (k == 1) { s = Stony(p.stony * (1.0 - p.sand), max(STONE_SEEN * p.stone, COBBLE_M), p.worn, BED_SUNK, snow); }""",
  """      var s = Stony(scree * (1.0 - plants), SCREE_STONE_M, 1.0, 0.0, sunk, snow);
      if (k == 1) { s = Stony(p.stony * (1.0 - p.sand), p.stone, p.small, p.worn, BED_SUNK, snow); }"""),
])
edit('web/shaders/lib/earth.wgsl', [
 ("""  s.stone = 0.0;
  s.stony = 0.0;""", """  s.stone = 0.0;
  s.small = 1.0;
  s.stony = 0.0;"""),
])
edit('web/shaders/land.wgsl', [
 ("""  p.stone = select(0.0, mix(stone_of(reach), stone_of(next.s), next.share), ch.bed > 0.0);
  let beside = ease(STEEP_STONE.x, STEEP_STONE.y, p.stone) * (1.0 - ease(ch.bed, STONES_OUT * ch.bed, g.d));
  p.stony = IN_CHANNEL * ease(0.0, BAR_SAND_STONE, p.stone) * select(beside, 1.0, g.d < ch.bed);""", """  // (The largest drawn takes its size from the point's own sample, in a step where two samples meet: the size sets
  // where the stones lie. What each is of it follows the size the channel moves there, which grows evenly along it.)
  let moved = select(0.0, mix(stone_of(reach), stone_of(next.s), next.share), ch.bed > 0.0);
  p.stone = max(STONE_SEEN * stone_of(reach), COBBLE_M);
  p.small = clamp(max(STONE_SEEN * moved, COBBLE_M) / p.stone, STONE_SMALL.x, STONE_SMALL.y);
  let beside = ease(STEEP_STONE.x, STEEP_STONE.y, moved) * (1.0 - ease(ch.bed, STONES_OUT * ch.bed, g.d));
  p.stony = IN_CHANNEL * ease(0.0, BAR_SAND_STONE, moved) * select(beside, 1.0, g.d < ch.bed);"""),
 ("""  return river_spots(out, shore_over, g, ch, q, p.stone, grows);""", """  return river_spots(out, shore_over, g, ch, q, moved, grows);"""),
 ("""  out.dry.stone *= 1.0 - BAR_FINES * r.up;""", """  out.dry.small *= 1.0 - BAR_FINES * r.up;"""),
 ("""// they are finer: `BAR_FINES` smaller at its top.
const IN_CHANNEL = 0.6; const STONES_OUT = 1.8; const WORN_Q = vec2f(0.005, 0.3); const BAR_FINES = 0.5;""", """// they are finer: `BAR_FINES` smaller at its top. Between two samples a stone is drawn from `STONE_SMALL.x` to
// `STONE_SMALL.y` of its sample's size.
const IN_CHANNEL = 0.6; const STONES_OUT = 1.8; const WORN_Q = vec2f(0.005, 0.3); const BAR_FINES = 0.5;
const STONE_SMALL = vec2f(0.3, 1.25);"""),
 ("""    let s = Stony(p.stony, max(STONE_SEEN * p.stone, COBBLE_M), p.worn, BED_SUNK, 0.0);""", """    let s = Stony(p.stony, p.stone, p.small, p.worn, BED_SUNK, 0.0);"""),
])
