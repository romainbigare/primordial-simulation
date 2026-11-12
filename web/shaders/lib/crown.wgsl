// The bodies of the plants the marks draw (cover.wgsl): a tree's crown in the form its kind's numbers choose
// (web/src/form.ts: round, cone, column, umbrella, tiered or lobed), a bush as a low knot of lobes with its flowers or
// fruit; seen from above or, angled, standing on the ground; lit from the north-west. A shape is worked in its own radii
// (v: east, south from its centre); a form drawn less than surely (`Plant.sure`) lies that much nearer the round crown.

// A plant: its foot, m from the draw's corner; its crown's width, its height and its stem's width, m; how closed its
// crown is; its leaves' fineness (0 broad .. 1 fine); its colour and its walls' share; its own random numbers; its
// design (style.ts `DESIGN`); its form (form.ts `Form`) and how surely; its flowers' colour, and 1 if it is in flower
// (a bush: or in fruit); its lean (a share of its height, east and south); and how far from its foot it may reach, m.
struct Plant {
  at: vec2f, d: f32, h: f32, stem: f32, closed: f32, fine: f32, rgb: vec3f, wood: f32, r: vec3f, design: f32,
  form: f32, sure: f32, petal: vec3f, blooms: f32, lean: vec2f, reach: f32,
}

// Paint colour rgb with coverage a over dst (premultiplied).
fn over(dst: ptr<function, vec4f>, rgb: vec3f, a: f32) { *dst = vec4f(rgb * a, a) + *dst * (1.0 - a); }

// What a plant's body shows at a point: its coverage, its light (1 on flat ground), and how near its rim the point is
// (0 inside .. 1 at its edge).
struct Body { a: f32, lit: f32, rim: f32 }

// The forms (form.ts `Form`).
const CONE = 1; const COLUMN = 2; const UMBRELLA = 3; const TIERED = 4; const LOBED = 5;
// The share of a plant's height its crown takes, of a round crown (the live crown ratio of trees in stands, 0.4 to 0.7;
// from memory) and of each form; and each form's drawn width over its kind's, so that the forms read apart.
const CROWN_SHARE = 0.6;
fn form_depth(f: i32) -> f32 { var d = array<f32, 6>(CROWN_SHARE, 0.8, 0.85, 0.3, 0.75, 0.6); return d[f]; }
fn form_wide(f: i32) -> f32 { var w = array<f32, 6>(1.0, 0.85, 0.6, 1.2, 1.0, 1.1); return w[f]; }
// Angled, a plant stands up this share of its height on the map (textures.ts `STAND`).
const STAND = 0.5;
// Seen from above a crown's edge draws in lobes, as many as `LOBES_FINE` for the smallest leaves and `LOBES_BROAD` for
// the broadest (capture area per kg, log10 m²/kg, from `LEAF_THICK` to `LEAF_THIN`), each this share of its radius deep;
// its leaves' clumps (three waves at 120 degrees: no rows) are as many across its width as its lobes, their light
// departing by up to `CLUMP_TONE` either way; where it is open the ground shows between its `TWIGS` branches, curling
// by `TWIG_CURL` radians a radius and spaced unevenly.
const LOBES_FINE = 13.0; const LOBES_BROAD = 6.0; const LEAF_THICK = -0.5; const LEAF_THIN = 0.7; const LOBE_DEEP = 0.18;
const CLUMP_TONE = 0.18; const TWIGS = 17.0; const TWIG_CURL = 2.0;
// A crown is a dome (profile power 2) or a cone (1) seen from above, lit from the north-west, its slope read no steeper
// than `STEEPEST`, and its light departs from the flat ground's by `LIGHT_SHARE` (the sky lights its far side, and light
// passes through its leaves). Each form's profile power: a cone's and a tiered crown's pointed, an umbrella's flat.
const STEEPEST = 1.5; const LIGHT_SHARE = 0.55;
fn form_power(f: i32) -> f32 { var b = array<f32, 6>(2.0, 1.0, 2.0, 6.0, 1.2, 2.0); return b[f]; }
// A cone: from above a star of `CONE_BRANCHES` to twice as many branches, notched between them `CONE_NOTCH` of its
// radius (their tips the sharper the greater `CONE_SHARP`), its whorls darker rings `WHORLS` across its radius; angled,
// it narrows to its top, its side in steps of whorls.
const CONE_BRANCHES = 8.0; const CONE_NOTCH = 0.3; const CONE_SHARP = 2.0; const WHORLS = 3.0; const WHORL_DARK = 0.12;
// A tiered crown: `TIERS` layers, each casting a dark band `TIER_DARK` deep on the one below; from above its edge in
// `TIER_LOBES` shallow lobes; angled, each layer narrower than the one below.
const TIERS = 3.0; const TIER_DARK = 0.22; const TIER_LOBES = 7.0;
// An umbrella: from above `PADS` broad pads `PAD_DEEP` of its radius deep; angled, flat beneath from `UNDER` of its
// half height down.
const PADS = 5.0; const PAD_DEEP = 0.25; const UNDER = 0.35;
// A lobed crown and a bush: `KNOTS` to `KNOTS + 2` round knots `KNOT_OUT` of its radius from its middle, each
// `KNOT_R` of its radius (up to `KNOT_SIZES` more), with shallow lobes of their own.
const KNOTS = 3.0; const KNOT_OUT = 0.5; const KNOT_R = 0.5; const KNOT_SIZES = 0.15;
// No knot reaches past its middle's distance and its radius, in radii of its crown.
const KNOTS_REACH = KNOT_OUT + KNOT_R + KNOT_SIZES;
// A crown leans by up to its plant's lean (cover.wgsl `LEAN`); from above a lean shows as this share of it.
const LEAN_SEEN = 0.3;
// A bush: as deep as `BUSH_DEEP` of its width, angled; its flowers or fruit, `BUSH_DOTS` to twice as many dots, each
// `DOT_SIZE` of its radius, on its top.
const BUSH_DEEP = 0.45; const BUSH_DOTS = 3.0; const DOT_SIZE = 0.13;

