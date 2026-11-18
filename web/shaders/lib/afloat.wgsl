// What floats on still standing liquid up close (colour.wgsl `colour_on`): the leaves the woody crowns about it shed
// this month. A lake's sheltered liquid, not the sea's waves nor a channel's flow, holds them.

// Fallen leaves drift on the water out to `LEAF_REACH_M` from its shore (a crown's reach and a little drift), on
// `LEAVES_MOST` of it where all the ground about is under crowns shedding (`Spot.shed`): one leaf at most in each cell
// of a lattice `LEAF_CELL_M` wide, `LEAF_LONG_M` long by its own draw, a third as wide, turned its own way, of one of
// the colours of fallen leaves (`leaf_tone`, linear).
const LEAF_REACH_M = 8.0; const LEAVES_MOST = 0.25; const LEAF_CELL_M = 0.14; const LEAF_LONG_M = vec2f(0.05, 0.1);
fn leaf_tone(k: u32) -> vec3f { var t = array<vec3f, 3>(vec3f(0.30, 0.20, 0.05), vec3f(0.24, 0.10, 0.03), vec3f(0.12, 0.08, 0.04)); return t[k]; }

// Water `water` (linear) of spot p with the leaves that float on it over it.
fn afloat(water: vec3f, p: Spot) -> vec3f {
  let still = (1.0 - p.sea) * (1.0 - p.mouth) * (1.0 - p.snow);
  if (still <= 0.0 || p.shed <= 0.0 || unseen(LEAF_CELL_M)) { return water; }
  let slope = max(p.slope / earth.look.x, 1e-2);
  let leaves = LEAVES_MOST * p.shed * (1.0 - ease(0.0, LEAF_REACH_M, p.depth / slope)) * still;
  if (leaves <= 0.0) { return water; }
  return floating_leaves(water, leaves, p.e);
}

// Leaves over `water` on `cover` of it (`afloat`), seen at e m a pixel.
fn floating_leaves(water: vec3f, cover: f32, e: f32) -> vec3f {
  let held = min(cover * LEAF_CELL_M * LEAF_CELL_M / (PI * LEAF_LONG_M.y * LEAF_LONG_M.y / 12.0), 1.0);
  let mean = mix(water, leaf_tone(0u), cover);
  let seen = as_thing(LEAF_LONG_M.x, e);
  if (seen <= 0.0) { return mean; }
  let l = lat_axes(LEAF_CELL_M, vec2f(0.0));
  var out = water;
  for (var j = -1; j <= 1; j++) {
    for (var i = -1; i <= 1; i++) {
      let cell = l.i + vec2i(i, j);
      let x = lowbias(cell, 153);
      if (f32(x & 1023u) / 1024.0 >= held) { continue; }
      let centre = vec2f(f32(i), f32(j)) + vec2f(f32((x >> 10u) & 255u), f32((x >> 18u) & 255u)) / 256.0;
      let long = 0.5 * mix(LEAF_LONG_M.x, LEAF_LONG_M.y, f32((x >> 26u) & 15u) / 15.0) / LEAF_CELL_M;
      let turn = PI * f32(lowbias(cell, 154) & 1023u) / 1024.0;
      let q = mat2x2f(cos(turn), -sin(turn), sin(turn), cos(turn)) * (l.f - centre);
      let d = length(q / vec2f(long, long / 3.0));
      let a = 1.0 - smoothstep(1.0 - e / (long * LEAF_CELL_M), 1.0, d);
      out = mix(out, leaf_tone((x >> 30u) % 3u), max(a, 0.0));
    }
  }
  return mix(mean, out, seen);
}
