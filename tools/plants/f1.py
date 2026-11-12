W = '/home/user/primordial-simulation/.claude/worktrees/agent-a257b588d76bf8a62/'
def edit(path, pairs):
    s = open(W + path).read()
    for a, b in pairs:
        assert a in s, (path, a[:80])
        s = s.replace(a, b)
    open(W + path, 'w').write(s)

edit('web/shaders/land.wgsl', [
("""  p.canopy = field_at(at, STANDS, 0.0).a;
  p.z = g.h;""", """  p.z = g.h;"""),
("""  p.loose = loose_at(at);
  p.cover = cover_field(at, e, Beneath(soaked(p), p.canopy, p.loose, st.snow, st.snow_m));""",
"""  p.loose = loose_at(at);
  // The height over the nearest water, standing or running: a channel's banks rise from its edge.
  let banks = select(p.over, min(p.over, max(g.d - bed.half, 0.0) * max(length(g.slope), bed.mean / max(bed.half, 1e-6))), bed.half > 0.0);
  let cf = cover_field(at, e, Beneath(soaked(p), banks, p.loose, st.snow, st.snow_m));
  p.cover = cf.cover;
  p.canopy = cf.canopy;"""),
("""  out.dry.over = min(p.over, max(g.d - bed.half, 0.0) * max(length(g.slope), bed.mean / bed.half));""",
 """  out.dry.over = banks;"""),
])

