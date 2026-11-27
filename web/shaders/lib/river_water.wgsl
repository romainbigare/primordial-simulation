// A stream's bed sends back this share of the light soaked bare ground does: a film of algae and fine dead matter
// coats the stones under running water, and a bed sends back about 5% of the light against about 20% for soaked sand
// (Legleiter and others 2004; from memory).
const BED_FILM = 0.3;
// A stone of the bed stands out of the water where it is `STONE_STANDS` of its size taller than the water is deep;
// fast water breaks white on it and trails `WAKE` of white water down the flow behind it (stone.wgsl `wake`).
const STONE_STANDS = 0.3; const WAKE = 0.45;
// A stone that stands in the water sits in its bed: its foot lies under the water and the spray and film of the flow
// wet it, so it takes `STONE_WATERED` of the water's colour over it.
const STONE_WATERED = 0.35;

// The surface of running water (Spots.flow: how broken it is, its white water, how fast it runs): a pool lies smooth,
// of mean square slope `POOL_MSS`; a riffle's water is broken, up to `RIFFLE_MSS` (a surface broken by the bed's
// stones, as steep as wind waves of 15 m/s by Cox and Munk 1954), and white over up to `RIFFLE_FOAM` of it where it is
// more broken than `RIFFLE_WHITE`. A channel too narrow for its pools and riffles to show is `ROUGH_FAR` broken, the mean of a reach.
const POOL_MSS = 0.004; const RIFFLE_MSS = 0.08; const RIFFLE_FOAM = 0.18; const RIFFLE_WHITE = 0.5; const ROUGH_FAR = 0.45;
// The surface's pattern runs down the flow (a flow map: two copies slid along it, each faded out as it starts again
// and the other at its full, so it never stretches and never jumps; Vlachos 2010): each copy runs `FLOW_CYCLE_M` of
// the slowest water's slide (web/src/land.ts `slid`, which comes round after a length this divides), a pool's at that
// pace, a riffle's `PACE_ROUGH` more and a steep reach's `PACE_FAST` more; the second copy is set `FLOW_COPY_M` off.
const FLOW_CYCLE_M = 2.0; const PACE_ROUGH = 1.0; const PACE_FAST = 2.0; const FLOW_COPY_M = vec2f(1.37, 2.71);
// Foam is white water's lace (tex.wgsl `foam_read`): a share s of the water white covers the texels of the lace over
// 1 - s, softened over `FOAM_SOFT` of it either side, so its mean is s at any zoom; its light varies by `FOAM_TONE` of
// the lace's own.
const FOAM_SOFT = 0.12; const FOAM_TONE = 0.6;
// Water whose bed's shape across it does not show is coloured as its mean across: as deep as `MEAN_REL` of its middle
// (a bed of a parabola's shape, its depth to the power `RIVER_SEEN_POW` on the mean), so its far tone is its near
// one's mean.
const MEAN_REL = 0.69;

// The surface of running water seen at `e` m a pixel, running `down` (a unit vector) with the signs `flow` of its flow
// (lib/flow.wgsl `flow_signs`), white over `white` of it: its slope east and south, the mean square slope of the waves
// too small to show, the light they focus on its bed (1 on the mean), and the share of it foam covers.
struct Surface { g: vec2f, hidden: f32, caustic: f32, foam: f32, tone: f32 }
fn running_surface(e: f32, down: vec2f, flow: vec3f, white: f32) -> Surface {
  let mss = mix(POOL_MSS, RIFFLE_MSS, flow.x);
  let pace = 1.0 + PACE_ROUGH * flow.x + PACE_FAST * flow.z;
  let cycle = fract(land.reach.z / FLOW_CYCLE_M);
  let laced = shows_at(FOAM_LACE_M, e);
  var g = vec2f(0.0);
  var weights = 0.0;
  var s = Surface(vec2f(0.0), mss * (1.0 - waves_shown(e)), 0.0, 0.0, 0.0);
  for (var k = 0; k < rolled(2); k++) {
    let phase = fract(cycle + 0.5 * f32(k));
    let weight = 1.0 - abs(2.0 * phase - 1.0);
    let drift = down * (pace * FLOW_CYCLE_M * phase) + f32(k) * FLOW_COPY_M;
    let w = waves_read(e, drift);
    g += weight * w.xy;
    weights += weight * weight;
    s.caustic += weight * w.z;
    var lace = vec2f(-1.0, 0.5);
    if (white > 0.0) { lace = foam_read(e, drift); }
    s.foam += weight * select(white, smoothstep(1.0 - white - FOAM_SOFT, 1.0 - white + FOAM_SOFT, lace.x), lace.x >= 0.0);
    s.tone += weight * lace.y;
  }
  // (The two copies' slopes are summed as random waves are: their spread kept whole as the weights shift.)
  s.g = sqrt(0.5 * mss) * g / sqrt(max(weights, 1e-6));
  s.foam = mix(white, s.foam, laced);
  s.tone = mix(1.0, 2.0 * s.tone, FOAM_TONE * laced);
  return s;
}

