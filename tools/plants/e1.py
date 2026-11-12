import re
W = '/home/user/primordial-simulation/.claude/worktrees/agent-a257b588d76bf8a62/'
def edit(path, pairs):
    s = open(W + path).read()
    for a, b in pairs:
        assert a in s, (path, a[:70])
        s = s.replace(a, b)
    open(W + path, 'w').write(s)

edit('web/shaders/lib/draw.wgsl', [
("""fn snow_of(ice_m: f32, t: f32, melt: f32) -> f32 {
  // (tanh of a large number overflows in some shaders: past 10 it is 1 to the last bit of a 32-bit float.)
  let cover = tanh(clamp(ice_m / COVER_ICE_M, 0.0, 10.0));
  return select(cover, cover * (1.0 - ease(melt - 1.0, melt + 1.0, t)), melt > 0.0);
}

// A land plant's roots need air in the soil: liquid standing over its foot as deep as this, m, drowns them (half of the
// roots of most kinds of cover lie in the top 0.3 m, Jackson and others 1996).
const ROOTS_M = 0.3;
// The share of a plant that stands where liquid stands `depth` m deep and snow covers `snow` of the ground.
fn stands_in(depth: f32, snow: f32) -> f32 { return (1.0 - ease(0.0, ROOTS_M, depth)) * (1.0 - snow); }""",
"""fn snow_of(ice_m: f32, t: f32, melt: f32) -> f32 {
  // (tanh of a large number overflows in some shaders: past 10 it is 1 to the last bit of a 32-bit float.)
  return tanh(clamp(ice_m / COVER_ICE_M, 0.0, 10.0)) * unmelted(t, melt);
}

// The share of the frozen cover left at a point at `t` K where it melts at `melt` K (0: not known, all of it).
fn unmelted(t: f32, melt: f32) -> f32 { return select(1.0, 1.0 - ease(melt - 1.0, melt + 1.0, t), melt > 0.0); }

// Lying snow holds this many times the depth of the ice it is made of: settled seasonal snow of about 300 kg/m³ against
// ice of 917 (Paterson 1994, from memory).
const SNOW_PER_ICE = 3.0;
// The depth of the snow on ground of `ice_m` m of frozen cover on average at `t` K (`snow_of`), m.
fn snow_depth(ice_m: f32, t: f32, melt: f32) -> f32 { return SNOW_PER_ICE * ice_m * unmelted(t, melt); }

// A land plant's roots need air in the soil: liquid standing over its foot as deep as this, m, drowns them (half of the
// roots of most kinds of cover lie in the top 0.3 m, Jackson and others 1996).
const ROOTS_M = 0.3;
// The share of a plant that stands where liquid stands `depth` m deep.
fn above_water(depth: f32) -> f32 { return 1.0 - ease(0.0, ROOTS_M, depth); }
// Snow buries a plant `height` m tall as it reaches from `BURIED_FROM` of its height to all of it: a herb is gone under
// a deep snow, a tree stands in it.
const BURIED_FROM = 0.6;
// The share of a plant `height` m tall that shows over snow `snow_m` m deep.
fn above_snow(snow_m: f32, height: f32) -> f32 { return 1.0 - ease(BURIED_FROM * height, height, snow_m); }"""),
])

edit('web/shaders/lib/land.wgsl', [
("""// surface, m; how wet it is, 0..1; its temperature, K; the snow and ice on it, 0..1; and how wet its cells are on
// their mean, 0..1.
struct State { depth: f32, over: f32, wet: f32, t: f32, snow: f32, mean: f32 }""",
"""// surface, m; how wet it is, 0..1; its temperature, K; the snow and ice on it, 0..1, and the snow's depth, m; and how
// wet its cells are on their mean, 0..1.
struct State { depth: f32, over: f32, wet: f32, t: f32, snow: f32, snow_m: f32, mean: f32 }"""),
("""  p.snow = snow_of(ground.w, p.t, land.time.z);
  p.mean""", """  p.snow = snow_of(ground.w, p.t, land.time.z);
  p.snow_m = snow_depth(ground.w, p.t, land.time.z);
  p.mean"""),
("""fn activity(wet: f32, need: f32) -> f32 {
  let x = wet / need;
  return select(pow(max(x, 0.0), land.soil.z), 1.0, x >= 1.0);
}""", """fn activity(wet: f32, need: f32) -> f32 {
  let x = wet / need;
  return select(pow(max(x, 0.0), land.soil.z), 1.0, x >= 1.0);
}

// The share of a kind whose roots need `need` m of loose ground that holds where `loose` m lies (canopy.ts `rooted`, the
// same rule): none from `ROOTED_FROM` of its need down, all from its need up. Bare bed holds no liquid in pores and
// nothing to anchor a tall plant; a kind roots about `ROOT_PER_HEIGHT` of its height deep (canopy.ts).
const ROOTED_FROM = 0.25;
fn rooted(loose: f32, need: f32) -> f32 { return ease(ROOTED_FROM * need, need, loose); }"""),
])
