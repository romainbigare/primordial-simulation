W = '/home/user/primordial-simulation/.claude/worktrees/agent-a257b588d76bf8a62/'
def edit(path, pairs):
    s = open(W + path).read()
    for a, b in pairs:
        assert a in s, (path, a[:70])
        s = s.replace(a, b)
    open(W + path, 'w').write(s)

edit('web/shaders/cover.wgsl', [
("// uses: draw.wgsl water.wgsl style.wgsl land.wgsl cover_field.wgsl crown.wgsl", "// uses: draw.wgsl water.wgsl style.wgsl land.wgsl rock.wgsl cover_field.wgsl crown.wgsl"),
("""// is drier than its cell's mean as the kind's activity law says; on the trees' lattice""", """// is drier than its cell's mean as the kind's activity law says and where its loose ground is thinner than its roots
// need (land.wgsl `rooted`, its chance raised where its square's ground is thin, so the square keeps its cover); a plant
// buried by the snow's depth against its height does not show, one standing in snow carries it on its crown; on the
// trees' lattice"""),
("const CHANCE = 0; const WIDTH = 3; const NEED = 6; const FORM = 7; const FROM = 11; const TO = 15; const PETAL = 19; const SHAPE = 23;",
 "const CHANCE = 0; const WIDTH = 3; const NEED = 6; const ROOTS = 7; const FORM = 8; const FROM = 12; const TO = 16; const PETAL = 20; const SHAPE = 24;"),
("""// What the ground at `at` leaves a plant: the share free of liquid over its roots and of snow, how wet it is and how
// wet its cells are on their mean (land.wgsl `state_at`).
struct Foot { stands: f32, wet: f32, mean: f32 }
fn foot(at: vec2f) -> Foot {
  let st = state_at(at, ground_at(at, false), corners(at / land.map.x));
  return Foot(stands_in(st.depth, st.snow), st.wet, st.mean);
}""", """// What the ground at `at` leaves a plant: the share free of liquid over its roots, how wet it is and how wet its cells
// are on their mean (land.wgsl `state_at`), the depth of its loose ground, m, the share snow covers and the snow's
// depth, m.
struct Foot { stands: f32, wet: f32, mean: f32, loose: f32, snow: f32, snow_m: f32 }
fn foot(at: vec2f) -> Foot {
  let st = state_at(at, ground_at(at, false), corners(at / land.map.x));
  return Foot(above_water(st.depth), st.wet, st.mean, loose_at(at), st.snow, st.snow_m);
}

// The share of each kind that stands where its roots need `need` m of loose ground and `loose` m lies (land.wgsl
// `rooted`).
fn holds(loose: f32, need: vec4f) -> vec4f {
  return vec4f(rooted(loose, need.x), rooted(loose, need.y), rooted(loose, need.z), rooted(loose, need.w));
}"""),
("""    w[s] = abs(chance[s]) * thick * thrive(f.wet, f.mean, kind(NEED, cs.at[s])) * f.stands;""",
 """    w[s] = abs(chance[s]) * thick * thrive(f.wet, f.mean, kind(NEED, cs.at[s])) * holds(f.loose, kind(ROOTS, cs.at[s])) * f.stands;"""),
("""      plant(dst, plant_of(l, cs.at[pick.x], pick.y, c, h.z, rand3(id, l + COVER_LATTICES), rise + side), p, e, fade);""",
 """      let q = plant_of(l, cs.at[pick.x], pick.y, c, h.z, rand3(id, l + COVER_LATTICES), rise + side);
      let shows = above_snow(f.snow_m, q.h);
      if (shows > 0.0) { plant(dst, q, p, e, fade * shows, f.snow); }"""),
("""// Plant q over dst at p, e m a pixel, as strongly as `fade`: its shadow, its stem (a tree), its body by its design, and
// a bush's flowers or fruit.
fn plant(dst: ptr<function, vec4f>, q: Plant, p: vec2f, e: f32, fade: f32) {""",
 """// Plant q over dst at p, e m a pixel, as strongly as `fade`, where snow covers `snow` of the ground: its shadow, its stem
// (a tree), its body by its design (a closed crown lit from above carries the snow, `CROWN_SNOW`), and a bush's flowers
// or fruit.
fn plant(dst: ptr<function, vec4f>, q: Plant, p: vec2f, e: f32, fade: f32, snow: f32) {"""),
("""  if (body.a > 0.0) { over(dst, mark_colour(q.rgb, body.lit, body.rim), body.a * fade); }""",
 """  if (body.a > 0.0) {
    let snowy = CROWN_SNOW * snow * q.closed * smoothstep(SNOW_LIT.x, SNOW_LIT.y, body.lit);
    over(dst, mark_colour(mix(q.rgb, CROWN_SNOW_RGB, snowy), body.lit, body.rim), body.a * fade);
  }"""),
("""const SHADOW = 0.42; const SIZES = 0.3; const LEAN = 0.08;""", """const SHADOW = 0.42; const SIZES = 0.3; const LEAN = 0.08;
// Snow lies on a crown's upper, lit side (its light from `SNOW_LIT.x` to `SNOW_LIT.y` times flat ground's) up to this
// share of its colour, as closed as the crown is (a bare one holds little), in the colour of fresh snow (colour.wgsl
// `SNOW`).
const CROWN_SNOW = 0.8; const SNOW_LIT = vec2f(0.9, 1.15); const CROWN_SNOW_RGB = vec3f(0.90, 0.92, 0.95);"""),
])
