def edit(p, pairs):
    t = open(p).read()
    for a, b in pairs:
        assert t.count(a) == 1, (p, a[:70], t.count(a))
        t = t.replace(a, b)
    open(p, 'w').write(t)

edit('web/shaders/lib/stone.wgsl', [
 ("""// The one maker of a stone of the map (colour.wgsl `ground_of`, land.wgsl): a block of rock lying on the ground, seen
// from above. Its outline is what `STONE_CUTS` planes leave of it, each at its own angle and distance from the centre
// (`STONE_CUT`), so no stone is a disc and no two are alike; it is longer one way than the other (`STONE_LONG`). Each""",
  """// The one maker of a stone of the map (colour.wgsl `ground_of`, land.wgsl): a block of rock lying on the ground, seen
// from above. Its outline is what five to eight planes leave of it (`STONE_CUTS`, each stone its own count), each at
// its own angle and distance from the centre (`STONE_CUT`), so no stone is a disc and no two are alike; it is longer
// one way than the other (`STONE_LONG`). Each"""),
 ("""// turned (draw.wgsl `LIGHT`), the stone throws a shadow on the ground away from the sun (`STONE_SHADOW`), and loose
// ground or snow about it buries its foot (`sunk`). It is its bed's colour (`STONE_TONE`, each stone a little its own,
// `STONE_TONES`) with the bands of the layers it broke from (`STONE_BANDS`). Stones lie one to a cell of two lattices
// turned from one another, in `STONE_OCTAVES` sizes each `STONE_STEP` times smaller than the last, the smaller the
// more of them (debris has many small pieces and few large: the largest cells hold `STONE_FEW` as often): fixed in the
// world, and each size drawn as itself from `STONE_PX` pixels across, below as the tone its stones give the ground.
const STONE_CUTS = 5; const STONE_CUT = 0.38; const STONE_LONG = 0.7; const STONE_TOP = 0.75; const STONE_TILT = 0.3;
const STONE_SOFT = vec2f(0.04, 0.45); const STONE_FACE = vec2f(1.7, 1.2); const STONE_RISE = 1.1;
const STONE_SHADOW = 0.3; const STONE_SHADOW_REACH = 0.4;
const STONE_TONE = 1.15; const STONE_TONES = 0.18; const STONE_BANDS = 0.12; const STONE_BAND_M = 0.35;
const STONE_OCTAVES = 3; const STONE_STEP = 2.2; const STONE_FEW = 0.5;
const STONE_GAP = 3.0; const STONE_SIZES = 0.4; const STONE_PX = 2.0;
// Snow lies on a stone's flat top (where its face is flatter than `SNOW_TOP`); a stone smaller than `SNOW_BURIES_M`
// is under the snow that covers its ground, one larger than twice that stands out of it.
const SNOW_TOP = vec2f(0.8, 0.95); const SNOW_BURIES_M = 0.7;
// The largest stone drawn, m (tape/src/ground/rock.rs `STONE_MOST_M`): no draw shows one from this many m a pixel on.
const STONE_MOST_M = 2.0;""",
  """// turned (draw.wgsl `LIGHT`), the stone throws a shadow on the ground away from the sun (`STONE_SHADOW`), and loose
// ground about it buries its foot (`sunk`). It is its bed's colour (`STONE_TONE`, each stone a little its own,
// `STONE_TONES`) with the bands of the layers it broke from (`STONE_BANDS`). Stones lie one to a cell of two lattices
// turned from one another, in `STONE_OCTAVES` sizes each `STONE_STEP` times smaller than the last, the smaller the
// more of them (debris has many small pieces and few large: the second largest cells hold `STONE_FEW` as often, the
// largest as often as the place says, `Stony.rare`, on a scree `STONE_RARE`): fixed in the world, and each size drawn
// as itself from `STONE_PX` pixels across, below as the tone its stones give the ground. A stone and its shadow lie
// within `STONE_REACH` of its radii of its centre.
const STONE_CUTS = vec2i(5, 8); const STONE_CUT = 0.38; const STONE_LONG = 0.7; const STONE_TOP = 0.75; const STONE_TILT = 0.3;
const STONE_SOFT = vec2f(0.04, 0.45); const STONE_FACE = vec2f(1.7, 1.2); const STONE_RISE = 1.1;
const STONE_SHADOW = 0.45; const STONE_SHADOW_REACH = 0.4; const STONE_REACH = 1.6;
const STONE_TONE = 1.15; const STONE_TONES = 0.18; const STONE_BANDS = 0.12; const STONE_BAND_M = 0.35;
const STONE_OCTAVES = 4; const STONE_STEP = 2.2; const STONE_FEW = 0.5; const STONE_RARE = 0.15;
const STONE_GAP = 3.0; const STONE_SIZES = 0.4; const STONE_PX = 2.0;
// Snow buries a stone as it buries a plant, by its depth against the stone's height (draw.wgsl `above_snow`); on one
// that stands out of it, it lies on the flat top (where its face is flatter than `SNOW_TOP`) once it is `SNOW_CAP_M`
// deep.
const SNOW_TOP = vec2f(0.8, 0.95); const SNOW_CAP_M = 0.1;
// The largest stone drawn, m: no draw shows one from this many m a pixel on.
const STONE_MOST_M = 4.5;"""),
 ("""  let y = x * 747796405u + 2891336453u;
  // The stone's own frame: turned, and squeezed across its long axis.""",
  """  let y = x * 747796405u + 2891336453u;
  let z = y * 747796405u + 2891336453u;
  let cuts = STONE_CUTS.x + i32((x >> 22u) % u32(STONE_CUTS.y - STONE_CUTS.x + 1));
  // The stone's own frame: turned, and squeezed across its long axis."""),
 ("""  for (var i = 0; i < STONE_CUTS; i++) {
    let bits = (y >> u32(6 * i)) & 63u;
    let a = (f32(i) + 0.8 * (f32(bits & 7u) / 7.0 - 0.5)) * (2.0 * PI / f32(STONE_CUTS));""",
  """  for (var i = 0; i < STONE_CUTS.y; i++) {
    if (i >= cuts) { break; }
    let bits = (select(z, y, i < 5) >> u32(6 * (i % 5))) & 63u;
    let a = (f32(i) + 0.8 * (f32(bits & 7u) / 7.0 - 0.5)) * (2.0 * PI / f32(cuts));"""),
 ("""// what each stone is of that size there, 0..1 (this may change smoothly: finer up a bar); how worn they are (0 angular
// .. 1 rounded), how deep their feet are sunk (0..1 of their height), and the snow on their ground, 0..1.
struct Stony { density: f32, size: f32, small: f32, worn: f32, sunk: f32, snow: f32 }""",
  """// what each stone is of that size there, 0..1 (this may change smoothly: finer up a bar); how worn they are (0 angular
// .. 1 rounded), how deep their feet are sunk (0..1 of their height), the snow on their ground, 0..1, and its depth
// where it lies, m; and the share of the cells of the largest size that hold a block.
struct Stony { density: f32, size: f32, small: f32, worn: f32, sunk: f32, snow: f32, snow_m: f32, rare: f32 }

// The share of the cells of octave `o` (0: the largest) that hold a stone of `s`.
fn stones_held(s: Stony, o: i32) -> f32 {
  return s.density * select(select(1.0, STONE_FEW, o == 1), s.rare, o == 0);
}"""),
 ("""  let tone = bed * STONE_TONE;
  var size = s.size / pow(STONE_STEP, f32(STONE_OCTAVES - 1));
  // The smallest first, the larger over them.
  for (var o = STONE_OCTAVES - 1; o >= 0; o--) {
    let density = s.density * select(1.0, STONE_FEW, o == 0);
    // Under the snow that covers its ground, a small stone does not show.
    var stands = 1.0 - s.snow * (1.0 - ease(SNOW_BURIES_M, 2.0 * SNOW_BURIES_M, size));
    if (least > 0.0) { stands *= ease(least, 1.5 * least, s.small * size); }
    // Smaller than a few pixels: the share of the ground they cover, as a tone.
    var near = 0.0;
    if (!unseen(STONE_MOST_M / STONE_PX)) { near = ease(STONE_PX * e, 2.0 * STONE_PX * e, s.small * size); }
    let mean = 2.0 * density * stands * PI * 0.25 * 0.7 * s.small * s.small / (STONE_GAP * STONE_GAP) * (1.0 - near);
    out = mix(out, vec4f(tone, 1.0), mean);
    if (near > 0.0 && stands > 0.0) {
      for (var l = 0; l < 2; l++) {
        let p = placed(at, size, density, o, l, k);
        if (!p.held) { continue; }
        let r = s.small * p.r;
        let v = p.v / r;""",
  """  let tone = bed * STONE_TONE;
  let snow_m = s.snow * s.snow_m;
  var size = s.size / pow(STONE_STEP, f32(STONE_OCTAVES - 1));
  // The smallest first, the larger over them.
  for (var o = STONE_OCTAVES - 1; o >= 0; o--) {
    let density = stones_held(s, o);
    // Out of the water, where it stands in a channel's.
    var stands = 1.0;
    if (least > 0.0) { stands = ease(least, 1.5 * least, s.small * size); }
    // Smaller than a few pixels: the share of the ground they cover, as a tone (as far as the snow leaves them).
    var near = 0.0;
    if (!unseen(STONE_MOST_M / STONE_PX)) { near = ease(STONE_PX * e, 2.0 * STONE_PX * e, s.small * size); }
    let high = STONE_TOP * (1.0 - s.sunk);
    let mean = 2.0 * density * stands * above_snow(snow_m, high * 0.5 * s.small * size) * PI * 0.25 * 0.7 * s.small * s.small / (STONE_GAP * STONE_GAP) * (1.0 - near);
    out = mix(out, vec4f(tone, 1.0), mean);
    if (near > 0.0 && stands > 0.0 && density > 0.0) {
      for (var l = 0; l < 2; l++) {
        let p = placed(at, size, density, o, l, k);
        let r = s.small * p.r;
        if (!p.held || dot(p.v, p.v) > STONE_REACH * STONE_REACH * r * r) { continue; }
        // Under snow deeper than it is tall, a stone does not show.
        let shows = near * stands * above_snow(snow_m, high * r);
        if (shows <= 0.0) { continue; }
        let v = p.v / r;"""),
 ("""        out = mix(out, vec4f(0.0, 0.0, 0.0, 1.0), shade * near * stands * (1.0 - st.a));""", """        out = mix(out, vec4f(0.0, 0.0, 0.0, 1.0), shade * shows * (1.0 - st.a));"""),
 ("""        let cap = s.snow * smoothstep(SNOW_TOP.x, SNOW_TOP.y, st.flat);
        let rgb = mix(lit_by(tone * own * bands, 0.35 + 0.65 * st.lit), SNOW, cap);
        out = mix(out, vec4f(rgb, 1.0), st.a * near * stands);""",
  """        let cap = ease(0.0, SNOW_CAP_M, snow_m) * smoothstep(SNOW_TOP.x, SNOW_TOP.y, st.flat);
        let rgb = mix(lit_by(tone * own * bands, 0.35 + 0.65 * st.lit), SNOW, cap);
        out = mix(out, vec4f(rgb, 1.0), st.a * shows);"""),
 ("""    if (stands > 0.0) {
      let density = s.density * select(1.0, STONE_FEW, o == 0);
      for (var l = 0; l < 2; l++) {""", """    let density = stones_held(s, o);
    if (stands > 0.0 && density > 0.0) {
      for (var l = 0; l < 2; l++) {"""),
])

