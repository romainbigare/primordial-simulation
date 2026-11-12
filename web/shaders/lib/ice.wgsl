// Ice on standing liquid (colour.wgsl `colour_on`): where in its cells the cells' share of ice lies, and how it looks
// from above. The share is true only cell by cell (the open low ground's cells are 8 km and freeze as one surface), so
// its place is drawn: an index, lowered by `ICE_SHALLOW` over liquid shallower than `ICE_SHALLOW_M` (a shallow bay
// loses its heat first), and moved by octaves of noise from `ICE_PATCH_M` down to the floes' scale (`ICE_PATCHY` of the
// index); the share s covers where the index lies under ln(s / (1 - s)), smoothed over `ICE_BAND` of it. Of that noise
// spread, the share drawn over open liquid is the cells' within a few hundredths (a share of 0.1 draws 0.10, 0.3 draws
// 0.31, 0.7 draws 0.69, 0.9 draws 0.90), and no edge follows a cell's border.
const ICE_SHALLOW = 1.0; const ICE_SHALLOW_M = 10.0;
const ICE_PATCH_M = 6000.0; const ICE_PATCH_STEP = 2.9; const ICE_PATCH_FALL = 0.62; const ICE_PATCH_OCTAVES = 5;
const ICE_PATCHY = 3.5; const ICE_BAND = 1.2; const ICE_OCTAVE_PX = 4.0;

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
// The pack from above (Weeks 2010, from memory): pressure ridges, short straight lines where floes were pressed
// together, one in `RIDGE_OF` of the cells `PACK_RIDGE_M` across, each up to `RIDGE_LONG` of a cell long and
// `RIDGE_W_M` wide, its face toward the sun lit (`ICE_RIDGE`) and the other shaded (`RIDGE_SHADE` of the ice);
// refrozen leads, dark cracks that end, one in `CRACK_OF` of the cells `PACK_CRACK_M` across, up to `CRACK_LONG` of a
// cell long and `CRACK_W_M` wide, tapering to nothing at their ends; and the snow's drift over `PACK_DRIFT_M`,
// `PACK_DRIFT` of its tone. Ridges and cracks lie in fields `LINE_FIELD_M` across where they run one way (the floes
// were pressed or pulled apart one way there), each turned from it by up to `LINE_TURN` radians either way, and come
// in bands as dense as a noise over that field. Each is fixed in the world.
const PACK_RIDGE_M = 140.0; const RIDGE_OF = 0.7; const RIDGE_LONG = 0.9; const RIDGE_W_M = 11.0; const RIDGE_SHADE = 0.68;
const PACK_CRACK_M = 700.0; const CRACK_OF = 0.7; const CRACK_LONG = 1.3; const CRACK_W_M = 9.0;
const PACK_DRIFT_M = 180.0; const PACK_DRIFT = 0.05; const LINE_FIELD_M = 2500.0; const LINE_TURN = 0.45;
// Close, a line's width varies by up to `LINE_JAG` of itself over pieces `LINE_PIECE` of its widths long, and it breaks
// where a noise four times slower along it is under `LINE_BREAKS`.
const LINE_JAG = 0.6; const LINE_PIECE = 1.5; const LINE_BREAKS = -0.45;
// The drift's tone is read from `PACK_PX` pixels a patch: finer, it changes the pack's tone by less than a step of the
// screen's. A crack or a ridge fades in as its width grows from the first of `PACK_LINE_PX` pixels to the second.
const PACK_PX = 8.0; const PACK_LINE_PX = vec2f(1.5, 3.0);
// Sea ice's colours (reflectances): old ice under snow, cooler and greyer than the land's snow and lit flat (it lies
// level); grey new ice; refrozen leads; a ridge's lit face. Along the shore a tide crack: ice over liquid shallower than
// `ICE_TIDE_M` is dark new ice, so a coast stays readable under the ice.
const ICE_OLD = vec3f(0.74, 0.80, 0.86); const ICE_NEW = vec3f(0.30, 0.37, 0.43); const ICE_REFROZEN = vec3f(0.58, 0.65, 0.71);
const ICE_RIDGE = vec3f(0.90, 0.93, 0.96); const ICE_TIDE_M = 1.5;

