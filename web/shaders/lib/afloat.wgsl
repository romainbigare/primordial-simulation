// What floats on still standing liquid up close (colour.wgsl `colour_on`): the leaves the woody crowns about it shed
// this month, and the round floating leaves of the herbs that stand in it deeper than they are tall (`Spot.pads`: the
// core stands them on its ponds' ground as deep as their roots reach). A lake's sheltered liquid, not the sea's waves
// nor a channel's flow, holds them.

// Fallen leaves drift on the water out to `LEAF_REACH_M` from its shore (a crown's reach and a little drift), on
// `LEAVES_MOST` of it where all the ground about is under crowns shedding (`Spot.shed`): one leaf at most in each cell
// of a lattice `LEAF_CELL_M` wide, `LEAF_LONG_M` long by its own draw, a third as wide, turned its own way, of one of
// the colours of fallen leaves (`leaf_tone`, linear).
const LEAF_REACH_M = 8.0; const LEAVES_MOST = 0.25; const LEAF_CELL_M = 0.14; const LEAF_LONG_M = vec2f(0.05, 0.1);
fn leaf_tone(k: u32) -> vec3f { var t = array<vec3f, 3>(vec3f(0.30, 0.20, 0.05), vec3f(0.24, 0.10, 0.03), vec3f(0.12, 0.08, 0.04)); return t[k]; }
// Floating-leaved herbs float their round leaves once the water is warmer than `PAD_WARM_K` (water lilies leaf in water
// of 15 °C and more; Sculthorpe 1967), over up to `PADS_MOST` of the share they cover, in patches `PAD_PATCH_M`
// across: a pad `PAD_R_M` across by its own draw, its notch a `PAD_NOTCH` of a turn, green and lighter at its rim
// (`PAD_RGB`, `PAD_RIM`). Where the pads are too small to show, they tint the water by `PAD_FAR` of their cover only, so
// a pond seen from afar shows no field of green.
const PAD_WARM_K = vec2f(286.0, 290.0); const PADS_MOST = 0.5;
const PAD_PATCH_M = 9.0; const PAD_CELL_M = 0.45; const PAD_R_M = vec2f(0.12, 0.22); const PAD_NOTCH = 0.08;
const PAD_RGB = vec3f(0.06, 0.16, 0.035); const PAD_RIM = 1.25; const PAD_FAR = 0.3;

// Water `water` (linear) of spot p with what floats on it over it: how much of the pixel each covers, their colours.
fn afloat(water: vec3f, p: Spot) -> vec3f {
  let still = (1.0 - p.sea) * (1.0 - p.mouth) * (1.0 - p.snow);
  if (still <= 0.0 || (p.shed <= 0.0 && p.pads <= 0.0) || unseen(LEAF_CELL_M)) { return water; }
  var out = water;
  let slope = max(p.slope / earth.look.x, 1e-2);
  let leaves = LEAVES_MOST * p.shed * (1.0 - ease(0.0, LEAF_REACH_M, p.depth / slope)) * still;
  let pads = PADS_MOST * p.pads * ease(PAD_WARM_K.x, PAD_WARM_K.y, p.t) * still;
  if (pads > 0.0) {
    let patches = smoothstep(-EVEN_ONE, EVEN_ONE, noise_l(lat_turned(PAD_PATCH_M, vec2f(0.3)), 150).x);
    let held = smoothstep(0.5 - pads, 0.5 + pads, patches) * 2.0 * pads;
    out = floating_pads(out, held, p.e);
  }
  if (leaves > 0.0) { out = floating_leaves(out, leaves, p.e); }
  return out;
}

// Pads over `water` where `held` of the cells hold one (`afloat`), seen at e m a pixel.
fn floating_pads(water: vec3f, held: f32, e: f32) -> vec3f {
  let seen = as_thing(2.0 * PAD_R_M.x, e);
  let mean = mix(water, PAD_RGB * (1.0 + 0.5 * (PAD_RIM - 1.0)), PAD_FAR * held);
  if (seen <= 0.0) { return mean; }
  let l = lat_axes(PAD_CELL_M, vec2f(0.0));
  var out = water;
  for (var j = -1; j <= 1; j++) {
    for (var i = -1; i <= 1; i++) {
      let cell = l.i + vec2i(i, j);
      let x = lowbias(cell, 151);
      if (f32(x & 1023u) / 1024.0 >= held) { continue; }
      let centre = vec2f(f32(i), f32(j)) + vec2f(f32((x >> 10u) & 255u), f32((x >> 18u) & 255u)) / 256.0;
      let r = mix(PAD_R_M.x, PAD_R_M.y, f32(x >> 26u) / 63.0) / PAD_CELL_M;
      let v = l.f - centre;
      let d = length(v);
      let turn = 2.0 * PI * f32(lowbias(cell, 152) & 1023u) / 1024.0;
      let off = atan2(v.y, v.x) - turn;
      let notch = atan2(sin(off), cos(off));
      let edge = e / PAD_CELL_M;
      let a = (1.0 - smoothstep(r - edge, r, d)) * smoothstep(PAD_NOTCH * PI - 0.02, PAD_NOTCH * PI + 0.02, abs(notch));
      out = mix(out, PAD_RGB * mix(1.0, PAD_RIM, d / r), a);
    }
  }
  return mix(mean, out, seen);
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
