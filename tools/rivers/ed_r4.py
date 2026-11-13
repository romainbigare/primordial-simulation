def edit(p, pairs):
    t = open(p).read()
    for a, b in pairs:
        assert t.count(a) == 1, (p, a[:70], t.count(a))
        t = t.replace(a, b)
    open(p, 'w').write(t)

edit('web/shaders/lib/rock.wgsl', [
 ("""// At one place a channel's water grows wider with its discharge as its power `STATION_EXP` (at-a-station hydraulic
// geometry, Leopold and Maddock 1953: 0.26), far less than from channel to channel: at its mean flow the water fills
// most of the bed its floods cut.
const STATION_EXP = 0.26;
""", """// At one place a channel's water grows wider with its discharge far less than from channel to channel (tape/src/ground/
// bends.rs `STATION_EXP`): at its mean flow the water is `land.reach.w` of the bed its floods cut wide.
"""),
 ("""// on (the sign of `Tiled.sd` there).
struct Channel { bed: f32, water: f32, middle: f32, mean: f32, deep: f32, bend: f32, outer: f32 }
fn channel_at(g: Tiled, q: f32) -> Channel {
  if (q <= 0.0) { return Channel(0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0); }
  let flare = 1.0 + MOUTH_FLARE * (1.0 - ease(0.0, MOUTH_OVER_M, over_ponds(g, 0.0)));
  let law = banks_of(q) * flare;""", """// on (the sign of `Tiled.sd` there); and the half width the channel law gives its bed, m (`banks_of`).
struct Channel { bed: f32, water: f32, middle: f32, mean: f32, deep: f32, bend: f32, outer: f32, law: f32 }
fn channel_at(g: Tiled, q: f32) -> Channel { return channel_of(g, q, banks_of(q)); }
// (The same, of a channel whose bed the law makes `banks` m half wide.)
fn channel_of(g: Tiled, q: f32, banks: f32) -> Channel {
  if (q <= 0.0) { return Channel(0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0); }
  let flare = 1.0 + MOUTH_FLARE * (1.0 - ease(0.0, MOUTH_OVER_M, over_ponds(g, 0.0)));
  let law = banks * flare;"""),
 ("""  let water = min(law * vary * pow(land.reach.y, -STATION_EXP) * wide, law * vary);""", """  let water = min(law * vary * land.reach.w * wide, law * vary);"""),
 ("""  return Channel(bed, water, middle, land.channel.z * pow(q, land.channel.w) / flare, 1.0 / wide, bend, outer);""",
  """  return Channel(bed, water, middle, land.channel.z * pow(q, land.channel.w) / flare, 1.0 / wide, bend, outer, banks);"""),
])

edit('web/shaders/lib/land.wgsl', [
 ("""  // many times the mean flow the flood that shapes a channel's bed passes (tape/src/ground/bends.rs `FLOOD`), and how
  // far the signs of the flow have slid down the slowest water, m (web/src/land.ts `slid`).""",
  """  // many times the mean flow the flood that shapes a channel's bed passes (tape/src/ground/bends.rs `FLOOD`), how
  // far the signs of the flow have slid down the slowest water, m (web/src/land.ts `slid`), and the share of its bed's
  // width a channel's water takes at its mean flow (`FLOOD` to the power minus `STATION_EXP`)."""),
])

edit('web/shaders/lib/river.wgsl', [
 ("""// The line a channel of discharge `q` is drawn as from afar where a pixel takes `e` m: its half width, m, and how
// much of it shows over the bed itself, 0..1.
fn line_of(q: f32, e: f32) -> vec2f {
  if (q <= 0.0) { return vec2f(0.0); }
  let css = e * land.samples.w;
  let shows = land.time.w * css;
  let line = 0.5 * land.line.x * css * min(pow(q / shows, land.line.y), land.line.z) * ease(shows / LINE_FADE, shows, q);
  return vec2f(line, ease(LINE_BENDS.x * banks_of(q), LINE_BENDS.y * banks_of(q), line));
}""", """// The line a channel of discharge `q`, whose bed the law makes `banks` m half wide, is drawn as from afar where a
// pixel takes `e` m: its half width, m, and how much of it shows over the bed itself, 0..1.
fn line_of(q: f32, e: f32, banks: f32) -> vec2f {
  if (q <= 0.0) { return vec2f(0.0); }
  let css = e * land.samples.w;
  let shows = land.time.w * css;
  let line = 0.5 * land.line.x * css * min(pow(q / shows, land.line.y), land.line.z) * ease(shows / LINE_FADE, shows, q);
  return vec2f(line, ease(LINE_BENDS.x * banks, LINE_BENDS.y * banks, line));
}
// The half width of the widest line drawn where a pixel takes `e` m, m.
fn line_most(e: f32) -> f32 { return 0.5 * land.line.x * land.line.z * e * land.samples.w; }"""),
 ("""// place along the channel growing by `grows` a pixel east and south), the ground `shore_over` m over its water: the bed (rock.wgsl `banks_of`) and this month's water in it (lib/river.wgsl `reach_at`)""",
  """// place along the channel growing by `grows` a pixel east and south), drawn from afar as the line `line`
// (`line_of`): the bed (rock.wgsl `banks_of`) and this month's water in it (lib/river.wgsl `reach_at`)"""),
 ("""fn river_spots(s: Spots, shore_over: f32, g: Tiled, ch: Channel, q: f32, stone: f32, grows: vec2f) -> Spots {
  var out = s;
  let at = s.dry.at;
  let e = s.dry.e;
  out.dry.over = shore_over;
  let line = line_of(q, e);
""", """fn river_spots(s: Spots, g: Tiled, ch: Channel, q: f32, stone: f32, grows: vec2f, line: vec2f) -> Spots {
  var out = s;
  let at = s.dry.at;
  let e = s.dry.e;
"""),
 ("""open = ease(BED_PX * e, 2.0 * BED_PX * e, banks_of(q)) * (1.0 - line.y); }""", """open = ease(BED_PX * e, 2.0 * BED_PX * e, ch.law) * (1.0 - line.y); }"""),
])

