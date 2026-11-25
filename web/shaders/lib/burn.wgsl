// Burn scars and cured grass on the live map's dry ground (land.wgsl), from what fire did on each square (fire.wgsl):
// in the patches of a square's scar, the ground is black with char and grey with ash where the flames left it, the char
// fades to the scorched brown of bare burnt soil and that to the ground's own colour as the months pass, under the
// plants that come back by the core's own numbers (the cover's field, drawn as it is). The Earth view draws the cured
// leaves standing as straw over the cover.

// Reflectances, linear (char: fire.wgsl `CHAR`): ash, grey to white where the flames burnt all (white ash 30 to 50 %,
// grey ash about 15 %: Smith and Hudak 2005, Bodí and others 2014); scorched soil, the reddish brown of mineral soil
// heated and bared (Certini 2005).
const ASH = vec3f(0.36, 0.35, 0.33);
const SCORCHED = vec3f(0.20, 0.12, 0.07);
// Cured leaves standing cover all but e^-1 of a herb layer at this much, kg/m² (a dry grass sward holds a few tenths
// of a kg/m² of standing dead leaf, from memory); the Earth view tints the low plants' cover towards straw by that share
// (`STRAW`, the pale yellow of dry grass in the style).
const CURED_HIDES_KG = 0.15; const STRAW = vec3f(0.62, 0.50, 0.22);
// The char's photograph (fire.jpg's second layer, burnt ground: char, ash, needles and stalks left) is this many metres
// across; up close it gives the char its own colours about the char's (its colour over its mean, squared: the photograph
// is of a weathered burn, a fresh one's grey ash and pale stalks stand out of blacker char). A fire leaves `UNBURNT` of
// the ground it runs over only scorched, in ragged spots from `UNBURNT_M` across (a burn's patchiness down to the
// metre: Key and Benson 2006, read).
const CHAR_PHOTO_M = 1.0; const UNBURNT = 0.07; const UNBURNT_M = 0.6; const UNBURNT_OCTAVES = 2;

// Far away, where they are not drawn one by one (cover.wgsl, snag.wgsl), the Earth view draws the dead trees standing
// on a patch as specks of their bark and shadow (cover_field.wgsl `specks`), as many as their walls make (fire.wgsl
// `snag_kg`), each as wide as the ground one covers (fire.wgsl `dead_seen_w`).
const DEAD_BARK = vec3f(0.035, 0.032, 0.03);

// Spot p with the marks of fire on it: where it lies dry, the share of it under a scar (`Spot.scar.x`), how much of it
// is char, ash and scorched soil (`.yzw`); in the Earth view its cover tinted by the cured leaves standing in it, under
// the dead trees of its patch seen from afar (`dead_far`).
fn burnt_spot(p: Spot) -> Spot { return dead_far(scarred(p)); }

// The dead trees standing at spot p as its field is read (cover_field.wgsl `dead_here`: the rest of the cells' and its
// patch's over the share of it the patch takes) seen from afar, in the Earth view, where it lies dry: specks as many as
// their walls make, those of a burn spread over its cell on the land it burnt (fire.wgsl `dead_on_scar`), as far as the
// marks do not draw them one by one.
fn dead_far(p: Spot) -> Spot {
  var q = p;
  if (p.depth > 0.0 || !styled()) { return q; }
  let dead = dead_here();
  if (dead.x <= 0.0 || dead.y <= 0.0) { return q; }
  // (Drawn one by one as they take pixels enough, by the test the marks take them by: mark.wgsl `dead_decided`.)
  let w = dead_seen_w(dead.y);
  let near = cover_look.shown[STANDS] * as_crown(w, p.e);
  if (near >= 1.0) { return q; }
  let on_scar = select(1.0, dead_on_scar(fire_at(p.at), dead.z, p.scar.x), cell_patch == 0 && fire.map.x > 0.0);
  let seen = specks(p.at, w, on_scar * dead.x / snag_kg(dead.y) * 0.25 * PI * w * w, p.e, 89);
  let share = seen.x * (1.0 - near);
  q.cover = vec4f(DEAD_BARK * (1.0 + DOME_LIT * seen.y) * share + q.cover.rgb * (1.0 - share), share + q.cover.a * (1.0 - share));
  return q;
}

// Spot p with the scar and the cured leaves of fire on it (`burnt_spot`).
fn scarred(p: Spot) -> Spot {
  var q = p;
  // (Squares of side 0: the page knows no fire, or draws none, fire.ts `groundBinds`.)
  if (p.depth > 0.0 || fire.map.x <= 0.0) { return q; }
  let f = fire_at(p.at);
  if (styled() && f.cured > 0.0) {
    // (The cured leaves stand in the low plants: the share of the cover that is theirs takes the tint.)
    let low = min(p.low / max(p.cover.a, 1e-3), 1.0);
    q.cover = vec4f(mix(q.cover.rgb, STRAW * q.cover.a, low * -expm1(-f.cured / CURED_HIDES_KG)), q.cover.a);
  }
  let b = burn_at(p.at, p.e);
  q.scar = scar_of(f, b, p.at, p.e);
  // The Earth view draws the plants on a scar scorched and with their dead leaves, and burning as a front passes (as
  // the marks drawn one by one, cover.wgsl).
  if (styled()) { q.cover = vec4f(burnt_plant(q.cover.rgb, q.cover.a, field_fire(plant_fire(f, q.scar, b))), q.cover.a); }
  return q;
}

// The scar's colour over ground colour `c` at spot p: its scorched soil, its char, its ash (`burnt_spot`), each over
// the last; up close the char in the photograph's colours and its unburnt spots scorched alone.
fn scar_over(c: vec3f, p: Spot) -> vec3f {
  let s = p.scar;
  if (s.x <= 0.0) { return c; }
  let near = shows_at(CHAR_PHOTO_M, p.e);
  var char_rgb = CHAR;
  var char_share = s.y;
  if (near > 0.0) {
    let g = p.e / CHAR_PHOTO_M;
    let photo_rgb = photo(FIRE_FROM + 1, p.at / CHAR_PHOTO_M, vec2f(g, 0.0), vec2f(0.0, g)) / max(photo_mean(FIRE_FROM + 1), vec3f(1e-3));
    char_rgb = CHAR * mix(vec3f(1.0), photo_rgb * photo_rgb, near);
    let spots = smoothstep(-EVEN_ONE, EVEN_ONE, ragged(p.at, p.e, UNBURNT_M, UNBURNT_OCTAVES, 93));
    char_share *= 1.0 - near * shows_at(UNBURNT_M, p.e) * (1.0 - smoothstep(UNBURNT - 0.05, UNBURNT + 0.05, spots));
  }
  var rgb = mix(c, SCORCHED, s.w);
  rgb = mix(rgb, char_rgb, char_share);
  rgb = mix(rgb, ASH, s.z);
  return mix(c, rgb, s.x);
}

// The Earth view's floor `c` (colour.wgsl `ground_of`) under the scar; and the share of it the scar takes from the
// soil's own tint.
fn burnt_floor(c: vec3f, p: Spot) -> vec4f { return vec4f(scar_over(c, p), p.scar.x); }
