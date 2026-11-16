// The bodies of the plants the marks draw (cover.wgsl): a tree's crown in the form its kind's numbers choose
// (web/src/form.ts: round, cone, column, umbrella, tiered or lobed), its stem and limbs, a bush as a low mound of leaf
// clumps with its flowers or fruit; seen from above or, angled, standing on the ground; lit from the north-west. A crown
// is a mass of leaf clumps, each a lit dome with its own leaves' bumps, the nearer ones over the farther, darker where
// they lie deep in the crown or under it; where the crown is open the limbs show between its clumps, and a crown
// without leaves is its limbs alone. A shape is worked in its crown's radii (u: east, south from its middle, both in
// the crown's half width); a form drawn less than surely (`Plant.sure`) is drawn as the round crown.

// A plant: its foot, m from the draw's corner; its crown's width, its height and its stem's width, m; how closed its
// crown is; its leaves' fineness (0 broad .. 1 fine); its colour and its walls' share; its own random numbers; its
// design (style.ts `DESIGN`); its form (form.ts `Form`) and how surely; its flowers' colour, and 1 if it is in flower
// (a bush: or in fruit); its lean (a share of its height, east and south); how far from its foot it may reach, m; and
// how dry its kind's ground, 0..1 (form.ts `Form`).
struct Plant {
  at: vec2f, d: f32, h: f32, stem: f32, closed: f32, fine: f32, rgb: vec3f, wood: f32, r: vec3f, design: f32,
  form: f32, sure: f32, petal: vec3f, blooms: f32, lean: vec2f, reach: f32, dry: f32,
}

// Paint colour rgb with coverage a over dst (premultiplied).
fn over(dst: ptr<function, vec4f>, rgb: vec3f, a: f32) { *dst = vec4f(rgb * a, a) + *dst * (1.0 - a); }

// What a plant's body shows at a point: its coverage, its light (1 on flat ground), how near its outline the point is
// (0 inside .. 1 at its edge), its tone (its colour times this: a clump's own and the shade deep in the crown) and how
// much of it is wood (its stem's and limbs' colour) rather than leaf.
struct Body { a: f32, lit: f32, rim: f32, tone: f32, wood: f32 }
fn nothing() -> Body { return Body(0.0, 1.0, 0.0, 1.0, 0.0); }

// The forms (form.ts `Form`).
const ROUND = 0; const CONE = 1; const COLUMN = 2; const UMBRELLA = 3; const TIERED = 4; const LOBED = 5;
// The share of a plant's height its crown takes, of a round crown (the live crown ratio of trees in stands, 0.4 to 0.7;
// from memory) and of each form; and each form's drawn width over its kind's, so that the forms read apart.
const CROWN_SHARE = 0.6;
fn form_depth(f: i32) -> f32 { var d = array<f32, 6>(CROWN_SHARE, 0.8, 0.85, 0.3, 0.75, 0.6); return d[f]; }
fn form_wide(f: i32) -> f32 { var w = array<f32, 6>(1.0, 0.85, 0.6, 1.2, 1.0, 1.1); return w[f]; }
// Angled, a plant stands up this share of its height on the map (textures.ts `STAND`).
const STAND = 0.5;
// A plant's leaves' fineness runs from the broadest (capture area per kg, log10 m²/kg, `LEAF_THICK`) to the finest
// (`LEAF_THIN`) (mark.wgsl `plant_of`).
const LEAF_THICK = -0.5; const LEAF_THIN = 0.7;
// A crown leans by up to its plant's lean (cover.wgsl `LEAN`); from above a lean shows as this share of it.
const LEAN_SEEN = 0.3;
// Nothing of a crown reaches past this many of its radii from its middle (a clump's leafy edge stands a little out).
const CROWN_REACH = 1.12;

