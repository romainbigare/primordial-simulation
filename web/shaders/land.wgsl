// uses: draw.wgsl water.wgsl colour.wgsl land.wgsl
// The live map in view: at each pixel of the screen the ground from its tiles and the state from the cells' table
// (land.wgsl), in the view's colours.

// What is at `at`, m from the map's north-west corner, seen at `e` m a pixel: its ground and its state (`state_at`), the
// cover's byte, bare share and sea blended between the squares' centres, and the channel nearest it.
fn land_spot(at: vec2f, e: f32) -> Spot {
  let look = earth.look;
  let cs = corners(at / land.map.x);
  let g = ground_at(at, true);
  let st = state_at(at, g, cs);
  let cover = blended(COVER, cs);
  var p: Spot;
  p.depth = st.depth;
  p.over = st.over;
  p.sea = cover.z;
  p.wet = st.wet;
  p.bare = cover.y;
  p.t = st.t;
  p.snow = st.snow;
  p.plants = 255.0 * cover.x;
  p.z = g.h;
  p.slope = length(g.slope) * look.x;
  p.shade = hill_shade(g.slope, EXAGGERATE * look.x);
  p.sand = SAND_SHARE * p.sea * (1.0 - ease(0.0, look.z, p.over));
  // A channel where the point lies nearer its centre line than half its width: its discharge is its drained area times
  // the volume the cells pass on per m² drained (flow/channel.rs), its share of a pixel what the pixel's footprint covers.
  let q = blended(SOIL, cs).z;
  p.river = 0.0;
  p.river_depth = 0.0;
  if (q > 0.0 && g.area > 0.0) {
    let k = pow(g.area, land.channel.y);
    let half = 0.5 * land.channel.x * pow(q, land.channel.y) * k;
    let d = g.ratio * k;
    p.river = tent((half - d) / e) - tent((-half - d) / e);
    p.river_depth = land.channel.z * pow(q * g.area, land.channel.w);
  }
  p.at = at;
  p.e = e;
  return p;
}

@fragment fn fs(v: Varying) -> @location(0) vec4f {
  let at = earth.place.xy + v.uv * earth.place.zw;
  let cs = corners(at / land.map.x);
  let rgb = in_view_of(colour(land_spot(at, metres_a_pixel(v.uv))), blended(VIEW, cs).xy, blended(TRUTH, cs).xyz);
  return vec4f(clamp(rgb / 255.0, vec3f(0.0), vec3f(1.0)) * draw.alpha, draw.alpha);
}