// The edge (in radii) of a shape of `n` lobes `deep` of its radius deep at angle `ang`, phases from `r`.
fn lobed_edge(ang: f32, n: f32, deep: f32, r: vec3f) -> f32 {
  return 1.0 - deep * (0.5 + 0.5 * sin(n * ang + 2.0 * PI * r.y)) * (0.55 + 0.45 * sin((0.5 * n + 1.0) * ang + 2.0 * PI * r.z));
}

// The light from above on a crown of profile power `beta`, `k` radii deep, at v (in its radii), and on its knots' clumps.
fn dome_lit(v: vec2f, beta: f32, k: f32) -> f32 {
  let r = clamp(length(v), 1e-3, 0.999);
  let slope = min(STEEPEST, k * pow(r, beta - 1.0) * pow(1.0 - pow(r, beta), 1.0 / beta - 1.0));
  let normal = normalize(vec3f(v / r * slope, 1.0));
  return 1.0 + LIGHT_SHARE * (dot(normal, LIGHT) / LIGHT.z - 1.0);
}

// The light of an angled crown at v: lit on its north-west side.
fn side_lit(v: vec2f) -> f32 { return 1.0 - LIGHT_SHARE * (0.35 * v.x + 0.15 * v.y); }

// The knots of a lobed crown or a bush at v, `sure` of the way from one round crown, at `soft` radii a pixel: its
// coverage, light (`k` radii deep) and rim.
fn knots(q: Plant, v: vec2f, soft: f32, sure: f32, k: f32) -> Body {
  let n = KNOTS + floor(3.0 * q.r.x);
  var best = Body(0.0, 1.0, 0.0);
  for (var i = 0.0; i < n; i += 1.0) {
    let h = fract(q.r * (7.0 + 3.0 * i) + 0.37 * i);
    let turn = 2.0 * PI * (i / n + q.r.y) + 1.2 * (h.x - 0.5);
    let w = (v - sure * KNOT_OUT * vec2f(cos(turn), sin(turn))) / mix(1.0, KNOT_R + KNOT_SIZES * h.z, sure);
    let rho = length(w);
    let edge = lobed_edge(atan2(w.y, w.x), LOBES_BROAD, 0.5 * LOBE_DEEP, h);
    let a = 1.0 - smoothstep(edge - soft, edge + 0.5 * soft, rho);
    if (a > best.a) { best = Body(a, dome_lit(w, 2.0, k), smoothstep(edge - 2.0 * soft, edge, rho)); }
  }
  return best;
}