edit('web/shaders/land.wgsl', [
 ("""  let near = g.d < CHANNEL_NEAR * banks_of(q) + 2.0 * e || abs(g.calm) < line_of(q, e).x + 2.0 * e;
  if (near) {
    next = next_along(reach, at);
    q = mix(q, discharge_of(next.s), next.share);
  }
  let ch = channel_at(g, q);""", """  var banks = banks_of(q);
  let near = g.d < CHANNEL_NEAR * banks + 2.0 * e || abs(g.calm) < line_most(e) + 2.0 * e;
  if (near) {
    next = next_along(reach, at);
    q = mix(q, discharge_of(next.s), next.share);
    banks = banks_of(q);
  }
  let ch = channel_of(g, q, banks);"""),
 ("""  var moved = 0.0;
  var cobble = COBBLE_M;
  if (near && ch.bed > 0.0) {
    moved = mix(stone_of(reach), stone_of(next.s), next.share);
    cobble = max(STONE_SEEN * stone_of(reach), COBBLE_M);
  }
  let deepens = vec2f(dpdx(loose), dpdy(loose));
  p = stony(p, g, ch, q, moved, cobble);
  p.below = mix(BELOW_LEAST, 1.0, 0.5 - 0.5 * dot(deepens, g.slope) / max(length(deepens) * length(g.slope), 1e-9));
  p.snow_m = st.snow_m;""", """  // (No bed takes `BED_PX` pixels and no stone shows in a program made for draws from far: neither is read there.)
  var moved = 0.0;
  var cobble = COBBLE_M;
  if (near && ch.bed > 0.0 && BED_PX * least_e() < BED_MOST_M) {
    moved = mix(stone_of(reach), stone_of(next.s), next.share);
    cobble = max(STONE_SEEN * stone_of(reach), COBBLE_M);
  }
  let deepens = vec2f(dpdx(loose), dpdy(loose));
  if (!unseen(STONE_MOST_M / STONE_PX)) {
    p = stony(p, g, ch, q, moved, cobble);
    p.below = mix(BELOW_LEAST, 1.0, 0.5 - 0.5 * dot(deepens, g.slope) / max(length(deepens) * length(g.slope), 1e-9));
  }
  p.snow_m = st.snow_m;"""),
 ("""  if (p.depth > 0.0 || ch.water <= 0.0) { return out; }
  return river_spots(out, shore_over, g, ch, q, moved, grows);""", """  var out = Spots(p, p, 0.0, 0.0, pool, st.snow, vec3f(0.0), 0.0, vec3f(0.0), vec2f(0.0), vec2f(0.0));
  if (p.depth > 0.0 || ch.water <= 0.0) { return out; }
  out.dry.over = shore_over;
  if (!near) { return out; }
  return river_spots(out, g, ch, q, moved, grows, line_of(q, e, banks));"""),
 ("""  let out = Spots(p, p, 0.0, 0.0, pool, st.snow, vec3f(0.0), 0.0, vec3f(0.0), vec2f(0.0), vec2f(0.0));
""", ""),
 ("""  p.worn = ease(log(WORN_Q.x), log(WORN_Q.y), log(max(q, 1e-9))) * (1.0 - STEEP_SHARP * steep_reach);""",
  """  if (p.stony > 0.0) { p.worn = ease(log(WORN_Q.x), log(WORN_Q.y), log(max(q, 1e-9))) * (1.0 - STEEP_SHARP * steep_reach); }"""),
])

edit('tape/src/ground/bends.rs', [
 ("""pub const FLOOD: f64 = 5.0;""", """pub const FLOOD: f64 = 5.0;
/// At one place a channel's water grows wider with its discharge as its power this (at-a-station hydraulic geometry,
/// Leopold and Maddock 1953), far less than from channel to channel: at its mean flow the water takes `FLOOD` to the
/// power minus this of the width of the bed its floods cut.
pub const STATION_EXP: f64 = 0.26;"""),
])
edit('tape/src/ground/mod.rs', [("pub use bends::FLOOD;", "pub use bends::{FLOOD, STATION_EXP};")])
edit('tape/src/lib.rs', [("PONDS_EXACT, SHOW_M,", "PONDS_EXACT, SHOW_M, STATION_EXP,")])
edit('wasm/src/lib.rs', [
 (""""flood": tape::FLOOD},""", """"flood": tape::FLOOD, "station": tape::STATION_EXP},"""),
 ("""/// bed passes (`tape::FLOOD`).""", """/// bed passes (`tape::FLOOD`) with the power a channel's water widens by at one place (`tape::STATION_EXP`)."""),
])
edit('web/src/land.ts', [
 ("""line_law: [number, number, number]; flood: number }""", """line_law: [number, number, number]; flood: number; station: number }"""),
 (""".vec4(tm.along_wrap, l.flood, this.slid(performance.now()))""", """.vec4(tm.along_wrap, l.flood, this.slid(performance.now()), l.flood ** -l.station)"""),
])
