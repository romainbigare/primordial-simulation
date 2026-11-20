// Burn scars on the live map's dry ground (burnt.wgsl), from what fire did on each square (fire.wgsl): in the patches of
// a square's burnt share, the ground is black with char and grey with ash where the flames left it, the char fades to
// the scorched brown of bare burnt soil and that to the ground's own colour as the months pass, and the plants come
// back by the core's own numbers. The Earth view draws the cover thinner in the scars and as much thicker outside them,
// so a square keeps its mean; the true view shows the ground's reflectance.

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
// The Earth view draws a scar's cover thinner by this share of it while the scar is fresh (e^-(months / `BARE_MONTHS`)):
// a fire takes nearly all of the fine fuel it runs through, and herbs come back within a season; the trees drawn one
// by one keep the core's numbers.
const SCAR_BARE = 1.0; const BARE_MONTHS = 6.0;
// What comes back on a scar is young leaf without the dead thatch about it, a brighter green than the plants about it
// (the green flush after a fire, Archibald and others 2009): the Earth view tints its cover towards `FLUSH` by up to
// `FLUSH_SHARE`, as long as its soil shows scorched and once its char is gone.
const FLUSH = vec3f(0.16, 0.36, 0.05); const FLUSH_SHARE = 0.6;
// The char's photograph (fire.jpg's second layer) is this many metres across.
const CHAR_PHOTO_M = 1.0;

// Spot p with the marks of fire on it: where it lies dry, the share of it under a scar (`Spot.scar.x`), how much of it
// is char, ash and scorched soil (`.yzw`); in the Earth view its cover moved out of the scar's patches.
fn burnt_spot(p: Spot) -> Spot {
  var q = p;
  let f = fire_at(p.at);
  if (f.burnt <= 0.0 || p.depth > 0.0) { return q; }
  let scar = under_share(burn_rank(p.at, p.e), f.burnt);
  let ash = -expm1(-f.ash / ASH_HIDES_KG);
  q.scar = vec4f(scar, exp(-f.age / CHAR_MONTHS), ash, exp(-f.age / SCORCHED_MONTHS));
  if (styled()) {
    // Thinner in the scar by `bare` of its cover, thicker outside it by as much over the rest of the square, as far
    // as the cover can grow.
    let bare = SCAR_BARE * exp(-f.age / BARE_MONTHS);
    let outside = min(1.0 + bare * f.burnt / max(1.0 - f.burnt, 1e-3), 1.0 / max(p.cover.a, 1e-3));
    let k = mix(max(outside, 1.0), 1.0 - bare, scar);
    q.cover *= k;
    let flush = FLUSH_SHARE * scar * (1.0 - q.scar.y) * q.scar.w;
    q.cover = vec4f(mix(q.cover.rgb, FLUSH * q.cover.a, flush), q.cover.a);
    q.low *= k;
    q.canopy *= k;
  }
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