// The leaf clumps of a crown: of each form, how many (and up to `CLUMPS_MORE` more by the plant's own draw), how wide
// each is in the crown's radii, and how much its leaves' size changes them (fine leaves: more and smaller clumps).
fn form_clumps(f: i32) -> f32 { var n = array<f32, 6>(8.0, 0.0, 10.0, 8.0, 9.0, 5.0); return n[f]; }
fn form_clump_r(f: i32) -> f32 { var r = array<f32, 6>(0.5, 0.0, 0.36, 0.34, 0.32, 0.6); return r[f]; }
const CLUMPS_MORE = 2.0; const FINE_MORE = 2.0; const FINE_SMALLER = 0.25;
// A clump's edge breaks into its leaves' lobes, `EDGE_LOBES` to twice as many for the finest leaves, each this share of
// its radius deep, and frays into single leaves `FRAY` of its radius deep; its face carries its sprays' bumps,
// `LEAF_CELLS` to twice as many across it, tilting its light by up to `LEAF_BUMP`, and its leaves, `LEAVES` times as
// many, which fray its edge.
const EDGE_LOBES = 9.0; const EDGE_DEEP = 0.22; const FRAY = 0.12; const LEAF_CELLS = 3.0; const LEAF_BUMP = 0.6;
const LEAVES = 3.2;
// A clump's face is the photograph of leaves (tex.wgsl `FOLIAGE_TEX`), `FOLIAGE_CELLS` times across its radius, its
// lightness shown `FOLIAGE_SHOW`; the finest leaves (`NEEDLE_FINE`) are needles.
const FOLIAGE_CELLS = 1.2; const FOLIAGE_SHOW = 0.9; const NEEDLE_FINE = 0.8;
// The light on a clump departs from flat ground's by this share of its turn to the sun (the sky lights its far side,
// and light passes through its leaves).
const LIGHT_SHARE = 0.6;
// Deep in the crown a clump is darker: its tone from `DEEP_TONE` at the crown's back to 1 at its front; angled, the
// underside of the crown darker by up to `UNDER_SHADE`; where a clump meets the one behind it a crease of shade
// `CREASE` deep; each clump's tone departs from its kind's by up to `CLUMP_TONES` either way.
const DEEP_TONE = 0.5; const UNDER_SHADE = 0.35; const CREASE = 0.3; const CLUMP_TONES = 0.14;
// Behind the clumps the crown's dark heart (`HEART` of its radius, in the deep tone) closes the gaps between them, as
// closed as the crown is (fully from `HEART_CLOSED`), broken where the crown is thinner.
const HEART = 0.62; const HEART_LIT = 0.62; const HEART_CLOSED = vec2f(0.55, 0.95);
// A clump's light turns with the crown's whole dome as much as with its own (`DOME`): the crown reads as one body.
const DOME = 0.5;
// An open crown shows fewer and smaller clumps (from `OPEN_SMALLEST` of their size; a clump shows while its own draw is
// under `OPEN_SHOWS` times the closure and `OPEN_FLOOR`), and its limbs between them.
const OPEN_SMALLEST = 0.55; const OPEN_SHOWS = 1.4; const OPEN_FLOOR = 0.25;
// The limbs: `LIMBS` to twice as many from the top of the stem to the crown's edge, each bent once and ending in a
// twig `TWIG_LONG` of it long, turned off its line; as thick as the stem at their foot (`LIMB_FOOT` of it) to `LIMB_TIP`
// of that at their tips; seen while the crown is less closed than `LIMBS_SEEN`.
const LIMBS = 3.0; const TWIG_LONG = 0.35; const LIMB_FOOT = 0.7; const LIMB_TIP = 0.25; const LIMBS_SEEN = 0.6;
// A bare crown's fine twigs: a haze of the bark's colour over the crown, `TWIG_HAZE` at most, as fine as `TWIG_CELLS`
// across it.
const TWIG_HAZE = 0.3; const TWIG_CELLS = 9.0;
// A conifer, angled: `TIERS` to `TIERS + 3` tiers of branches from its spire to its foot, each flaring from
// `TIER_NARROW` of its width at its top, its lower edge broken into branch tips `TIP_DEEP` of its width deep; the band
// under each tier in its shade (`TIER_SHADE`); its needles' streaks departing by `NEEDLE_TONE`. From above, a star of
// `CONE_BRANCHES` to twice as many branches, notched `CONE_NOTCH` of its radius between them.
const TIERS = 6.0; const TIER_NARROW = 0.5; const TIP_DEEP = 0.22; const TIER_SHADE = 0.45; const NEEDLE_TONE = 0.2;
// Each tier's width departs from the spire's line by up to `TIER_VARY`, and its edge frays into needle sprays
// `SPRAY_DEEP` of its width deep; a conifer's green is `CONIFER_TONE` of its kind's (needles are darker than leaves).
const TIER_VARY = 0.15; const SPRAY_DEEP = 0.12; const CONIFER_TONE = 0.85;
// The photograph of needles over a conifer's tiers, `NEEDLE_CELLS` times across its depth.
const NEEDLE_CELLS = 2.0;
const CONE_BRANCHES = 8.0; const CONE_NOTCH = 0.3;
// A flat crown (umbrella, tiered): its clumps lie in discs seen at a slant, `FLAT_SLANT` of their width deep on the map,
// each clump squashed to `FLAT_CLUMP` of its width; a tiered crown's `FLAT_TIERS` discs narrow upwards.
const FLAT_SLANT = 0.35; const FLAT_CLUMP = 0.6; const FLAT_TIERS = 3.0;
// The stem, angled: from its foot (flaring `FLARE` wider over its own width) to the top of its stem, tapering to
// `STEM_TOP` of its foot's width; its bark the photograph of bark.
const FLARE = 0.8; const STEM_TOP = 0.6;
// Its bark is the photograph of bark (tex.wgsl `BARK_TEX`), `BARK_ACROSS` of it across the stem and one up every
// `BARK_LONG` of the stem's width.
const BARK_ACROSS = 0.5; const BARK_LONG = 6.0;
// A bush: as deep as `BUSH_DEEP` of its width, angled; `BUSH_CLUMPS` to twice as many clumps of `BUSH_CLUMP_R` of its
// radius; its flowers or fruit, `BUSH_DOTS` to twice as many dots, each `DOT_SIZE` of its radius, on its top.
// A bush's twigs are `BUSH_TWIG` of its radius thick at their foot; wood shows at least `WOOD_LEAST` strong, however
// little of a plant is walls.
const BUSH_TWIG = 0.05; const WOOD_LEAST = 0.6;
const BUSH_DEEP = 0.45; const BUSH_CLUMPS = 5.0; const BUSH_CLUMP_R = 0.42; const BUSH_DOTS = 3.0; const DOT_SIZE = 0.13;

