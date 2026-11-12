W = '/home/user/primordial-simulation/.claude/worktrees/agent-a257b588d76bf8a62/'
def edit(path, pairs):
    s = open(W + path).read()
    for a, b in pairs:
        assert a in s, (path, a[:80])
        s = s.replace(a, b)
    open(W + path, 'w').write(s)

edit('web/shaders/lib/cover_field.wgsl', [
# the cover_field body
("""fn cover_field(at: vec2f, e: f32, b: Beneath) -> CoverAt {
  // The patches as far as they show: where they take less than a pixel the field is their mean (and no noise is read).
  let patches = shows_at(PATCH_M, e);
  let thick = select(1.0, 1.0 + (patchy(at) - 1.0) * patches, patches > 0.0);
  let hold = held_at(at, b);
  // The trees' field, read once: their crowns' share, and their layer below (plainly blended where their marks show in
  // its place: only the shade under them reads it).
  var trees = vec4f(0.0);
  if (cover_look.sides[STANDS] > 0.0) {
    trees = select(field_smooth(at, STANDS), field_at(at, STANDS, 0.0), cover_look.shown[STANDS] >= 1.0);
  }
  var dst = sward(at, e, b, trees.a, thick, hold.w);
  for (var l = 0; l < COVER_LATTICES; l++) {
    let unshown = 1.0 - cover_look.shown[l];
    if (unshown <= 0.0 || cover_look.sides[l] <= 0.0) { continue; }
    var f = trees;
    if (l != STANDS) { f = field_smooth(at, l); }
    if (f.a <= 0.0) { continue; }
    var c = f.rgb / f.a;
    var share = min(f.a * hold[l], 1.0);
    if (share <= 0.0) { continue; }
    if (l == STANDS) {
      c = in_stands(at, e, f, STANDS_FIRST, STAND_M);
      let groups = shows_at(CANOPY_M, e);
      if (groups > 0.0) { c *= 1.0 + CANOPY_TONE * noise_turned(at / CANOPY_M, 79) * groups; }
      c = mix(c, vec3f(1.0), CANOPY_SNOW * b.snow);
      share = 1.0 - pow(1.0 - share, CANOPY_SEEN);
    }
    let g = grain(at, l, share * thick, e);
    let a = g.x * unshown;
    dst = vec4f(c * g.y * a, a) + dst * (1.0 - a);
  }
  return CoverAt(dst, trees.a);
}""", """fn cover_field(at: vec2f, e: f32, b: Beneath) -> CoverAt {
  // The patches as far as they show: where they take less than a pixel the field is their mean (and no noise is read).
  let patches = shows_at(PATCH_M, e);
  let thick = select(1.0, 1.0 + (patchy(at) - 1.0) * patches, patches > 0.0);
  let hold = held_at(at, b);
  // The trees' field, read once: their crowns' share, and their layer below (plainly blended where their marks show in
  // its place: only the shade under them reads it).
  var trees = vec4f(0.0);
  if (cover_look.sides[STANDS] > 0.0) {
    trees = select(field_smooth(at, STANDS), field_at(at, STANDS, 0.0), cover_look.shown[STANDS] >= 1.0);
  }
  // Under the trees: the sward and the low lattices one by one near, as one layer far (`LOW_FAR_E`).
  let far = ease(LOW_FAR_E, LOW_FAR_FULL_E, e);
  var dst = vec4f(0.0);
  if (far < 1.0) {
    dst = sward(at, e, b, trees.a, thick, hold.w);
    for (var l = 0; l < STANDS; l++) { dst = lattice_field(at, e, l, field_smooth(at, l), b, thick, hold[l], dst); }
  }
  if (far > 0.0) { dst = mix(dst, field_smooth(at, LOW_LAYER) * hold.w, far); }
  dst = lattice_field(at, e, STANDS, trees, b, thick, hold[STANDS], dst);
  return CoverAt(dst, trees.a);
}

// Lattice l's field `f` at `at`, seen at `e` m a pixel over `b`, as far as its marks do not show, thicker and thinner by
// `thick`, holding `hold` of its plants there, with its grain, over `dst` (premultiplied); the trees' in their stands,
// their canopy seen as closer (`CANOPY_SEEN`) and snowy as the ground under it. A lattice no square holds (its side 0,
// textures.ts) adds nothing.
fn lattice_field(at: vec2f, e: f32, l: i32, f: vec4f, b: Beneath, thick: f32, hold: f32, dst: vec4f) -> vec4f {
  let unshown = 1.0 - cover_look.shown[l];
  if (unshown <= 0.0 || cover_look.sides[l] <= 0.0 || f.a <= 0.0) { return dst; }
  var c = f.rgb / f.a;
  var share = min(f.a * hold, 1.0);
  if (share <= 0.0) { return dst; }
  if (l == STANDS) {
    c = in_stands(at, e, f, STANDS_FIRST, STAND_M);
    let groups = shows_at(CANOPY_M, e);
    if (groups > 0.0) { c *= 1.0 + CANOPY_TONE * noise_turned(at / CANOPY_M, 79) * groups; }
    c = mix(c, vec3f(1.0), CANOPY_SNOW * b.snow);
    share = 1.0 - pow(1.0 - share, CANOPY_SEEN);
  }
  let g = grain(at, l, share * thick, e);
  let a = g.x * unshown;
  return vec4f(c * g.y * a, a) + dst * (1.0 - a);
}"""),
("""// The cover's field at `at` seen at `e` m a pixel over `b`, premultiplied: the herbs' sward, then each lattice as far as
// its marks do not show, the lower under the taller, thicker and thinner over the patches (`patchy`), as far as it holds
// there (`held_at`), with its grain (`grain`); the trees' in their stands, their canopy seen as closer (`CANOPY_SEEN`)
// and snowy as the ground under it. A lattice no square holds (its side 0, textures.ts) is not read.""",
"""// The cover's field at `at` seen at `e` m a pixel over `b`, premultiplied, and the trees' crowns' share: the herbs'
// sward, then each lattice (`lattice_field`), the lower under the taller, thicker and thinner over the patches
// (`patchy`), as far as it holds there (`held_at`); far away, where the sward's and the low lattices' detail no longer
// shows, those three as their one layer (canopy.ts `FIELD.low`)."""),
# layers and constants
("""const TALL_LAYER = 7; const ROOTED_LAYER = 8;""", """const TALL_LAYER = 7; const ROOTED_LAYER = 8;
// The sward and the low lattices as one layer (canopy.ts `lowOf`), drawn in their place where a pixel takes
// `LOW_FAR_E` m and more (where their detail and patches no longer show), fully from `LOW_FAR_FULL_E`.
const LOW_LAYER = 9; const LOW_FAR_E = 6.0; const LOW_FAR_FULL_E = 8.0;"""),
("""// `SWARD_SEEN` (an Earth view's style).""", """// `cover_look.sides.w` (canopy.ts `SWARD_SEEN`, an Earth view's style)."""),
("""const SWARD = 0.45; const SWARD_M = 0.35; const SWARD_SEEN = 5.0;""", """const SWARD = 0.45; const SWARD_M = 0.35;"""),
("""// (`SWARD_SEEN`) with its tufts""", """// (canopy.ts `SWARD_SEEN`) with its tufts"""),
("""  let seen = 1.0 - pow(1.0 - s, SWARD_SEEN);""", """  let seen = 1.0 - pow(1.0 - s, cover_look.sides.w);"""),
("""  // Of each lattice: its cell's side, m (0: no square holds it), and how strongly its marks show (the rest is the field).""",
 """  // Of each lattice: its cell's side, m (0: no square holds it), and how strongly its marks show (the rest is the field);
  // and how much more than its share the sward is seen to cover (`sides.w`, canopy.ts `SWARD_SEEN`)."""),
])