edit('web/shaders/lib/cover_field.wgsl', [
("""// What lies under and over the cover at a point (land.wgsl `land_spots`): how soaked its ground is, 0..1; the share the
// trees' crowns close over it; the depth of its loose ground, m; the share snow covers and the snow's depth, m.
struct Beneath { soaked: f32, canopy: f32, loose: f32, snow: f32, snow_m: f32 }

// Of each lattice and the sward (red, green, blue, alpha), the share of its plants that stand at a point over `b`:
// where their roots hold (land.wgsl `rooted`, over their square's mean, so the square keeps its cover) and out of the
// snow (draw.wgsl `above_snow`).
fn held_at(at: vec2f, b: Beneath) -> vec4f {
  let tall = field_at(at, TALL_LAYER, 0.0);
  let mean = field_at(at, ROOTED_LAYER, 0.0);
  let need = max(tall * cover_look.look.z, vec4f(cover_look.look.w));
  var out: vec4f;
  for (var c = 0; c < 4; c++) { out[c] = rooted(b.loose, need[c]) / max(mean[c], 1e-3) * above_snow(b.snow_m, tall[c]); }
  return out;
}""", """// What lies under and over the cover at a point (land.wgsl `land_spots`): how soaked its ground is, 0..1; its height
// over the nearest water's level, m; the depth of its loose ground, m; the share snow covers and the snow's depth, m.
struct Beneath { soaked: f32, over: f32, loose: f32, snow: f32, snow_m: f32 }

// The cover's field at a point, premultiplied, and the share the trees' crowns close over the ground there.
struct CoverAt { cover: vec4f, canopy: f32 }

// A square whose kinds' roots hold everywhere has its mean rooted share this near 1: no point of it limits them.
const ROOTED_ALL = 0.999;

// Of each lattice and the sward (red, green, blue, alpha), the share of its plants that stand at a point over `b`:
// where their roots hold (land.wgsl `rooted`, over their square's mean, so the square keeps its cover) and out of the
// snow (draw.wgsl `above_snow`). Each layer is read only where it may limit them.
fn held_at(at: vec2f, b: Beneath) -> vec4f {
  let mean = field_at(at, ROOTED_LAYER, 0.0);
  let roots = any(mean < vec4f(ROOTED_ALL));
  if (!roots && b.snow_m <= 0.0) { return vec4f(1.0); }
  let tall = field_at(at, TALL_LAYER, 0.0);
  let need = max(tall * cover_look.look.z, vec4f(cover_look.look.w));
  var out = vec4f(1.0);
  for (var c = 0; c < 4; c++) {
    if (roots) { out[c] = rooted(b.loose, need[c]) / max(mean[c], 1e-3); }
    out[c] *= above_snow(b.snow_m, tall[c]);
  }
  return out;
}"""),
("""const TUFT_TONE = 0.15; const SWATH = 0.18; const SWATH_M = 3.0; const BLADES = 0.3; const BLADE_W = 0.05; const BLADE_L = 0.25;
// Reeds: herbs on ground soaked from `REED_WET` to fully stand dense (`REED_FILL` of the ground), their colour times
// `REED_TINT`, in streaks of blades `REED_W` wide, `REED_L` long, `REED_TONE` deep.
const REED_WET = 0.85; const REED_FILL = 0.9; const REED_TINT = vec3f(0.8, 0.85, 0.55); const REED_W = 0.04; const REED_L = 0.6; const REED_TONE = 0.4;""",
"""const TUFT_TONE = 0.15; const SWATH = 0.18; const SWATH_M = 3.0; const BLADES = 0.3; const BLADE_W = 0.05; const BLADE_L = 0.25;
// Swathes and blades are drawn only where a pixel takes less than `SWATH_E` m: past it they would read only as noise.
const SWATH_E = 1.0;
// Reeds: herbs on soaked ground (from `REED_WET`) and on the shore of standing or running water, up to `REED_OVER_M` m
// over its level, stand dense (`REED_FILL` of the ground), their colour times `REED_TINT`, in streaks of tall blades
// `REED_W` wide, `REED_L` long, `REED_TONE` deep.
const REED_WET = 0.85; const REED_OVER_M = 0.6; const REED_FILL = 0.95; const REED_TINT = vec3f(0.75, 0.8, 0.45);
const REED_W = 0.07; const REED_L = 0.9; const REED_TONE = 0.5;"""),
("const FLOWERS_GONE_M = 5.0; const FLOWER_M = 0.8; const DOT_R = 0.08; const DOT_GAP = 0.36; const DOT_PRESENT = 0.7; const DOT_HEART = 0.25;",
 "const FLOWERS_GONE_M = 5.0; const FLOWER_M = 1.2; const DOT_R = 0.1; const DOT_GAP = 0.38; const DOT_PRESENT = 0.8; const DOT_HEART = 0.25;"),
("""// The stands' draw at `at` over patches of `m` m, even over 0..1.
fn stand_draw(at: vec2f, m: f32) -> f32 { return smoothstep(-EVEN_TWO, EVEN_TWO, noise_two(at / m, 70)); }""",
"""// The stands' draw at `at` over patches of `m` m, even over 0..1.
fn stand_draw(at: vec2f, m: f32) -> f32 { return smoothstep(-EVEN_ONE, EVEN_ONE, noise_turned(at / m, 70)); }"""),
("// and each kind keeps its share (draw.wgsl `noise_two`, made even over 0..1).", "// and each kind keeps its share (draw.wgsl `noise_turned`, made even over 0..1)."),
("""  let soaked = b.soaked;
  var c = in_stands""", """  var c = in_stands"""),
("""  let swaths = shows_at(SWATH_M, e);
  if (swaths > 0.0) { tone += SWATH * swaths * noise_two(at / SWATH_M, 73); }
  let blades = shows_at(BLADE_W, e);
  if (blades > 0.0) { tone += BLADES * blades * streaks(at, BLADE_W, BLADE_L, PI * noise(at / SWARD_M, 74).x, 75); }
  let reeds = ease(REED_WET, 1.0, soaked);""", """  let swaths = shows_at(SWATH_M, e) * shows_at(SWATH_E / 0.75, e);
  if (swaths > 0.0) { tone += SWATH * swaths * noise_two(at / SWATH_M, 73); }
  let blades = shows_at(BLADE_W, e);
  if (blades > 0.0) { tone += BLADES * blades * streaks(at, BLADE_W, BLADE_L, PI * noise(at / SWARD_M, 74).x, 75); }
  let reeds = max(ease(REED_WET, 1.0, b.soaked), ease(REED_OVER_M, 0.0, b.over));"""),
("""  c *= max(tone, 0.0) * (1.0 - SWARD_SHADE * min(b.canopy, 1.0));""", """  c *= max(tone, 0.0) * (1.0 - SWARD_SHADE * min(canopy, 1.0));"""),
("""fn sward(at: vec2f, e: f32, b: Beneath, thick: f32, hold: f32) -> vec4f {""", """fn sward(at: vec2f, e: f32, b: Beneath, canopy: f32, thick: f32, hold: f32) -> vec4f {"""),
("""// (`SWARD_SEEN`) with its tufts, swathes and blades where each shows, in the shade of the trees' crowns, reeds on
// soaked ground and flowers over it.""", """// (`SWARD_SEEN`) with its tufts, swathes and blades where each shows, in the shade of `canopy` of the trees' crowns,
// reeds on soaked ground and at the water's edge, and flowers over it."""),
("""// there (`held_at`), with its grain (`grain`); the trees' in their stands, their canopy seen as closer (`CANOPY_SEEN`)
// and snowy as the ground under it.
fn cover_field(at: vec2f, e: f32, b: Beneath) -> vec4f {
  // The patches as far as they show: where they take less than a pixel the field is their mean.
  let thick = 1.0 + (patchy(at) - 1.0) * shows_at(PATCH_M, e);
  let hold = held_at(at, b);
  var dst = sward(at, e, b, thick, hold.w);
  for (var l = 0; l < COVER_LATTICES; l++) {
    let unshown = 1.0 - cover_look.shown[l];
    if (unshown <= 0.0) { continue; }
    let f = field_smooth(at, l);
    if (f.a <= 0.0) { continue; }""", """// there (`held_at`), with its grain (`grain`); the trees' in their stands, their canopy seen as closer (`CANOPY_SEEN`)
// and snowy as the ground under it. A lattice no square holds (its side 0, textures.ts) is not read.
fn cover_field(at: vec2f, e: f32, b: Beneath) -> CoverAt {
  // The patches as far as they show: where they take less than a pixel the field is their mean (and no noise is read).
  let patches = shows_at(PATCH_M, e);
  let thick = select(1.0, 1.0 + (patchy(at) - 1.0) * patches, patches > 0.0);
  let hold = held_at(at, b);
  // The trees' field, read once: their crowns' share, and their layer below.
  let trees = select(vec4f(0.0), field_smooth(at, STANDS), cover_look.sides[STANDS] > 0.0);
  var dst = sward(at, e, b, trees.a, thick, hold.w);
  for (var l = 0; l < COVER_LATTICES; l++) {
    let unshown = 1.0 - cover_look.shown[l];
    if (unshown <= 0.0 || cover_look.sides[l] <= 0.0) { continue; }
    var f = trees;
    if (l != STANDS) { f = field_smooth(at, l); }
    if (f.a <= 0.0) { continue; }"""),
("""      c = in_stands(at, e, f, STANDS_FIRST, STAND_M) * (1.0 + CANOPY_TONE * noise_turned(at / CANOPY_M, 79) * shows_at(CANOPY_M, e));""",
 """      c = in_stands(at, e, f, STANDS_FIRST, STAND_M);
      let groups = shows_at(CANOPY_M, e);
      if (groups > 0.0) { c *= 1.0 + CANOPY_TONE * noise_turned(at / CANOPY_M, 79) * groups; }"""),
("""    dst = vec4f(c * g.y * a, a) + dst * (1.0 - a);
  }
  return dst;
}""", """    dst = vec4f(c * g.y * a, a) + dst * (1.0 - a);
  }
  return CoverAt(dst, trees.a);
}"""),
])