// The i-th of a plant's own random numbers after its three, each 0..1 (the digits of its own, turned by i).
fn own(r: vec3f, i: f32) -> vec3f { return fract(r * vec3f(7.31, 5.13, 3.77) * (1.0 + 0.43 * i) + vec3f(0.37, 0.61, 0.83) * i); }

// The light at a face of normal n (the sun from the north-west, draw.wgsl `LIGHT`: from above, up is z; angled, z faces
// the viewer, and the sun lights the crowns from their upper left).
fn lit_by_sun(n: vec3f) -> f32 { return 1.0 + LIGHT_SHARE * (max(dot(n, LIGHT), -0.3) / LIGHT.z - 1.0); }

// A leaf clump: its middle (x east, y south on the map in the crown's radii, z towards the viewer: angled, out of the
// map; from above, up), its radius, how much flatter than wide it is, and its tone.
struct Clump { c: vec3f, r: f32, flat: f32, tone: f32 }

// Clump i of n of plant q of form f, in a crown `tall` of its radii deep on the map: on the crown's near side (a
// sunflower's spiral, no rows), or in the discs of a flat crown.
fn clump_of(q: Plant, f: i32, i: f32, n: f32, tall: f32, r0: f32) -> Clump {
  let h = own(q.r, i);
  let r = r0 * (0.75 + 0.5 * h.x);
  let turn = 2.39996 * i + 2.0 * PI * q.r.y + 0.6 * (h.y - 0.5);
  if (f == UMBRELLA || f == TIERED) {
    // A disc of clumps seen at a slant: a tiered crown's discs one over another, narrower upwards.
    let tiers = select(1.0, FLAT_TIERS, f == TIERED);
    let tier = floor(h.z * tiers);
    let wide = 1.0 - 0.28 * tier;
    let rho = sqrt((i + 0.5) / n) * (wide - 0.7 * r);
    let top = select(-0.45 * tall, tall * (0.55 - 0.75 * (tier + 0.5) / tiers), f == TIERED);
    let y = top + FLAT_SLANT * tall * rho * sin(turn);
    return Clump(vec3f(rho * cos(turn), y, rho * sin(turn)), r, FLAT_CLUMP, 1.0 + CLUMP_TONES * (2.0 * h.z - 1.0));
  }
  let ring = sqrt((i + 0.5) / n);
  let x = ring * cos(turn) * (1.0 - 0.7 * r);
  let y = ring * sin(turn) * (tall - 0.7 * r);
  // On the crown's near half, the outer clumps farther back, a little out of round by its own draw.
  let z = sqrt(max(1.0 - ring * ring, 0.0)) * (1.0 - 0.5 * r) - 0.2 * h.y;
  return Clump(vec3f(x, y, z), r, 1.0, 1.0 + CLUMP_TONES * (2.0 * h.z - 1.0));
}

