// A stream's bed sends back this share of the light soaked bare ground does: a film of algae and fine dead matter
// coats the stones under running water, and a bed sends back about 5% of the light against about 20% for soaked sand
// (Legleiter and others 2004; from memory).
const BED_FILM = 0.3;
// A streak of the flow reflects this share of the sky's light more than the water about it (a line of small ripples
// where the current shears). Fast shallow water is `FAST` of the way to `FAST_WATER`, the grey green of water full of
// air over a brown bed.
const REFLECT_STREAK = 0.25; const FAST = 0.25; const FAST_WATER = vec3f(0.16, 0.22, 0.17);
// A stone of the bed stands out of the water where it is `STONE_STANDS` of its size taller than the water is deep;
// fast water breaks white on it and trails `WAKE` of white water down the flow behind it (stone.wgsl `wake`).
const STONE_STANDS = 0.3; const WAKE = 0.45;
// A stone that stands in the water sits in its bed: its foot lies under the water and the spray and film of the flow
// wet it, so it takes `STONE_WATERED` of the water's colour over it.
const STONE_WATERED = 0.35;

// The colour of a channel's water at spot `p` (`land_spots`) over its bed `g` (colour.wgsl `ground_for`), a reflectance
// in linear light: its bed, soaked bare ground (washed sand and gravel: the flow takes the plants) under its film
// (`BED_FILM`), seen through running water as deep as it is there (water.wgsl); snow lies on it as on standing liquid
// (colour.wgsl `colour_on`), and a layer's view shades its bed as far as it shows (`Shaded.m`).
fn river_on(p: Spot, g: Ground, flow: vec3f, w: vec2f, down: vec2f) -> Shaded {
  let lit = light(1.0 + (p.shade - 1.0) * (0.8 + 0.3 * p.snow), p.slope);
  let soaked_bed = soil(g, 1.0, lit);
  let ripple = ripples(p.e) * (1.0 - p.snow);
  let k = loaded_k(RUNNING_CLEAR, p.load);
  var water = seen_through(BED_FILM * soaked_bed, p.depth, ripple, k, loaded_deep(DEEP_WATER, p.load));
  // In the style a channel's water is coloured by its bed's shape (style.wgsl `river_seen`): light shallows over the
  // bed's stones, a dark deep thread, a light wash at its edge.
  if (styled()) { water = mix(water_painted(BED_SHOWN * soaked_bed, river_seen(p.depth, w.x), ripple, p.load) * (1.0 + EDGE_LIGHT * w.y), FAST_WATER, FAST * flow.z); }
  // The signs of its flow (lib/flow.wgsl `flow_signs`): streaks catch the sky's light, white water breaks over its
  // steps.
  water = mix(water + flow.x * REFLECT_STREAK * SKY, FOAM_RGB, flow.y);
  // The stones that stand out of it, with their wakes.
  if (styled() && p.stony > 0.0 && !unseen(STONE_MOST_M / THING_PX.x)) {
    let s = Stony(p.stony, p.stone, p.small, p.worn, BED_SUNK, 0.0, 0.0, 0.0, 1.0);
    let least = p.depth / STONE_STANDS;
    if (flow.z > 0.0) { water = mix(water, FOAM_RGB, WAKE * flow.z * wake(p.e, s, least, down, 61 + 8)); }
    let st = stones(p.e, s, least, p.bed, 61 + 8);
    water = mix(water, mix(st.rgb / max(st.a, 1e-4), water, STONE_WATERED), st.a);
    // The blocks of a steep reach: the scree's own (colour.wgsl `ground_of`), where they stand out of the water.
    if (p.blocks > 0.0) {
      let fallen = Stony(p.blocks, SCREE_STONE_M, 1.0, 0.0, BED_SUNK, 0.0, 0.0, STONE_RARE, 1.0);
      water = mix(water, FOAM_RGB, WAKE * flow.z * wake(p.e, fallen, least, down, 61));
      let block = stones(p.e, fallen, least, p.bed, 61);
      water = mix(water, mix(block.rgb / max(block.a, 1e-4), water, STONE_WATERED), block.a);
    }
  }
  let rgb = mix(water, SNOW, p.snow * 0.85);
  return Shaded(rgb, 1.0 + (lit - 1.0) * exp(-2.0 * k.g * p.depth), RIVER);
}