// The share of liquid `depth` m deep at `at`, seen at `e` m a pixel, under ice where its cells hold `share` of theirs
// under ice: 0..1, smooth across the margin (the floes' cover there).
fn ice_cover(share: f32, depth: f32, at: vec2f, e: f32) -> f32 {
  if (share <= 0.0 || share >= 1.0) { return share; }
  let line = log(share / (1.0 - share));
  let shallow = -ICE_SHALLOW * (1.0 - ease(0.0, ICE_SHALLOW_M, depth));
  // The octaves' weights summed, whether they show or not, so the index's spread is the same at every zoom; and the
  // index's change a unit of the summed noise.
  let sum = (1.0 - pow(ICE_PATCH_FALL, f32(ICE_PATCH_OCTAVES))) / (1.0 - ICE_PATCH_FALL);
  let k = 2.0 * ICE_PATCHY / sum;
  var n = 0.0;
  var size = ICE_PATCH_M;
  var weight = 1.0;
  // What the octaves not yet read can still move the index by at most: once the index lies past the margin by more,
  // the point is all ice or all water whatever they hold, and they are not read (the picture is the same).
  var rest = k * sum;
  for (var o = 0; o < ICE_PATCH_OCTAVES; o++) {
    let past = shallow + clamp(k * n, -ICE_PATCHY, ICE_PATCHY) - line;
    if (past - rest >= ICE_BAND) { return 0.0; }
    if (past + rest <= -ICE_BAND) { return 1.0; }
    // An octave under `ICE_OCTAVE_PX` pixels only frets the edge's last pixels: its noise is not read (and in a draw
    // of pixels that large, not compiled: draw.wgsl `LEAST_E`).
    if (size < ICE_OCTAVE_PX * max(e, least_e())) { break; }
    n += weight * shows_at(size, e) * noise_turned(at / size + f32(o) * 3.7, 90 + o);
    rest -= k * weight;
    size /= ICE_PATCH_STEP;
    weight *= ICE_PATCH_FALL;
  }
  let index = shallow + clamp(k * n, -ICE_PATCHY, ICE_PATCHY);
  return 1.0 - smoothstep(-ICE_BAND, ICE_BAND, index - line);
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
  // Each detail's noise is read only where it shows.
  var c = ice;
  let drift = select(shows_at(PACK_DRIFT_M, e), 0.0, PACK_DRIFT_M < PACK_PX * max(e, least_e()));
  if (drift > 0.0) { c *= 1.0 + PACK_DRIFT * noise_turned(at / PACK_DRIFT_M, 98) * drift; }
  // Each kind of line fades in as its width grows from `PACK_LINE_PX` pixels (a hatch of lines under that only speckles).
  let cracks = ease(PACK_LINE_PX.x, PACK_LINE_PX.y, CRACK_W_M / max(e, least_e()));
  if (cracks > 0.0) {
    let k = segments(at, e, PACK_CRACK_M, CRACK_OF, CRACK_LONG, CRACK_W_M, 99);
    c = mix(c, ICE_REFROZEN * RIDGE_SHADE, k.x * cracks);
  }
  let ridges = ease(PACK_LINE_PX.x, PACK_LINE_PX.y, RIDGE_W_M / max(e, least_e()));
  if (ridges > 0.0) {
    let r = segments(at, e, PACK_RIDGE_M, RIDGE_OF, RIDGE_LONG, RIDGE_W_M, 100);
    c = mix(c, mix(c * RIDGE_SHADE, ICE_RIDGE, r.y), r.x * ridges);
  }
  return c;
}