edit('web/shaders/lib/crag.wgsl', [
 ("""// Stones (stone.wgsl): on a slope from `SCREE_FROM` to `CLIFF_FROM` where the loose ground is thinner than
// `SCREE_LOOSE_M` (the scree below a face), angular blocks up to `SCREE_STONE_M` across in at most `SCREE` of the
// places they could lie; on bare bed, `STONES_ON_BED` of them; none on a beach. A stone's foot is sunk by up to""",
  """// Stones (stone.wgsl): bare bed sheds blocks, which lie thick at its foot and thin out down the slope. So they lie
// where the loose ground is thinner than `SCREE_LOOSE_M` (the thinner the more of them: the square of how thin), on
// a slope from `SCREE_FROM` (all of them from `SCREE_FULL`), and below the bed that sheds them (where the loose
// ground deepens down the slope; `BELOW_LEAST` of them where it deepens up it): angular blocks up to `SCREE_STONE_M`
// across in at most `SCREE` of the places they could lie; on bare bed, `STONES_ON_BED` of them; none on a beach. A
// stone's foot is sunk by up to"""),
 ("""const SCREE_FROM = 0.6; const SCREE_LOOSE_M = 1.0; const SCREE_STONE_M = 1.2; const SCREE = 0.7;""",
  """const SCREE_FROM = 0.25; const SCREE_FULL = 0.7; const SCREE_LOOSE_M = 1.0; const SCREE_STONE_M = 3.6; const SCREE = 0.8;
const BELOW_LEAST = 0.2;"""),
])

