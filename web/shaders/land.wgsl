// uses: draw.wgsl water.wgsl style.wgsl colour.wgsl land.wgsl cover_field.wgsl
// The live map in view: at each pixel of the screen the ground from its tiles and the state from the cells' table
// (land.wgsl), in the view's colours.

// A channel shows as a line of at least `LINE_PX` css pixels once its discharge passes the discharge that shows in the
// view (`land.time.w` times the metres a css pixel takes: tape/src/ground/channels.rs `SHOW_M`), as a map draws its
// rivers; wider as its discharge grows past that, as the channel law widens a channel, up to `LINE_MOST` times; it
// fades in from `LINE_FADE` times less, so no channel springs up as the view zooms.
const LINE_PX = 1.2;
const LINE_MOST = 2.5;
const LINE_FADE = 4.0;

// What is at a point: the ground without the channel's water (`dry`), the channel's water (`wet`) and the share of the
// pixel that water covers.
struct Spots { dry: Spot, wet: Spot, share: f32 }

// What is at `at`, m from the map's north-west corner, seen at `e` m a pixel: its ground and its state (`state_on`), the
// ground's own colour and the sea blended between the squares' centres, the plants' field (lib/cover_field.wgsl); and
// its channel (`bed_at`), from its own sample's discharge. The channel's bed is cut into the ground, so where it runs
// into standing liquid that liquid takes it over at its shore at the same depth. Beside it the ground is wet by the
// capillary fringe over its water, which rises from the water's edge as the valley's side does, at least as steeply as
// the channel's own banks. Its water (`river_colour`) is as deep as its bed lies there; a pixel wider than the channel
// sees its mean depth.
fn land_spots(at: vec2f, e: f32) -> Spots {
  let look = earth.look;
  let cs = corners(at / land.map.x);
  let g = ground_at(at, true);
  let q = discharge_at(g);
  let bed = bed_at(g, q);
  let st = state_on(at, g, cs, bed.depth);
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
  p.sand = beach(p.sea, g.h - earth.view.w, length(g.slope), look.z);
  p.river = 0.0;
  p.river_depth = 0.0;
  p.at = at;
  p.e = e;
  var out = Spots(p, p, 0.0);
  if (st.depth > 0.0 || bed.half <= 0.0) { return out; }
  out.dry.over = min(p.over, max(g.d - bed.half, 0.0) * max(length(g.slope), bed.mean / bed.half));
  let css = e * land.samples.w;
  let shows = land.time.w * css;
  let line = 0.5 * LINE_PX * css * min(pow(q / shows, land.channel.y), LINE_MOST) * ease(shows / LINE_FADE, shows, q);
  let drawn = max(bed.half, line);
  out.share = tent((drawn - g.d) / e) - tent((-drawn - g.d) / e);
  let resolved = ease(e, 2.0 * e, bed.half);
  let x = min(g.d / bed.half, 1.0);
  out.wet.depth = mix(bed.mean, bed.depth, resolved);
  out.wet.sand = 0.0;
  out.wet.cover = vec4f(0.0);
  // The bed's slope across the channel, d(depth)/d(distance) along the distance's own slope; where the channel is
  // narrower than a pixel its banks do not show.
  let bank = 2.0 * BED_MIDDLE * bed.mean * x / bed.half * g.d_slope * select(0.0, 1.0, x < 1.0) * resolved;
  out.wet.slope = length(g.slope + bank) * look.x;
  out.wet.shade = hill_shade(g.slope + bank, EXAGGERATE * look.x);
  return out;
}

// A stream's bed sends back this share of the light soaked bare ground does: a film of algae and fine dead matter
// coats the stones under running water, and a bed sends back about 5% of the light against about 20% for soaked sand
// (Legleiter and others 2004; from memory).
const BED_FILM = 0.3;

// The colour of a channel's water at spot `p` (`land_spots`), a reflectance in linear light: its bed, soaked bare
// ground (washed sand and gravel: the flow takes the plants) under its film (`BED_FILM`), seen through running water as
// deep as it is there (water.wgsl); snow lies on it as on standing liquid (colour.wgsl `colour`), and the true view
// shades its bed as far as it shows.
fn river_colour(p: Spot) -> Shaded {
  let lit = light(1.0 + (p.shade - 1.0) * (0.8 + 0.3 * p.snow), p.slope);
  let bed = BED_FILM * soil(ground_of(p, 0.0), 1.0, lit);
  let rgb = mix(seen_through(bed, p.depth, ripples(p.at, p.e) * (1.0 - p.snow), RUNNING_ATTENUATION, DEEP_WATER), SNOW, p.snow * 0.85);
  return Shaded(rgb, 1.0 + (lit - 1.0) * exp(-2.0 * RUNNING_ATTENUATION.g * p.depth), RIVER);
}

@fragment fn fs(v: Varying) -> @location(0) vec4f {
  let at = earth.place.xy + v.uv * earth.place.zw;
  let cs = corners(at / land.map.x);
  let spots = land_spots(at, metres_a_pixel(v.uv));
  var c = colour(spots.dry);
  if (spots.share > 0.0) {
    let w = river_colour(spots.wet);
    c = Shaded(mix(c.rgb, w.rgb, spots.share), mix(c.m, w.m, spots.share), select(c.kind, RIVER, spots.share > 0.5));
  }
  let rgb = in_view_of(c, blended(VIEW, cs).xy, blended(TRUTH, cs).xyz);
  return vec4f(clamp(rgb / 255.0, vec3f(0.0), vec3f(1.0)) * draw.alpha, draw.alpha);
}