// Short straight lines fixed in the world at `at`, seen at `e` m a pixel: one in `held` of the cells `cell` m across
// holds one, through a point anywhere in it, turned any way, up to `longest` of a cell long and `width` m wide, tapering to its
// ends. Of the nearest: how much of the pixel it covers (a line under a pixel wide drawn a pixel wide, as faint as it is
// narrow), and how lit its face there is, 0..1 (its side toward the sun, `LIGHT`).
fn segments(at: vec2f, e: f32, cell: f32, held: f32, longest: f32, width: f32, k: i32) -> vec2f {
  let q = at / cell;
  let c = floor(q);
  var best = 0.0;
  var lit = 0.5;
  for (var j = -1; j <= 1; j++) {
    for (var i = -1; i <= 1; i++) {
      let id = c + vec2f(f32(i), f32(j));
      let x = lowbias(vec2i(id), k);
      let centre = (id + vec2f(f32((x >> 10u) & 255u), f32((x >> 18u) & 255u)) / 256.0) * cell;
      // The field's way and density at the line's centre (`LINE_FIELD_M`).
      let field = vec2f(noise_turned(centre / LINE_FIELD_M, k + 7), noise_turned(centre / LINE_FIELD_M + 9.0, k + 8));
      if (f32(x & 1023u) / 1024.0 >= held * 2.0 * ease(-0.35, 0.35, field.y)) { continue; }
      let turn = field.x * PI + LINE_TURN * (f32((x >> 26u) & 63u) / 31.5 - 1.0);
      let along = vec2f(cos(turn), sin(turn));
      let reach = 0.5 * longest * cell * (0.4 + 0.6 * f32((x >> 4u) & 63u) / 63.0);
      let v = at - centre;
      let t = dot(v, along);
      let across = dot(v, vec2f(-along.y, along.x));
      // Tapering to its ends: as wide as `width` in its middle, nothing at them; and near, as rough as rubble along it
      // (`LINE_JAG` of its width over pieces `LINE_PIECE` widths long) and broken where a slower noise along it is low.
      var w = 0.5 * width * sqrt(max(1.0 - (t / reach) * (t / reach), 0.0));
      if (w > 0.0 && abs(across) < 2.0 * w + e) {
        let seed = f32(x & 255u);
        w *= 1.0 + LINE_JAG * noise(vec2f(t / (LINE_PIECE * width), seed), k + 20).x * shows_at(LINE_PIECE * width, e);
        w *= ease(LINE_BREAKS, LINE_BREAKS + 0.2, noise(vec2f(t / (4.0 * LINE_PIECE * width), seed), k + 21).x);
      }
      let drawn = max(w, 0.5 * e);
      let cover = (1.0 - smoothstep(drawn - 0.5 * e, drawn + 0.5 * e, abs(across))) * min(w / drawn, 1.0);
      if (cover > best) {
        best = cover;
        // The side of the ridge turned toward the sun (its normal across, signed by the side the point lies on).
        let n = sign(across) * vec2f(-along.y, along.x);
        lit = smoothstep(-0.2, 0.2, -dot(n, LIGHT.xy) / length(LIGHT.xy));
      }
    }
  }
  return vec2f(best, lit);
}

// Water of colour `water`, `depth` m deep, at `at`, seen at `e` m a pixel, under ice that covers `cover` of it
// (`ice_cover`): the pack, its floes and brash at its margin (their cover's mean from afar), grey where the ice is new,
// and the tide crack along the shore.
fn ice_on(water: vec3f, cover: f32, depth: f32, at: vec2f, e: f32) -> vec3f {
  if (cover <= 0.0) { return water; }
  var ice = mix(ICE_NEW, ICE_OLD, ease(0.2, 0.9, cover));
  ice = mix(ICE_NEW, ice, ease(0.0, ICE_TIDE_M, depth));
  let near = ease(FLOE_PX.x * e, FLOE_PX.y * e, FLOE_M);
  var on = cover;
  // Floes show from `FLOE_PX` pixels a floe (in a draw of larger pixels they are not compiled: draw.wgsl `LEAST_E`),
  // and not in the closed pack; each lattice is read only where it shows.
  if (near > 0.0 && cover < PACK_WHOLE.y && FLOE_PX.x * least_e() < FLOE_M) {
    let wide = ease(-0.2, 0.2, noise_turned(at / FLOE_REGION_M, 101));
    var f = 0.0;
    if (wide < 1.0) { f += (1.0 - wide) * floes(at, e, cover, FLOE_M, 97); }
    if (wide > 0.0) { f += wide * floes(at, e, cover, FLOE_WIDE_M, 103); }
    // Brash in the leads, where its pieces take a few pixels.
    let brash = ease(FLOE_PX.x * e, FLOE_PX.y * e, BRASH_M);
    if (brash > 0.0 && f < 1.0) { f = max(f, brash * floes(at, e, BRASH * cover, BRASH_M, 107)); }
    // In the closed pack the floes have frozen together: only its own texture shows (`PACK_WHOLE`).
    on = mix(mix(cover, f, near), 1.0, ease(PACK_WHOLE.x, PACK_WHOLE.y, cover));
  } else if (cover >= PACK_WHOLE.y) { on = 1.0; }
  // Between the floes open water where the ice is loose; in the pack the leads have refrozen (`LEAD_REFROZEN`).
  let between = mix(water, ICE_REFROZEN, ease(LEAD_REFROZEN.x, LEAD_REFROZEN.y, cover));
  return mix(between, pack(ice, at, e), on);
}
