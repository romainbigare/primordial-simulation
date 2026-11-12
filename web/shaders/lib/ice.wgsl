// Ice on standing liquid (colour.wgsl `colour_on`): where in its cells the cells' share of ice lies, and how it looks
// from above. The share is true only cell by cell (the open low ground's cells are 8 km and freeze as one surface), so
// its place is drawn: an index, lowered by `ICE_SHALLOW` over liquid shallower than `ICE_SHALLOW_M` (a shallow bay
// loses its heat first), and moved by octaves of noise from `ICE_PATCH_M` down to the floes' scale (`ICE_PATCHY` of the
// index); the share s covers where the index lies under ln(s / (1 - s)), smoothed over `ICE_BAND` of it. Of that noise
// spread, the share drawn over open liquid is the cells' within a few hundredths (a share of 0.1 draws 0.10, 0.3 draws
// 0.31, 0.7 draws 0.69, 0.9 draws 0.90), and no edge follows a cell's border.
const ICE_SHALLOW = 1.0; const ICE_SHALLOW_M = 10.0;
const ICE_PATCH_M = 6000.0; const ICE_PATCH_STEP = 2.9; const ICE_PATCH_FALL = 0.62; const ICE_PATCH_OCTAVES = 5;
const ICE_PATCHY = 3.5; const ICE_BAND = 1.2;

// Over its margin the sheet breaks into floes (a Voronoi diagram, Worley 1996, its cells' edges bent by noise
// `FLOE_BEND` of a cell, so none is a straight polygon): wide floes `FLOE_WIDE_M` across where a slow noise over
// `FLOE_REGION_M` is high, smaller ones `FLOE_M` elsewhere, each whole or open by its own draw against the cover there;
// leads between them as wide as `LEAD` of a floe where the cover is thin, closing to cracks `LEAD_LEAST` wide in the
// pack, each lead its own width (`LEAD_VARY`); and in the leads brash, pieces `BRASH_M` across, `BRASH` as many as the
// cover. Floes show from `FLOE_PX` pixels a floe; smaller, the cover's mean.
const FLOE_WIDE_M = 400.0; const FLOE_M = 110.0; const FLOE_REGION_M = 2500.0; const FLOE_BEND = 0.25;
const LEAD = 0.12; const LEAD_LEAST = 0.015; const LEAD_VARY = 0.6; const BRASH_M = 25.0; const BRASH = 0.6;
const FLOE_PX = vec2f(4.0, 10.0);
// The leads between floes have refrozen from this cover of ice to that one: open dark water at the margin, grey new ice
// in the pack.
const LEAD_REFROZEN = vec2f(0.6, 0.95);
// From this cover of ice to that one the floes' outlines fade into the closed pack.
const PACK_WHOLE = vec2f(0.9, 0.99);
// The pack from above: refrozen leads, long cracks `PACK_CRACK_M` apart, `PACK_CRACK_W` of that wide, greyer; pressure
// ridges `PACK_RIDGE_M` apart as fine light lines; and the snow's drift over `PACK_DRIFT_M`, `PACK_DRIFT` of its tone.
const PACK_CRACK_M = 900.0; const PACK_CRACK_W = 0.04; const PACK_RIDGE_M = 260.0; const PACK_RIDGE_W = 0.03;
const PACK_DRIFT_M = 180.0; const PACK_DRIFT = 0.05;
// Sea ice's colours (reflectances): old ice under snow, cooler and greyer than the land's snow and lit flat (it lies
// level); grey new ice; refrozen leads; a ridge's lit face. Along the shore a tide crack: ice over liquid shallower than
// `ICE_TIDE_M` is dark new ice, so a coast stays readable under the ice.
const ICE_OLD = vec3f(0.74, 0.80, 0.86); const ICE_NEW = vec3f(0.30, 0.37, 0.43); const ICE_REFROZEN = vec3f(0.58, 0.65, 0.71);
const ICE_RIDGE = vec3f(0.90, 0.93, 0.96); const ICE_TIDE_M = 1.5;

// The share of liquid `depth` m deep at `at`, seen at `e` m a pixel, under ice where its cells hold `share` of theirs
// under ice: 0..1, smooth across the margin (the floes' cover there).
fn ice_cover(share: f32, depth: f32, at: vec2f, e: f32) -> f32 {
  if (share <= 0.0 || share >= 1.0) { return share; }
  var n = 0.0;
  var size = ICE_PATCH_M;
  var weight = 1.0;
  var sum = 0.0;
  for (var o = 0; o < ICE_PATCH_OCTAVES; o++) {
    let shown = shows_at(size, e);
    if (shown > 0.0) { n += weight * shown * noise_turned(at / size + f32(o) * 3.7, 90 + o); }
    sum += weight;
    size /= ICE_PATCH_STEP;
    weight *= ICE_PATCH_FALL;
  }
  let index = -ICE_SHALLOW * (1.0 - ease(0.0, ICE_SHALLOW_M, depth)) + ICE_PATCHY * clamp(2.0 * n / sum, -1.0, 1.0);
  return 1.0 - smoothstep(-ICE_BAND, ICE_BAND, index - log(share / (1.0 - share)));
}