// The leaf mass of plant q of form f at u (in its radii), `tall` of its radii deep on the map, at `soft` radii a pixel,
// its clumps' number and size `n` and `r0`, and `heart` how much its dark heart closes it: the nearest clump the point
// sees (or the heart), lit, with its leafy edge, its crease against the clump behind and the outline where none is.
fn leaves(q: Plant, f: i32, u: vec2f, tall: f32, soft: f32, n: f32, r0: f32, heart: f32) -> Body {
  let open = 1.0 - q.closed;
  let shrink = mix(1.0, OPEN_SMALLEST, open);
  var front = -9.0;
  var best = -1.0;
  var bc = Clump(vec3f(0.0), 1.0, 1.0, 1.0);
  var bw = vec2f(0.0);
  var behind = 0.0;
  for (var i = 0.0; i < n; i += 1.0) {
    // An open crown shows the clumps whose own draw is under its closure.
    if (own(q.r, i + 17.0).x > OPEN_SHOWS * q.closed + OPEN_FLOOR) { continue; }
    var c = clump_of(q, f, i, n, tall, r0);
    c.r *= shrink;
    let w = (u - c.c.xy) / vec2f(c.r, c.r * c.flat);
    let d2 = dot(w, w);
    if (d2 >= 1.0) { continue; }
    let z = c.c.z + c.r * sqrt(1.0 - d2);
    if (z > front) {
      if (best >= 0.0) { behind = 1.0; }
      front = z; best = i; bc = c; bw = w;
    } else {
      behind = 1.0;
    }
  }
  // The heart: behind every clump, darker.
  let hv = u / vec2f(HEART, HEART * tall);
  var heart_a = heart * (1.0 - smoothstep(1.0 - soft, 1.0, length(hv)));
  if (heart_a > 0.0) { heart_a *= smoothstep(-0.3, 0.3, noise(u * 4.0 + 5.0 * q.r.xy, 16).x + 2.0 * (q.closed - HEART_CLOSED.x)); }
  if (best < 0.0) {
    if (heart_a <= 0.0) { return nothing(); }
    return Body(heart_a, HEART_LIT, 0.0, DEEP_TONE * (1.0 + 0.3 * noise(u * 9.0 + 3.0 * q.r.yz, 18).x), 0.0);
  }
  // The clump's edge in its leaves' lobes, and its coverage.
  let d = length(bw);
  let k = own(q.r, best + 31.0);
  let lobes = EDGE_LOBES * (1.0 + q.fine);
  let ang = atan2(bw.y, bw.x);
  let cs = soft / bc.r;
  let cells = LEAF_CELLS * (1.0 + q.fine);
  // Single leaves show where they take a few pixels.
  let seen = smoothstep(1.0, 3.0, 1.0 / (cs * cells * LEAVES));
  let leafy = noise(bw * (cells * LEAVES) + 29.0 * k.yz, 15);
  let edge = 1.0 - EDGE_DEEP * (0.5 + 0.5 * sin(lobes * ang + 2.0 * PI * k.x)) * (0.6 + 0.4 * sin((0.5 * lobes + 1.0) * ang + 2.0 * PI * k.y)) - FRAY * seen * (0.5 + 0.5 * leafy.x);
  var a = 1.0 - smoothstep(edge - cs, edge + 0.5 * cs, d);
  if (a <= 0.0) {
    if (heart_a <= 0.0) { return nothing(); }
    return Body(heart_a, HEART_LIT, 0.0, DEEP_TONE * (1.0 + 0.3 * noise(u * 9.0 + 3.0 * q.r.yz, 18).x), 0.0);
  }
  // Its face: a dome, tilted by its leaves' bumps (noise over its width, fewer to see the smaller it is drawn).
  let bump = (noise(bw * cells + 23.0 * k.xy, 9).yz + 0.35 * seen * leafy.yz / LEAVES) * LEAF_BUMP * smoothstep(1.0, 3.0, 1.0 / (cs * cells));
  let nz = sqrt(max(1.0 - d * d, 0.0));
  let dome_v = u / vec2f(1.0, max(tall, 1e-3));
  let dome = normalize(vec3f(dome_v, sqrt(max(1.0 - dot(dome_v, dome_v), 0.05))));
  let nrm = normalize(mix(dome, normalize(vec3f(bw + bump * nz, nz)), 1.0 - DOME));
  let lit = lit_by_sun(nrm);
  // Its tone: its own, deeper in the crown darker, under the crown darker, and in the crease against the clump behind.
  let depth = clamp(0.5 * (front + 1.0), 0.0, 1.0);
  var tone = bc.tone * mix(DEEP_TONE, 1.0, depth) * (1.0 - UNDER_SHADE * smoothstep(-0.2, 1.0, u.y / max(tall, 1e-3)) * cover.corner.w);
  tone *= 1.0 - CREASE * behind * smoothstep(0.7, 1.0, d / edge);
  // Its leaves: the photograph of leaves close together over its face (needles for the finest), its lightness about its
  // mean, `FOLIAGE_CELLS` times across a clump's radius.
  let cells_on = FOLIAGE_CELLS * (1.0 + q.fine);
  tone *= mix(1.0, photo_light(select(FOLIAGE_TEX, NEEDLES_TEX, q.fine > NEEDLE_FINE), bw * cells_on + 9.0 * k.xy, cs * cells_on), FOLIAGE_SHOW);
  // A flat clump's underside lies in its own shade.
  tone *= 1.0 - (1.0 - bc.flat) * smoothstep(0.0, 0.8, bw.y);
  // The outline only where nothing stands behind it (its silhouette), else the clump lies over the heart's dark.
  let rim_at = (1.0 - behind) * (1.0 - heart_a) * smoothstep(edge - 2.0 * cs, edge, d);
  a = max(a, heart_a);
  return Body(a, lit, rim_at, tone, 0.0);
}

