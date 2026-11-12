W = '/home/user/primordial-simulation/.claude/worktrees/agent-a257b588d76bf8a62/'
def edit(path, pairs):
    s = open(W + path).read()
    for a, b in pairs:
        assert a in s, (path, a[:70])
        s = s.replace(a, b)
    open(W + path, 'w').write(s)

edit('web/shaders/lib/cover_field.wgsl', [
("""  // 1 where the cover is drawn (0: none); how strongly its marks cover the map.
  look: vec4f,""", """  // 1 where the cover is drawn (0: none); how strongly its marks cover the map; the loose ground a kind's roots need per
  // m of its height, and the least any kind needs, m (roots.ts).
  look: vec4f,"""),
("""const SWARD_LAYER = 3; const FLOWER_LAYER = 4; const SWARD_FIRST = 5; const STANDS_FIRST = 6;""",
 """const SWARD_LAYER = 3; const FLOWER_LAYER = 4; const SWARD_FIRST = 5; const STANDS_FIRST = 6;
// Of each lattice and the sward (red, green, blue, alpha): its plants' mean height, m, and mean rooted share in the square.
const TALL_LAYER = 7; const ROOTED_LAYER = 8;"""),
("""const CANOPY_SEEN = 2.0; const CANOPY_TONE = 0.2; const CANOPY_M = 35.0;""", """const CANOPY_SEEN = 2.0; const CANOPY_TONE = 0.2; const CANOPY_M = 35.0;
// Snow lies on the trees' crowns as on the ground under them, up to this share of their colour seen from afar (the
// evergreen ones hold it; the bare ones are thin and dark).
const CANOPY_SNOW = 0.35;

// What lies under and over the cover at a point (land.wgsl `land_spots`): how soaked its ground is, 0..1; the share the
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
}"""),
("""// The herbs' sward at `at`, seen at `e` m a pixel, on ground soaked to `soaked`, premultiplied: their colour in stands
// of their kinds, the share they cover seen from above (`SWARD_SEEN`) with its tufts, swathes and blades where each
// shows, in the shade of `canopy` of the trees' crowns (their field's share), reeds on soaked ground and flowers over
// it; thicker and thinner by `thick` (`patchy`).
fn sward(at: vec2f, e: f32, soaked: f32, thick: f32, canopy: f32) -> vec4f {
  let f = field_smooth(at, SWARD_LAYER);
  if (f.a <= 0.0) { return vec4f(0.0); }
  var c = in_stands(at, e, f, SWARD_FIRST, SWARD_STAND_M);
  var s = min(f.a * thick, 1.0);""", """// The herbs' sward at `at`, seen at `e` m a pixel, over `b`, premultiplied: their colour in stands of their kinds, the
// share they cover (thicker and thinner by `thick`, `patchy`, as far as they hold there, `hold`) seen from above
// (`SWARD_SEEN`) with its tufts, swathes and blades where each shows, in the shade of the trees' crowns, reeds on
// soaked ground and flowers over it.
fn sward(at: vec2f, e: f32, b: Beneath, thick: f32, hold: f32) -> vec4f {
  let f = field_smooth(at, SWARD_LAYER);
  if (f.a <= 0.0 || hold <= 0.0) { return vec4f(0.0); }
  let soaked = b.soaked;
  var c = in_stands(at, e, f, SWARD_FIRST, SWARD_STAND_M);
  var s = min(f.a * thick * hold, 1.0);"""),
("""  c *= max(tone, 0.0) * (1.0 - SWARD_SHADE * min(canopy, 1.0));""", """  c *= max(tone, 0.0) * (1.0 - SWARD_SHADE * min(b.canopy, 1.0));"""),
("""// The cover's field at `at` seen at `e` m a pixel on ground soaked to `soaked` under `canopy` of the trees' crowns,
// premultiplied: the herbs' sward, then each lattice as far as its marks do not show, the lower under the taller,
// thicker and thinner over the patches (`patchy`), with its grain (`grain`); the trees' in their stands, their canopy
// seen as closer (`CANOPY_SEEN`).
fn cover_field(at: vec2f, e: f32, soaked: f32, canopy: f32) -> vec4f {
  // The patches as far as they show: where they take less than a pixel the field is their mean.
  let thick = 1.0 + (patchy(at) - 1.0) * shows_at(PATCH_M, e);
  var dst = sward(at, e, soaked, thick, canopy);""", """// The cover's field at `at` seen at `e` m a pixel over `b`, premultiplied: the herbs' sward, then each lattice as far as
// its marks do not show, the lower under the taller, thicker and thinner over the patches (`patchy`), as far as it holds
// there (`held_at`), with its grain (`grain`); the trees' in their stands, their canopy seen as closer (`CANOPY_SEEN`)
// and snowy as the ground under it.
fn cover_field(at: vec2f, e: f32, b: Beneath) -> vec4f {
  // The patches as far as they show: where they take less than a pixel the field is their mean.
  let thick = 1.0 + (patchy(at) - 1.0) * shows_at(PATCH_M, e);
  let hold = held_at(at, b);
  var dst = sward(at, e, b, thick, hold.w);"""),
("""    var c = f.rgb / f.a;
    var share = f.a;
    if (l == STANDS) {
      c = in_stands(at, e, f, STANDS_FIRST, STAND_M) * (1.0 + CANOPY_TONE * noise_turned(at / CANOPY_M, 79) * shows_at(CANOPY_M, e));
      share = 1.0 - pow(1.0 - min(f.a, 1.0), CANOPY_SEEN);
    }""", """    var c = f.rgb / f.a;
    var share = min(f.a * hold[l], 1.0);
    if (share <= 0.0) { continue; }
    if (l == STANDS) {
      c = in_stands(at, e, f, STANDS_FIRST, STAND_M) * (1.0 + CANOPY_TONE * noise_turned(at / CANOPY_M, 79) * shows_at(CANOPY_M, e));
      c = mix(c, vec3f(1.0), CANOPY_SNOW * b.snow);
      share = 1.0 - pow(1.0 - share, CANOPY_SEEN);
    }"""),
])
edit('web/shaders/land.wgsl', [
("""  p.canopy = field_at(at, STANDS, 0.0).a;
  p.cover = cover_field(at, e, soaked(p), p.canopy);
  p.z = g.h;
  p.slope = length(g.slope) * look.x;
  p.shade = hill_shade(g.slope, EXAGGERATE * look.x);
  p.loose = loose_at(at);""", """  p.canopy = field_at(at, STANDS, 0.0).a;
  p.z = g.h;
  p.slope = length(g.slope) * look.x;
  p.shade = hill_shade(g.slope, EXAGGERATE * look.x);
  p.loose = loose_at(at);
  p.cover = cover_field(at, e, Beneath(soaked(p), p.canopy, p.loose, st.snow, st.snow_m));"""),
])
edit('web/src/textures.ts', [
("import { clamp } from './earth';", "import { clamp } from './earth';\nimport { ROOT_LEAST, ROOT_PER_HEIGHT } from './roots';"),
(".vec4(this.kinds ? 1 : 0, this.alpha);", ".vec4(this.kinds ? 1 : 0, this.alpha, ROOT_PER_HEIGHT, ROOT_LEAST);"),
])