// The crown of tree q at p seen from above or, angled, lifted on its stem, at e m a pixel.
fn crown(q: Plant, p: vec2f, e: f32) -> Body {
  let angled = cover.corner.w;
  let f = i32(q.form + 0.5);
  let s = q.sure;
  let depth = mix(CROWN_SHARE, form_depth(f), s) * q.h;
  let lean = q.lean * q.h * mix(LEAN_SEEN, STAND, angled);
  // Its radius, within the reach of its cell's marks (a lobed crown's knots reach a tenth past it).
  let half = min(0.5 * q.d * mix(1.0, form_wide(f), s), (q.reach - length(lean)) / 1.1);
  // Its centre lifted when angled to the middle of its crown, its shape squeezed upright by its depth.
  let up = angled * STAND * (q.h - 0.5 * depth);
  let tall = mix(1.0, max(STAND * depth / max(2.0 * half, 1e-3), 0.3), angled);
  var v = (p - q.at - lean + vec2f(0.0, up)) / vec2f(half, half * tall);
  let soft = e / max(half, 1e-4);
  // No form reaches past its knots' reach (an angled taper only narrows it): the many pixels a mark does not cover
  // read nothing more.
  if (length(v) > KNOTS_REACH + soft) { return Body(0.0, 1.0, 0.0); }
  let k = depth / half;
  if (f == LOBED) {
    var b = knots(q, v, soft, s, k);
    b.lit = mix(b.lit, side_lit(v), angled);
    return b;
  }
  // Angled: its outline narrows towards its top as its form does.
  let y01 = clamp(0.5 * (1.0 + v.y), 0.0, 1.0);
  var taper = 1.0;
  var band = 1.0;
  if (f == CONE) { taper = clamp(y01 + 0.12, 0.12, 1.0) * (0.82 + 0.18 * fract(y01 * 2.0 * WHORLS)); }
  if (f == TIERED) {
    let t = floor(y01 * TIERS);
    taper = (0.4 + 0.6 * (t + 1.0) / TIERS) * (0.5 + 0.5 * fract(y01 * TIERS));
    band = 1.0 - TIER_DARK * (1.0 - smoothstep(0.0, 0.2, fract(y01 * TIERS)));
  }
  v.x /= mix(1.0, mix(1.0, taper, s), angled);
  let rho = length(v);
  // No edge lies past its radius.
  if (rho > 1.0 + soft) { return Body(0.0, 1.0, 0.0); }
  let ang = atan2(v.y, v.x);
  // From above: its edge in lobes as broad as its leaves, or its form's.
  var edge = lobed_edge(ang, mix(LOBES_BROAD, LOBES_FINE, q.fine), LOBE_DEEP, q.r);
  var form_edge = edge;
  if (f == CONE) {
    let n = CONE_BRANCHES * (1.0 + floor(2.0 * q.r.x) * 0.5);
    form_edge = 1.0 - CONE_NOTCH * (1.0 - pow(abs(cos(0.5 * n * ang + 2.0 * PI * q.r.y)), CONE_SHARP));
  } else if (f == UMBRELLA) {
    form_edge = lobed_edge(ang, PADS, PAD_DEEP, q.r);
  } else if (f == TIERED) {
    form_edge = lobed_edge(ang, TIER_LOBES, 0.5 * LOBE_DEEP, q.r);
  } else if (f == COLUMN) {
    form_edge = lobed_edge(ang, LOBES_FINE, 0.5 * LOBE_DEEP, q.r);
  }
  edge = mix(edge, mix(form_edge, 1.0, angled * 0.5), s);
  var a = 1.0 - smoothstep(edge - soft, edge + soft * 0.5, rho);
  if (f == UMBRELLA) { a *= 1.0 - angled * s * smoothstep(UNDER - soft, UNDER + soft, v.y); }
  if (a <= 0.0) { return Body(0.0, 1.0, 0.0); }
  // Its leaves' clumps: the ground shows between them as far as the crown is open; their light varies.
  let wave = 0.5 * PI * mix(LOBES_BROAD, LOBES_FINE, q.fine);
  let clump = (3.0 + cos(wave * v.x + 6.0 * q.r.x) + cos(wave * dot(v, vec2f(-0.5, 0.866)) + 6.0 * q.r.y) + cos(wave * dot(v, vec2f(-0.5, -0.866)) + 6.0 * q.r.z)) / 6.0;
  // Open, the ground shows between its branches, which spread from its stem and curl a little.
  let twigs = 0.5 + 0.5 * cos(TWIGS * ang + TWIG_CURL * rho + 2.0 * PI * q.r.x + 1.5 * sin(5.0 * ang + 6.0 * q.r.z));
  a *= mix(1.0, smoothstep(0.0, 0.25, twigs - (1.0 - q.closed)), smoothstep(0.0, 1.0, (1.0 - q.closed) * 4.0));
  // Its light: from above by its profile, with its whorls' or tiers' rings; angled, lit on its north-west side.
  var lit = dome_lit(v, mix(2.0, form_power(f), s), k);
  if (f == CONE) { lit *= 1.0 - s * WHORL_DARK * (1.0 - smoothstep(0.0, 0.3, fract(rho * WHORLS + q.r.z))); }
  if (f == TIERED) { lit *= 1.0 - s * TIER_DARK * (1.0 - smoothstep(0.0, 0.25, fract(rho * TIERS + 0.5 * q.r.z))); }
  lit = mix(lit, side_lit(v) * mix(1.0, band, s), angled) * (1.0 + CLUMP_TONE * (2.0 * clump - 1.0));
  return Body(a, lit, rim(rho, edge, half, e));
}

// The body of bush q at p, at e m a pixel: a low knot of lobes; angled, squat on the ground.
fn bush(q: Plant, p: vec2f, e: f32) -> Body {
  let angled = cover.corner.w;
  let half = min(0.5 * q.d, q.reach / 1.1);
  let tall = mix(1.0, BUSH_DEEP, angled);
  let v = (p - q.at + vec2f(0.0, angled * tall * half)) / vec2f(half, half * tall);
  let soft = e / max(half, 1e-4);
  if (length(v) > KNOTS_REACH + soft) { return Body(0.0, 1.0, 0.0); }
  var b = knots(q, v, soft, 1.0, 2.0 * BUSH_DEEP);
  b.lit = mix(b.lit, side_lit(v), angled);
  return b;
}

// The flowers or fruit of bush q at p, at e m a pixel: dots over its top, each over dst with a lighter heart.
fn bush_dots(dst: ptr<function, vec4f>, q: Plant, p: vec2f, e: f32, fade: f32) {
  let half = min(0.5 * q.d, q.reach / 1.1);
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
