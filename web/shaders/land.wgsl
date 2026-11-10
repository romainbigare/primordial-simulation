// uses: draw.wgsl water.wgsl colour.wgsl land.wgsl cover_field.wgsl
// The live map in view: at each pixel of the screen the ground from its tiles and the state from the cells' table
// (land.wgsl), in the view's colours.

// A channel's bed is a parabola across it, whose mean depth is two thirds of its depth at the middle.
const BED_MIDDLE = 1.5;

// What is at `at`, m from the map's north-west corner, seen at `e` m a pixel: its ground and its state (`state_at`), the
// ground's own colour and the sea blended between the squares' centres, the plants' field (lib/cover_field.wgsl) and
// the channel nearest it.
fn land_spot(at: vec2f, e: f32) -> Spot {
  let look = earth.look;
  let cs = corners(at / land.map.x);
  let g = ground_at(at, true);
  let st = state_at(at, g, cs);
  let surface = blended(SURFACE, cs);
  var p: Spot;
  p.depth = st.depth;
  p.over = st.over;
  p.sea = surface.w;
  p.wet = st.wet;
  p.t = st.t;
  p.snow = st.snow;
  p.ground = surface.xyz;
  p.cover = cover_field(at, e);
  p.z = g.h;
  p.slope = length(g.slope) * look.x;
  p.shade = hill_shade(g.slope, EXAGGERATE * look.x);
  p.sand = SAND_SHARE * p.sea * (1.0 - ease(0.0, look.z, p.over));
  // A channel where the point lies nearer its centre line than half its width: its discharge is its drained area times
  // the volume the cells pass on per m² drained (flow/channel.rs; no more than the tiles' cap), its share of a pixel what
  // the pixel's footprint covers. Its bed is cut into the ground as a parabola as wide as it is, as deep at its middle as
  // its mean depth makes it (`BED_MIDDLE`): the water is deepest there, and its banks catch the light.
  let runoff = land.time.w;
  let q = min(blended(SOIL, cs).z, runoff);
  p.river = 0.0;
  p.river_depth = 0.0;
  if (q > 0.0 && g.ratio < 1.0) {
    let k = pow(g.area, land.channel.y);
    let half = 0.5 * land.channel.x * pow(q, land.channel.y) * k;
    let to_m = 0.5 * land.channel.x * pow(runoff, land.channel.y) * k;
    let d = g.ratio * to_m;
    p.river = tent((half - d) / e) - tent((-half - d) / e);
    let x = min(d / half, 1.0);
    let deepest = BED_MIDDLE * land.channel.z * pow(q * g.area, land.channel.w);
    p.river_depth = deepest * (1.0 - x * x);
    // The bed's slope across the channel, d(depth)/d(distance) along the distance's own slope; where the channel is
    // narrower than a pixel its banks do not show.
    let bank = 2.0 * deepest * x / half * to_m * g.ratio_slope * select(0.0, 1.0, x < 1.0) * ease(e, 2.0 * e, half);
    p.slope = length(g.slope + bank) * look.x;
    p.shade = hill_shade(g.slope + bank, EXAGGERATE * look.x);
  }
  p.at = at;
  p.e = e;
  return p;
}

@fragment fn fs(v: Varying) -> @location(0) vec4f {
  let at = earth.place.xy + v.uv * earth.place.zw;
  let cs = corners(at / land.map.x);
  let spot = land_spot(at, metres_a_pixel(v.uv));
  let rgb = in_view_of(colour(spot), blended(VIEW, cs).xy, blended(TRUTH, cs).xyz);
  return vec4f(clamp(rgb / 255.0, vec3f(0.0), vec3f(1.0)) * draw.alpha, draw.alpha);
}