edit('web/src/textures.ts', [
(".vec4(...LATTICES.map((a, l) => (this.kinds?.held[l] ? a : 0))).vec4(", ".vec4(...LATTICES.map((a, l) => (this.kinds?.held[l] ? a : 0)), SWARD_SEEN).vec4("),
("import { FIELD, Kinds, LATTICES, KIND_PLANES, half } from './canopy';", "import { FIELD, Kinds, LATTICES, KIND_PLANES, SWARD_SEEN, half } from './canopy';"),
])

edit('web/src/canopy.ts', [
("""/** The layers of the field (`CoverField`): one a lattice (its marks' plants), then the herbs' sward, the herbs' flowers,
 * of the sward and of the lattice of stands the part its first kind makes (`Paint`), and of each lattice and the sward
 * (red, green, blue, alpha) its plants' mean height, m, and their mean rooted share in the square (roots.ts). */
export const FIELD = { sward: LATTICES.length, flowers: LATTICES.length + 1, swardFirst: LATTICES.length + 2, standsFirst: LATTICES.length + 3, tall: LATTICES.length + 4, rooted: LATTICES.length + 5, count: LATTICES.length + 6 } as const;""",
"""/** The layers of the field (`CoverField`): one a lattice (its marks' plants), then the herbs' sward, the herbs' flowers,
 * of the sward and of the lattice of stands the part its first kind makes (`Paint`), of each lattice and the sward
 * (red, green, blue, alpha) its plants' mean height, m, and their mean rooted share in the square (roots.ts), and the
 * sward and the lattices under the trees' as one layer, as the field draws them far away (`lowOf`). */
export const FIELD = { sward: LATTICES.length, flowers: LATTICES.length + 1, swardFirst: LATTICES.length + 2, standsFirst: LATTICES.length + 3, tall: LATTICES.length + 4, rooted: LATTICES.length + 5, low: LATTICES.length + 6, count: LATTICES.length + 7 } as const;
/** Seen from above the herbs' leaves leaning and their shade cover more than their share: the ground they leave is
 * (1 - share) to this power (cover_field.wgsl `sward`, `lowOf`; an Earth view's style). */
export const SWARD_SEEN = 5;"""),
("""    [...paint, sward].forEach((p, q) => p.putRoots(field, layer(FIELD.tall, i) + q, layer(FIELD.rooted, i) + q));""",
 """    [...paint, sward].forEach((p, q) => p.putRoots(field, layer(FIELD.tall, i) + q, layer(FIELD.rooted, i) + q));
    lowOf(field, i, layer);"""),
("""/** The levels of field `data` (`w` x `h`) below its own,""", """/** Into the far layer of `field` at square `i` (`FIELD.low`, `layer` its place): the sward as it is seen
 * (`SWARD_SEEN`), the lattices under the trees' over it, each over the one below as cover_field.wgsl lays them, at their
 * mean (no patches, no grain: far away neither shows). */
function lowOf(field: Float32Array, i: number, layer: (l: number, i: number) => number): void {
  const s = layer(FIELD.sward, i), sward = field[s + 3], seen = sward > 0 ? 1 - (1 - Math.min(sward, 1)) ** SWARD_SEEN : 0;
  const dst = [0, 1, 2].map(q => (sward > 0 ? (field[s + q] / sward) * seen : 0));
  let a = seen;
  for (let l = 0; l < STANDS; l++) {
    const at = layer(l, i), share = Math.min(field[at + 3], 1);
    if (share <= 0) continue;
    for (let q = 0; q < 3; q++) dst[q] = (field[at + q] / field[at + 3]) * share + dst[q] * (1 - share);
    a = share + a * (1 - share);
  }
  const out = layer(FIELD.low, i);
  dst.forEach((v, q) => { field[out + q] = v; });
  field[out + 3] = a;
}

/** The levels of field `data` (`w` x `h`) below its own,"""),
])

edit('docs/numbers.md', [
("| `SWARD_SEEN` (5) and `CANOPY_SEEN` (2), `web/shaders/lib/cover_field.wgsl` |", "| `SWARD_SEEN` (5, `web/src/canopy.ts`) and `CANOPY_SEEN` (2, `web/shaders/lib/cover_field.wgsl`) |"),
])