// The colour of a channel's water at spot `p` (`land_spots`) over its bed `g` (colour.wgsl `ground_for`), a reflectance
// in linear light, with the signs `flow` of its flow, its depth over its middle's and its wash at the edge `w`
// (river.wgsl `river_spots`), as far as its bed's shape shows (`shaped`), running `down`: its bed, soaked bare ground (washed sand and gravel: the flow takes the
// plants) under its film (`BED_FILM`), seen through running water as deep as it is there (water.wgsl), in the style
// coloured by what it holds (style.wgsl `inland_optics`) and by its bed's shape (`river_seen`); its surface's waves
// and foam (`running_surface`); snow lies on it as on standing liquid (colour.wgsl `colour_on`), and a layer's view
// shades its bed as far as it shows (`Shaded.m`).
fn river_on(p: Spot, g: Ground, flow: vec3f, w: vec2f, down: vec2f, shaped: f32) -> Shaded {
  let lit = light(1.0 + (p.shade - 1.0) * (0.8 + 0.3 * p.snow), p.slope);
  let soaked_bed = soil(g, 1.0, lit);
  // White water: over its steps and cascades, where its riffles break, and in the wakes of the stones that stand out
  // of it.
  var white = max(flow.y, RIFFLE_FOAM * ease(RIFFLE_WHITE, 1.0, flow.x));
  let s = Stony(p.stony, p.stone, p.small, p.worn, BED_SUNK, 0.0, 0.0, 0.0, 1.0);
  let fallen = Stony(p.blocks, SCREE_STONE_M, 1.0, 0.0, BED_SUNK, 0.0, 0.0, STONE_RARE, 1.0);
  let least = p.depth / STONE_STANDS;
  let stones_seen = styled() && p.stony > 0.0 && !unseen(STONE_MOST_M / THING_PX.x);
  if (stones_seen && flow.z > 0.0) {
    white = max(white, WAKE * flow.z * wake(p.e, s, least, down, 61 + 8));
    if (p.blocks > 0.0) { white = max(white, WAKE * flow.z * wake(p.e, fallen, least, down, 61)); }
  }
  let surf = running_surface(p.e, down, flow, white * (1.0 - p.snow));
  let calm = 1.0 - p.snow;
  let k = loaded_k(RUNNING_CLEAR, p.load);
  var water = seen_through(BED_FILM * soaked_bed, p.depth, surf.g * calm, k, loaded_deep(DEEP_WATER, p.load));
  if (styled()) {
    // In the style the bed takes the light the waves focus on it in the shallows, and the water is coloured by what it
    // holds and by its bed's shape: light shallows over the bed's stones, a dark deep thread, a light wash at its edge.
    let bed = BED_SHOWN * soaked_bed * mix(1.0, surf.caustic, CAUSTICS * calm * (1.0 - ease(0.0, CAUSTIC_DEEP_M, p.depth)));
    let o = inland_optics(p.t, p.grain.z, p.canopy, p.load);
    water = painted_through(bed, river_seen(p.depth, mix(MEAN_REL, w.x, shaped)), surf.g * calm, surf.hidden * calm, o) * (1.0 + EDGE_LIGHT * w.y);
  }
  water = mix(water, FOAM_RGB * lit * surf.tone, surf.foam);
  // The stones that stand out of it.
  if (stones_seen) {
    let st = stones(p.e, s, least, p.bed, 61 + 8);
    water = mix(water, mix(st.rgb / max(st.a, 1e-4), water, STONE_WATERED), st.a);
    // The blocks of a steep reach: the scree's own (colour.wgsl `ground_of`), where they stand out of the water.
    if (p.blocks > 0.0) {
      let block = stones(p.e, fallen, least, p.bed, 61);
      water = mix(water, mix(block.rgb / max(block.a, 1e-4), water, STONE_WATERED), block.a);
    }
  }
  let rgb = mix(water, SNOW, p.snow * 0.85);
  return Shaded(rgb, 1.0 + (lit - 1.0) * exp(-2.0 * k.g * p.depth), RIVER);
}