// The distance from u to the segment from s to t, and how far along it the nearest point lies (0..1).
fn to_segment(u: vec2f, s: vec2f, t: vec2f) -> vec2f {
  let st = t - s;
  let k = clamp(dot(u - s, st) / max(dot(st, st), 1e-6), 0.0, 1.0);
  return vec2f(length(u - s - st * k), k);
}

// The limbs of plant q at u (in its crown's radii, `tall` deep on the map), its stem `w` radii thick, at `soft` radii a
// pixel: from the top of its stem (angled, the crown's foot; from above, its middle) to its crown's edge, bent once and
// forked into two twigs at the end, thinner towards their tips; their coverage and how far across a limb (-1..1).
fn limbs(q: Plant, u: vec2f, tall: f32, w: f32, soft: f32) -> vec2f {
  let angled = cover.corner.w;
  let foot = vec2f(0.0, angled * 0.8 * tall);
  let n = LIMBS + floor(LIMBS * q.r.x);
  var best = vec2f(0.0);
  for (var i = 0.0; i < n; i += 1.0) {
    let h = own(q.r, i + 50.0);
    // Angled, the limbs fan upwards across the crown; from above, they spread all round.
    let x = mix(2.0 * (i + 0.5) / n - 1.0, cos(2.0 * PI * (i + h.x) / n), 1.0 - angled) * (0.75 + 0.2 * h.y);
    let y = mix(sin(2.0 * PI * (i + h.x) / n) * (0.75 + 0.2 * h.y), -tall * (0.25 + 0.6 * (1.0 - abs(x))) + 0.2 * tall * (h.z - 0.5), angled);
    let tip = vec2f(x, y);
    let bend = foot + 0.5 * (tip - foot) + 0.15 * vec2f(h.z - 0.5, h.x - 0.5);
    let dir = normalize(tip - bend);
    let side = vec2f(-dir.y, dir.x);
    let twig = TWIG_LONG * length(tip - foot);
    let tip2 = tip + twig * normalize(dir + select(-0.6, 0.6, h.y > 0.5) * side);
    // Skip a limb whose box the point lies outside.
    let lo = min(min(foot, bend), min(tip, tip2)) - vec2f(w + soft);
    let hi = max(max(foot, bend), max(tip, tip2)) + vec2f(w + soft);
    if (any(u < lo) || any(u > hi)) { continue; }
    var ends = array<vec2f, 3>(foot, bend, tip);
    var tips = array<vec2f, 3>(bend, tip, tip2);
    var thick_from = array<f32, 3>(1.0, 0.7, LIMB_TIP);
    var thick_to = array<f32, 3>(0.7, LIMB_TIP, 0.08);
    for (var j = 0; j < 3; j++) {
      let s = to_segment(u, ends[j], tips[j]);
      let thick = max(LIMB_FOOT * w * mix(thick_from[j], thick_to[j], s.y), 0.5 * soft);
      let a = 1.0 - smoothstep(thick - soft, thick + 0.5 * soft, s.x);
      if (a > best.x) { best = vec2f(a, clamp(s.x / thick, -1.0, 1.0)); }
    }
  }
  return best;
}