edit('web/shaders/lib/colour.wgsl', [
 ("""  // of that size, the share of the places that hold one (0: none), and how round the channel has worn them, 0..1.
  bed: vec3f, rock: f32, loose: f32, stone: f32, small: f32, stony: f32, worn: f32,""",
  """  // of that size, the share of the places that hold one (0: none), how round the channel has worn them, 0..1, and
  // the share of the places of the largest that hold a block; the depth of the snow where it lies, m; and how far the
  // point lies below the bed that sheds its scree, `BELOW_LEAST`..1 (crag.wgsl).
  bed: vec3f, rock: f32, loose: f32, stone: f32, small: f32, stony: f32, worn: f32, rare: f32, snow_m: f32, below: f32,"""),
 ("""    let scree = max(SCREE * ease(SCREE_FROM, CLIFF_FROM, steep) * thin, STONES_ON_BED * p.rock) * (1.0 - p.sand);""",
  """    let scree = max(SCREE * ease(SCREE_FROM, SCREE_FULL, steep) * thin * thin * p.below, STONES_ON_BED * p.rock) * (1.0 - p.sand);"""),
 ("""      var s = Stony(scree * (1.0 - plants), SCREE_STONE_M, 1.0, 0.0, sunk, snow);
      if (k == 1) { s = Stony(p.stony * (1.0 - p.sand), p.stone, p.small, p.worn, BED_SUNK, snow); }""",
  """      var s = Stony(scree * (1.0 - plants), SCREE_STONE_M, 1.0, 0.0, sunk, snow, p.snow_m, STONE_RARE);
      if (k == 1) { s = Stony(p.stony * (1.0 - p.sand), p.stone, p.small, p.worn, BED_SUNK, snow, p.snow_m, p.rare); }"""),
])

