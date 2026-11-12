import os
os.chdir('/home/user/primordial-simulation/.claude/worktrees/agent-a2f9648fe0863f025')
p = 'web/shaders/lib/colour.wgsl'
s = open(p).read()
old = '''// The ground spot p's colour reads (`ground_of`): with its snow over the liquid, without it under the liquid (frozen
// liquid lies on the surface); black where it is not read. Each pixel's colours read it at one place in the code
// (land.wgsl `fs`), so its rock and stones are compiled once.
fn ground_for(p: Spot) -> Ground {
  if (!reads_ground(p)) { return Ground(vec3f(0.0), 1.0, BARE); }
  return ground_of(p, select(p.snow, 0.0, p.depth > 0.0));
}'''
new = '''// The ground spot p's colour reads (`ground_of`), the bed of running water where `running` (always read): with its
// snow over the liquid, without it under the liquid (frozen liquid lies on the surface); black where it is not read.
// Each pixel's colours read it at one place in the code (land.wgsl `fs`), so its rock and stones are compiled once.
fn ground_for(p: Spot, running: bool) -> Ground {
  if (!running && !reads_ground(p)) { return Ground(vec3f(0.0), 1.0, BARE); }
  return ground_of(p, select(p.snow, 0.0, p.depth > 0.0 || running));
}'''
assert old in s
s = s.replace(old, new)
s = s.replace("fn colour(p: Spot) -> Shaded { return colour_on(p, ground_for(p)); }", "fn colour(p: Spot) -> Shaded { return colour_on(p, ground_for(p, false)); }")
s = s.replace("// The colour of spot p over its ground g (`ground_for`).", "// The colour of spot p over its ground g (`ground_for`), lit.")
open(p, 'w').write(s)

p = 'web/shaders/land.wgsl'
s = open(p).read()
i = s.index("// The colour of a channel's water at spot `p`")
new = '''// The colour of a channel's water at spot `p` (`land_spots`) over its bed `g` (colour.wgsl `ground_for`), a reflectance
// in linear light: its bed, soaked bare ground (washed sand and gravel: the flow takes the plants) under its film
// (`BED_FILM`), seen through running water as deep as it is there (water.wgsl); snow lies on it as on standing liquid
// (colour.wgsl `colour_on`), and the true view shades its bed as far as it shows.
fn river_on(p: Spot, g: Ground) -> Shaded {
  let lit = light(1.0 + (p.shade - 1.0) * (0.8 + 0.3 * p.snow), p.slope);
  let bed = BED_FILM * soil(g, 1.0, lit);
  let ripple = ripples(p.at, p.e) * (1.0 - p.snow);
  // In the style a channel's water is a lake's (style.wgsl `lake_seen`), lighter over its shallow banks.
  var water = seen_through(bed, p.depth, ripple, RUNNING_ATTENUATION, DEEP_WATER);
  if (styled()) { water = water_painted(bed, lake_seen(p.depth, 0.0, p.e, RIVER_RIM_M), ripple); }
  let rgb = mix(water, SNOW, p.snow * 0.85);
  return Shaded(rgb, 1.0 + (lit - 1.0) * exp(-2.0 * RUNNING_ATTENUATION.g * p.depth), RIVER);
}

// The colours a pixel mixes, each over the last by the share of the pixel it covers: its ground (`LAYER_GROUND`); the
// bars of a channel's bed, washed gravel, bare, as high over the water as it is deep (`LAYER_BARS`); where the edge of
// standing liquid crosses the pixel, the other side of it (`LAYER_EDGE`); a channel's water (`LAYER_RIVER`). One loop
// draws them all, so the colour's code (its ground, rock and stones) is compiled once.
const LAYER_GROUND = 0; const LAYER_BARS = 1; const LAYER_EDGE = 2; const LAYER_RIVER = 3; const LAYERS = 4;

@fragment fn fs(v: Varying) -> @location(0) vec4f {
  let at = earth.place.xy + v.uv * earth.place.zw;
  let cs = corners(at / land.map.x);
  let spots = land_spots(at, metres_a_pixel(v.uv));
  var c = Shaded(vec3f(0.0), 1.0, BARE);
  for (var k = LAYER_GROUND; k < LAYERS; k++) {
    var p = spots.dry;
    var share = 1.0;
    if (k == LAYER_BARS) {
      share = spots.bars;
      p.sand = 1.0;
      p.cover = vec4f(0.0);
      p.over = spots.wet.depth;
    } else if (k == LAYER_EDGE) {
      share = select(0.0, select(1.0 - spots.pool, spots.pool, spots.dry.depth <= 0.0), spots.pool > 0.0 && spots.pool < 1.0);
      p.depth = select(FILM_M, 0.0, spots.dry.depth > 0.0);
    } else if (k == LAYER_RIVER) {
      share = spots.share;
      p = spots.wet;
    }
    if (share <= 0.0) { continue; }
    let running = k == LAYER_RIVER;
    let g = ground_for(p, running);
    var o: Shaded;
    if (running) { o = river_on(p, g); } else { o = colour_on(p, g); }
    c = Shaded(mix(c.rgb, o.rgb, share), mix(c.m, o.m, share), select(c.kind, o.kind, k != LAYER_BARS && share > 0.5));
  }
  // The true colour is the cells', and the bed's where it shows.
  let truth = mix(blended(TRUTH, cs).xyz, spots.dry.bed, spots.dry.rock);
  let rgb = in_view_of(c, blended(VIEW, cs).xy, truth);
  return vec4f(clamp(rgb / 255.0, vec3f(0.0), vec3f(1.0)) * draw.alpha, draw.alpha);
}
'''
s = s[:i] + new
open(p, 'w').write(s)