// A conifer's crown at u (in its radii, `tall` deep on the map), at `soft` radii a pixel: angled, tiers of branches from
// its spire to its foot, each flaring to its lower edge of branch tips, the band under each in its shade; from above, a
// star of branches.
fn conifer(q: Plant, u: vec2f, tall: f32, soft: f32) -> Body {
  let angled = cover.corner.w;
  if (angled > 0.5) {
    let t = (u.y + tall) / (1.9 * tall);
    if (t < 0.0 || t > 1.0) { return nothing(); }
    let tiers = TIERS + floor(4.0 * q.r.x);
    let f = fract(t * tiers);
    let k = floor(t * tiers);
    let kh = own(q.r, k + 70.0);
    let wide = (0.08 + 0.92 * pow(t, 0.9)) * mix(TIER_NARROW, 1.0, pow(f, 0.6)) * (1.0 + TIER_VARY * (2.0 * kh.x - 1.0));
    let w0 = max(wide, 0.05);
    // The tier's lower edge breaks into branch tips, and every edge into needle sprays.
    let tips = 1.0 - TIP_DEEP * f * (0.5 + 0.5 * cos(7.0 * u.x / w0 + 2.0 * PI * kh.z));
    let spray = noise(vec2f(u.x * 14.0 / max(tall, 0.3), t * tiers * 3.0) + 31.0 * q.r.xy, 17).x;
    let edge = wide * tips * (1.0 - SPRAY_DEEP * (0.5 + 0.5 * spray));
    let a = 1.0 - smoothstep(edge - soft, edge + 0.5 * soft, abs(u.x));
    if (a <= 0.0) { return nothing(); }
    let across = u.x / max(edge, 1e-3);
    let nrm = normalize(vec3f(0.9 * across, -0.5 + 0.7 * f, sqrt(max(1.0 - across * across, 0.05))));
    // Needles in streaks drooping outwards from the stem, dark between the sprays.
    let needles = noise(vec2f(abs(u.x) * 22.0 / max(tall, 0.3) - 5.0 * f, 6.0 * t * tiers) + 17.0 * q.r.xy, 11).x;
    let photo_uv = vec2f(abs(u.x) * NEEDLE_CELLS / max(tall, 0.3), t * tiers * 0.5);
    let tone = CONIFER_TONE * mix(TIER_SHADE, 1.0, smoothstep(0.0, 0.45, f)) * (1.0 + NEEDLE_TONE * needles) * mix(0.75, 1.0, t) * mix(1.0, photo_light(NEEDLES_TEX, photo_uv, soft * NEEDLE_CELLS / max(tall, 0.3)), FOLIAGE_SHOW);
    let rim_at = smoothstep(edge - 2.0 * soft, edge, abs(u.x));
    return Body(a, lit_by_sun(nrm), rim_at, tone, 0.0);
  }
  let rho = length(u);
  if (rho > 1.0 + soft) { return nothing(); }
  let ang = atan2(u.y, u.x);
  let n = CONE_BRANCHES * (1.0 + floor(2.0 * q.r.x) * 0.5);
  let spoke = abs(cos(0.5 * n * ang + 2.0 * PI * q.r.y));
  let edge = 1.0 - CONE_NOTCH * (1.0 - spoke * spoke);
  let a = 1.0 - smoothstep(edge - soft, edge + 0.5 * soft, rho);
  if (a <= 0.0) { return nothing(); }
  let slope = 1.2;
  let nrm = normalize(vec3f(u / max(rho, 1e-3) * slope * rho, 1.0));
  let tone = (0.75 + 0.25 * spoke) * mix(0.75, 1.0, rho) * (1.0 + NEEDLE_TONE * noise(vec2f(ang * 6.0, rho * 8.0) + 9.0 * q.r.xy, 12).x);
  return Body(a, lit_by_sun(nrm), smoothstep(edge - 2.0 * soft, edge, rho), tone, 0.0);
}