edit('web/shaders/lib/earth.wgsl', [
 ("""  s.worn = 0.0;
""", """  s.worn = 0.0;
  s.rare = 0.0;
  s.snow_m = 0.0;
  s.below = 1.0;
"""),
])

edit('web/shaders/land.wgsl', [
 ("""// line, thinning. A channel wears its stones round as it carries them:""", """// line, thinning, and there the bed is bare (`WALL` of it: a steep reach runs between walls of its bed); the largest
// of all are blocks a steep reach alone holds (`Stony.rare`). A channel wears its stones round as it carries them:"""),
 ("""const IN_CHANNEL = 0.6; const STONES_OUT = 1.8; const WORN_Q = vec2f(0.005, 0.3); const BAR_FINES = 0.5;""",
  """const IN_CHANNEL = 0.6; const STONES_OUT = 1.8; const WORN_Q = vec2f(0.005, 0.3); const BAR_FINES = 0.5; const WALL = 0.85;"""),
 ("""  let moved = select(0.0, mix(stone_of(reach), stone_of(next.s), next.share), ch.bed > 0.0);
  p.stone = max(STONE_SEEN * stone_of(reach), COBBLE_M);
  p.small = clamp(max(STONE_SEEN * moved, COBBLE_M) / p.stone, STONE_SMALL.x, STONE_SMALL.y);
  let beside = ease(STEEP_STONE.x, STEEP_STONE.y, moved) * (1.0 - ease(ch.bed, STONES_OUT * ch.bed, g.d));
  p.stony = IN_CHANNEL * ease(0.0, BAR_SAND_STONE, moved) * select(beside, 1.0, g.d < ch.bed);""",
  """  let moved = select(0.0, mix(stone_of(reach), stone_of(next.s), next.share), ch.bed > 0.0);
  let cobble = max(STONE_SEEN * stone_of(reach), COBBLE_M);
  p.stone = min(STONE_STEP * cobble, STONE_MOST_M);
  p.small = clamp(max(STONE_SEEN * moved, COBBLE_M) / cobble, STONE_SMALL.x, STONE_SMALL.y);
  let steep_reach = ease(STEEP_STONE.x, STEEP_STONE.y, moved);
  let beside = steep_reach * (1.0 - ease(ch.bed, STONES_OUT * ch.bed, g.d));
  p.stony = IN_CHANNEL * ease(0.0, BAR_SAND_STONE, moved) * select(beside, 1.0, g.d < ch.bed);
  p.rare = STONE_RARE * steep_reach;
  p.rock = max(p.rock, WALL * beside);
  p.cover *= 1.0 - WALL * beside;
  // The scree lies below the bed that sheds it: where the loose ground deepens down the slope.
  let deepens = vec2f(dpdx(loose), dpdy(loose));
  p.below = mix(BELOW_LEAST, 1.0, 0.5 - 0.5 * dot(deepens, g.slope) / max(length(deepens) * length(g.slope), 1e-9));
  p.snow_m = st.snow_m;"""),
 ("""    let s = Stony(p.stony, p.stone, p.small, p.worn, BED_SUNK, 0.0);""", """    let s = Stony(p.stony, p.stone, p.small, p.worn, BED_SUNK, 0.0, 0.0, p.rare);"""),
 ("""  if (resolved > 0.0) { out.flow = resolved * flow_signs(e, g.along, x, ch, r.steep); }""",
  """  if (resolved > 0.0) {
    // (Below a step's white water a steep reach's water lies in a pool: slower, and `STEP_POOL` deeper.)
    let signs = flow_signs(e, g.along, x, ch, r.steep);
    out.wet.depth *= 1.0 + STEP_POOL * resolved * (r.steep - signs.z);
    out.flow = resolved * signs;
  }"""),
])

