// What floats on still standing liquid up close (colour.wgsl `colour_on`): the leaves the woody crowns about it shed
// this month, and the round floating leaves of the herbs that stand in it deeper than they are tall (`Spot.pads`: the
// core stands them on its ponds' ground down to their line). A lake's sheltered liquid, not the sea's waves
// nor a channel's flow, holds them.

// Fallen leaves drift on the water out to `LEAF_REACH_M` from its shore (a crown's reach and a little drift), on
// `LEAVES_MOST` of it where all the ground about is under crowns shedding (`Spot.shed`): one leaf at most in each cell
// of a lattice `LEAF_CELL_M` wide, `LEAF_LONG_M` long by its own draw, a third as wide, turned its own way, of one of
// the colours of fallen leaves (`leaf_tone`, linear).
const LEAF_REACH_M = 8.0; const LEAVES_MOST = 0.25; const LEAF_CELL_M = 0.14; const LEAF_LONG_M = vec2f(0.05, 0.1);
fn leaf_tone(k: u32) -> vec3f { var t = array<vec3f, 3>(vec3f(0.30, 0.20, 0.05), vec3f(0.24, 0.10, 0.03), vec3f(0.12, 0.08, 0.04)); return t[k]; }
// Floating-leaved herbs float their round leaves once the water is warmer than `PAD_WARM_K` (water lilies leaf in water
// of 15 °C and more; Sculthorpe 1967), over up to `PADS_MOST` of the share they cover, in patches `PAD_PATCH_M`
// across: a pad `PAD_R_M` across by its own draw, one of the scanned floating leaves (sprites.wgsl `PAD_FROM`) as the
// map sees it, mirrored by its own draw; in water warmer than `PAD_BLOOM_K` (all from .y) `PAD_BLOOMS` of them hold a
// flower (`PAD_FLOWER_FROM`) `PAD_FLOWER_M` across standing on its middle (water lilies flower in the warmest months:
// from memory). Where the pads are too small to show, they tint the water by `PAD_FAR` of their cover only, in their
// mean colour (`PAD_RGB`), so a pond seen from afar shows no field of green.
const PAD_WARM_K = vec2f(286.0, 290.0); const PADS_MOST = 0.5;
const PAD_PATCH_M = 9.0; const PAD_CELL_M = 0.45; const PAD_R_M = vec2f(0.12, 0.22);
const PAD_BLOOM_K = vec2f(291.0, 295.0); const PAD_BLOOMS = 0.1; const PAD_FLOWER_M = 0.14;
const PAD_RGB = vec3f(0.06, 0.16, 0.035); const PAD_FAR = 0.3;

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
    out = floating_pads(out, held, PAD_BLOOMS * ease(PAD_BLOOM_K.x, PAD_BLOOM_K.y, p.t), p.e);
  }
  if (leaves > 0.0) { out = floating_leaves(out, leaves, p.e); }
  return out;
}

// Pads over `water` where `held` of the cells hold one and `blooms` of those a flower (`afloat`), seen at e m a pixel.
fn floating_pads(water: vec3f, held: f32, blooms: f32, e: f32) -> vec3f {
  let seen = as_thing(2.0 * PAD_R_M.x, e);
  let mean = mix(water, PAD_RGB, PAD_FAR * held);
  if (seen <= 0.0 || !models_ready()) { return mean; }
  let l = lat_axes(PAD_CELL_M, vec2f(0.0));
  var out = water;
  for (var j = -1; j <= rolled(1); j++) {
    for (var i = -1; i <= 1; i++) {
      let cell = l.i + vec2i(i, j);
      let x = lowbias(cell, 151);
      if (f32(x & 1023u) / 1024.0 >= held) { continue; }
      let centre = vec2f(f32(i), f32(j)) + vec2f(f32((x >> 10u) & 255u), f32((x >> 18u) & 255u)) / 256.0;
      let d = 2.0 * mix(PAD_R_M.x, PAD_R_M.y, f32(x >> 26u) / 63.0) / PAD_CELL_M;
      let own = lowbias(cell, 152);
      let s = PAD_FROM + i32(own % u32(PAD_N));
      let v = l.f - centre;
      var uv = vec2f(v.x / d + 0.5, 0.5 + v.y / (d * model_h(s) / model_w(s)));
      if (((own >> 8u) & 1u) == 1u) { uv.x = 1.0 - uv.x; }
      if (all(uv >= vec2f(0.0)) && all(uv <= vec2f(1.0))) {
        let t = model_texel(s, uv, e / (d * PAD_CELL_M));
        out = mix(out, t.rgb, t.a);
      }
      // Its flower stands on its middle, its foot there.
      if (f32((own >> 9u) & 1023u) / 1024.0 >= blooms) { continue; }
      let w = PAD_FLOWER_M / PAD_CELL_M;
      let fuv = vec2f(v.x / w + 0.5, 1.0 + v.y / (w * model_h(PAD_FLOWER_FROM) / model_w(PAD_FLOWER_FROM)));
      if (all(fuv >= vec2f(0.0)) && all(fuv <= vec2f(1.0))) {
        let t = model_texel(PAD_FLOWER_FROM, fuv, e / PAD_FLOWER_M);
        out = mix(out, t.rgb, t.a);
      }
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
  for (var j = -1; j <= rolled(1); j++) {
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