// Over leaves `b` at u (in a crown's radii, `tall` deep on the map, at `soft` radii a pixel), the wood of plant q where
// its leaves leave it showing: its limbs (`limbs`, from a stem `w` radii thick) and a bare crown's haze of twigs.
fn bare_wood(q: Plant, u: vec2f, tall: f32, w: f32, soft: f32, b: Body) -> Body {
  if (q.closed >= LIMBS_SEEN || b.a >= 0.9) { return b; }
  let limb = limbs(q, u, tall, w, soft);
  let haze_v = u / vec2f(0.9, 0.9 * tall);
  let haze = TWIG_HAZE * (1.0 - q.closed) * (1.0 - smoothstep(0.6, 1.0, length(haze_v))) * smoothstep(0.1, 0.6, noise(u * TWIG_CELLS + 13.0 * q.r.xy, 13).x + 0.4);
  let wa = max(limb.x, haze) * max(q.wood, WOOD_LEAST);
  if (wa <= b.a) { return b; }
  let lit = lit_by_sun(normalize(vec3f(limb.y, 0.0, sqrt(max(1.0 - limb.y * limb.y, 0.0)))));
  return Body(wa, lit, 0.0, select(0.8, 1.0, limb.x >= haze), 1.0);
}

// The crown of tree q at p seen from above or, angled, lifted on its stem, at e m a pixel: its leaves over its limbs.
fn crown(q: Plant, p: vec2f, e: f32) -> Body {
  let angled = cover.corner.w;
  let f = select(ROUND, i32(q.form + 0.5), q.sure > 0.5);
  let depth = mix(CROWN_SHARE, form_depth(f), q.sure) * q.h;
  let lean = q.lean * q.h * mix(LEAN_SEEN, STAND, angled);
  let half = min(0.5 * q.d * mix(1.0, form_wide(f), q.sure), (q.reach - length(lean)) / CROWN_REACH);
  // Its middle lifted when angled to the middle of its crown, and its depth on the map.
  let up = angled * STAND * (q.h - 0.5 * depth);
  let tall = mix(1.0, max(STAND * depth / max(2.0 * half, 1e-3), 0.3), angled);
  let u = (p - q.at - lean + vec2f(0.0, up)) / half;
  let soft = e / max(half, 1e-4);
  if (abs(u.x) > CROWN_REACH + soft || abs(u.y) > CROWN_REACH * tall + soft) { return nothing(); }
  var b = nothing();
  if (f == CONE) {
    b = conifer(q, u, tall, soft);
  } else {
    let n = form_clumps(f) + floor(CLUMPS_MORE * q.r.z) + floor(FINE_MORE * q.fine);
    let r0 = form_clump_r(f) * (1.0 - FINE_SMALLER * q.fine);
    let heart = select(1.0, 0.0, f == UMBRELLA || f == TIERED) * smoothstep(HEART_CLOSED.x, HEART_CLOSED.y, q.closed);
    b = leaves(q, f, u, tall, soft, n, r0, heart);
  }
  if (f != CONE) { b = bare_wood(q, u, tall, max(0.5 * q.stem / half, 0.6 * soft), soft, b); }
  return b;
}