// The floes of one lattice of cells `size` m across at `at`, seen at `e` m a pixel, under `cover` of ice, stream `k`:
// the share of the point on a floe (its lead's edge anti-aliased), with a lead whose width closes as the cover grows.
fn floes(at: vec2f, e: f32, cover: f32, size: f32, k: i32) -> f32 {
  let bend = FLOE_BEND * vec2f(noise_turned(at / (0.6 * size), k + 1), noise_turned(at / (0.6 * size) + 31.0, k + 2));
  let q = at / size + bend;
  let c = floor(q);
  var d1 = 1e9;
  var d2 = 1e9;
  var id = 0u;
  for (var j = -1; j <= 1; j++) {
    for (var i = -1; i <= 1; i++) {
      let cell = c + vec2f(f32(i), f32(j));
      let x = lowbias(vec2i(cell), k);
      let centre = cell + vec2f(f32(x & 1023u), f32((x >> 10u) & 1023u)) / 1024.0;
      let d = length(q - centre);
      if (d < d1) { d2 = d1; d1 = d; id = x; } else if (d < d2) { d2 = d; }
    }
  }
  let whole = select(0.0, 1.0, f32(id >> 22u) / 1024.0 < cover);
  let vary = 1.0 + LEAD_VARY * (f32((id >> 20u) & 3u) / 1.5 - 1.0);
  let lead = max(LEAD * (1.0 - cover) * vary, LEAD_LEAST);
  return whole * smoothstep(lead, lead + e / size, 0.5 * (d2 - d1));
}

// The pack's tone at `at`, seen at `e` m a pixel, over ice of colour `ice`: refrozen leads, ridges, drift.
fn pack(ice: vec3f, at: vec2f, e: f32) -> vec3f {
  var c = ice * (1.0 + PACK_DRIFT * noise_turned(at / PACK_DRIFT_M, 98) * shows_at(PACK_DRIFT_M, e));
  let cracks = crack(noise_turned(at / PACK_CRACK_M, 99), PACK_CRACK_W) * shows_at(PACK_CRACK_M * PACK_CRACK_W * 4.0, e);
  c = mix(c, ICE_REFROZEN, cracks);
  let ridges = crack(noise_turned(at / PACK_RIDGE_M, 100), PACK_RIDGE_W) * shows_at(PACK_RIDGE_M * PACK_RIDGE_W * 4.0, e);
  return mix(c, ICE_RIDGE, ridges);
}

// Water of colour `water`, `depth` m deep, at `at`, seen at `e` m a pixel, under ice that covers `cover` of it
// (`ice_cover`): the pack, its floes and brash at its margin (their cover's mean from afar), grey where the ice is new,
// and the tide crack along the shore.
fn ice_on(water: vec3f, cover: f32, depth: f32, at: vec2f, e: f32) -> vec3f {
  if (cover <= 0.0) { return water; }
  var ice = mix(ICE_NEW, ICE_OLD, ease(0.2, 0.9, cover));
  ice = mix(ICE_NEW, ice, ease(0.0, ICE_TIDE_M, depth));
  let wide = ease(-0.2, 0.2, noise_turned(at / FLOE_REGION_M, 101));
  let near = ease(FLOE_PX.x * e, FLOE_PX.y * e, FLOE_M);
  var on = cover;
  if (near > 0.0) {
    var f = mix(floes(at, e, cover, FLOE_M, 97), floes(at, e, cover, FLOE_WIDE_M, 103), wide);
    // Brash in the leads, where its pieces take a few pixels.
    let brash = ease(FLOE_PX.x * e, FLOE_PX.y * e, BRASH_M);
    if (brash > 0.0 && f < 1.0) { f = max(f, brash * floes(at, e, BRASH * cover, BRASH_M, 107)); }
    // In the closed pack the floes have frozen together: only its own texture shows (`PACK_WHOLE`).
    on = mix(mix(cover, f, near), 1.0, ease(PACK_WHOLE.x, PACK_WHOLE.y, cover));
  }
  // Between the floes open water where the ice is loose; in the pack the leads have refrozen (`LEAD_REFROZEN`).
  let between = mix(water, ICE_REFROZEN, ease(LEAD_REFROZEN.x, LEAD_REFROZEN.y, cover));
  return mix(between, pack(ice, at, e), on);
}