edit('web/shaders/lib/flow.wgsl', [
 ("""// of the channel's width only, in patches `STEP_PATCH` steps long; the water between runs fast and shallow. The signs""",
  """// of the channel's width only, in patches `STEP_PATCH` steps long; past the white water it lies in a small pool
// (`STEP_POOL` deeper, `POOL_CALM` slower) up to the lip of the next step, where it runs fast and shallow. The signs"""),
 ("""const STEP_M = 8.0; const STEP_WAVE = 0.7; const STEP_FADE = 0.3; const STEP_PATCH = 2.0; const FOAM_STEEP = 0.9;""",
  """const STEP_M = 8.0; const STEP_WAVE = 0.7; const STEP_FADE = 0.3; const STEP_PATCH = 2.0; const FOAM_STEEP = 0.9;
const STEP_POOL = 1.6; const POOL_CALM = 0.8;"""),
 ("""    f.y = max(f.y, FOAM_STEEP * shown * steep * broken * fall * torn * sqrt(mid));""",
  """    f.y = max(f.y, FOAM_STEEP * shown * steep * broken * fall * torn * sqrt(mid));
    f.z = steep * (1.0 - POOL_CALM * smoothstep(STEP_FADE, 2.0 * STEP_FADE, fract(p)) * (1.0 - smoothstep(0.75, 0.95, fract(p))));"""),
])