// The stem of tree q at p, angled, at e m a pixel: from its foot, flared, to the middle of its crown, tapering, leaning
// with it, lit on its west side, its bark in streaks.
fn stem(q: Plant, p: vec2f, e: f32) -> Body {
  let s = p - q.at;
  let top = STAND * q.h * (1.0 - 0.5 * CROWN_SHARE);
  if (s.y > e || s.y < -top) { return nothing(); }
  let rise = clamp(-s.y / max(top, 1e-3), 0.0, 1.0);
  let w0 = max(0.5 * q.stem, 0.35 * e);
  let w = max(w0 * mix(1.0, STEM_TOP, rise) * (1.0 + FLARE * exp(-(-s.y) / max(w0, 1e-4))), 0.35 * e);
  let x = (s.x - q.lean.x * q.h * STAND * rise) / w;
  let a = (1.0 - smoothstep(1.0 - e / w, 1.0, abs(x))) * (1.0 - smoothstep(-e, e, s.y));
  if (a <= 0.0) { return nothing(); }
  let nrm = normalize(vec3f(x, 0.0, sqrt(max(1.0 - x * x, 0.0))));
  let bark = photo_light(BARK_TEX, vec2f(BARK_ACROSS * x + q.r.x, -s.y / max(BARK_LONG * w0, 1e-3)), e / max(w0, 1e-4));
  return Body(a, lit_by_sun(nrm), smoothstep(1.0 - 2.0 * e / w, 1.0, abs(x)), bark, 1.0);
}

// The body of bush q at p, at e m a pixel: a low mound of leaf clumps; angled, squat on the ground.
fn bush(q: Plant, p: vec2f, e: f32) -> Body {
  let angled = cover.corner.w;
  let half = min(0.5 * q.d, q.reach / CROWN_REACH);
  let tall = mix(1.0, BUSH_DEEP, angled);
  let u = (p - q.at + vec2f(0.0, angled * tall * half)) / half;
  let soft = e / max(half, 1e-4);
  if (abs(u.x) > CROWN_REACH + soft || abs(u.y) > CROWN_REACH * tall + soft) { return nothing(); }
  let n = BUSH_CLUMPS + floor(BUSH_CLUMPS * q.r.z);
  let b = leaves(q, ROUND, u, tall, soft, n, BUSH_CLUMP_R * (1.0 - FINE_SMALLER * q.fine), smoothstep(HEART_CLOSED.x, HEART_CLOSED.y, q.closed));
  // A bush without its leaves is its twigs.
  return bare_wood(q, u, tall, max(BUSH_TWIG, 0.6 * soft), soft, b);
}

// The flowers or fruit of bush q at p, at e m a pixel: dots over its top, each over dst with a lighter heart.
fn bush_dots(dst: ptr<function, vec4f>, q: Plant, p: vec2f, e: f32, fade: f32) {
  let half = min(0.5 * q.d, q.reach / CROWN_REACH);
  if (length(p - q.at) > 2.0 * half) { return; }
  let count = BUSH_DOTS * (1.0 + q.r.z);
  let radius = max(DOT_SIZE * half, 0.6 * e);
  let lift = vec2f(0.0, cover.corner.w * BUSH_DEEP * half);
  for (var i = 0.0; i < count; i += 1.0) {
    let h = fract(q.r.zxy * (5.0 + 2.0 * i) + 0.61 * i);
    let at = q.at - lift + 0.7 * half * sqrt(h.x) * vec2f(cos(2.0 * PI * h.y), sin(2.0 * PI * h.y) * mix(1.0, BUSH_DEEP, cover.corner.w));
    let d = length(p - at) / radius;
    let a = 1.0 - smoothstep(1.0 - e / radius, 1.0, d);
    if (a <= 0.0) { continue; }
    over(dst, mark_colour(q.petal, 1.2 - 0.4 * d, rim(d, 1.0, radius, e)), a * fade);
  }
}
