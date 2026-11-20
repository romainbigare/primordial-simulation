// Burn scars and cured grass on the live map's dry ground (burnt.wgsl), from what fire did on each square (fire.wgsl):
// in the patches of a square's scar, the ground is black with char and grey with ash where the flames left it, the char
// fades to the scorched brown of bare burnt soil and that to the ground's own colour as the months pass, under the
// plants that come back by the core's own numbers (the cover's field, drawn as it is). The Earth view draws the cured
// leaves standing as straw over the cover; the true view shows the ground's reflectance.

// Reflectances, linear: char, the black of a fresh burn (charred litter and soil reflect 3 to 6 % of visible light:
// Smith and Hudak 2005, Roy and others 2010, read); ash, grey to white where the flames burnt all (white ash 30 to 50 %,
// grey ash about 15 %: Smith and Hudak 2005, Bodí and others 2014); scorched soil, the reddish brown of mineral soil
// heated and bared (Certini 2005).
const CHAR = vec3f(0.022, 0.020, 0.019);
const ASH = vec3f(0.36, 0.35, 0.33);
const SCORCHED = vec3f(0.20, 0.12, 0.07);
// The char darkens a scar by e^-(months / `CHAR_MONTHS`): wind and rain take a burn's char and ash away within months
// (burn scars of savannas fade in 1 to 3 months, of forests over a year or more: Trigg and Flasse 2000); its bared,
// scorched soil fades by e^-(months / `SCORCHED_MONTHS`) as litter falls on it again.
const CHAR_MONTHS = 3.0; const SCORCHED_MONTHS = 12.0;
// Ash on the ground covers all but e^-1 of it at this much, kg/m² (about a millimetre of loose ash at 0.2 to 0.4 t/m³:
// Bodí and others 2014).
const ASH_HIDES_KG = 0.25;
// Cured leaves standing cover all but e^-1 of a herb layer at this much, kg/m² (a dry grass sward holds a few tenths
// of a kg/m² of standing dead leaf, from memory); the Earth view tints the cover towards straw by that share
// (`STRAW`, the pale yellow of dry grass in the style).
const CURED_HIDES_KG = 0.15; const STRAW = vec3f(0.62, 0.50, 0.22);
// The char's photograph (fire.jpg's second layer) is this many metres across.
const CHAR_PHOTO_M = 1.0;

// Spot p with the marks of fire on it: where it lies dry, the share of it under a scar (`Spot.scar.x`), how much of it
// is char, ash and scorched soil (`.yzw`); in the Earth view its cover tinted by the cured leaves standing in it.
fn burnt_spot(p: Spot) -> Spot {
  var q = p;
  if (p.depth > 0.0) { return q; }
  let f = fire_at(p.at);
  if (styled() && f.cured > 0.0) {
    q.cover = vec4f(mix(q.cover.rgb, STRAW * q.cover.a, -expm1(-f.cured / CURED_HIDES_KG)), q.cover.a);
  }
  if (f.burnt <= 0.0) { return q; }
  let scar = under_share(burn_rank(p.at, p.e), f.burnt);
  q.scar = vec4f(scar, exp(-f.age / CHAR_MONTHS), -expm1(-f.ash / ASH_HIDES_KG), exp(-f.age / SCORCHED_MONTHS));
  return q;
}

// The scar's colour over ground colour `c` at spot p: its scorched soil, its char, its ash (`burnt_spot`), each over
// the last; up close the char's photograph gives it its grain.
fn scar_over(c: vec3f, p: Spot) -> vec3f {
  let s = p.scar;
  var rgb = mix(c, SCORCHED, s.w);
  rgb = mix(rgb, CHAR, s.y);
  rgb = mix(rgb, ASH, s.z);
  return mix(c, rgb, s.x);
}

// The Earth view's floor `c` (colour.wgsl `ground_of`) under the scar, with the char's grain; and the share of it the
// scar takes from the soil's own tint.
fn burnt_floor(c: vec3f, p: Spot) -> vec4f {
  if (p.scar.x <= 0.0) { return vec4f(c, 0.0); }
  let grain = mix(1.0, photo_light(FIRE_FROM + 1, p.at / CHAR_PHOTO_M, p.e / CHAR_PHOTO_M), shows_at(CHAR_PHOTO_M, p.e) * max(p.scar.y, p.scar.w));
  return vec4f(scar_over(c, p) * mix(1.0, grain, p.scar.x), p.scar.x);
}

// The true colour `truth` of spot p's ground under its scar.
fn burnt_truth(truth: vec3f, p: Spot) -> vec3f { return scar_over(truth, p); }
