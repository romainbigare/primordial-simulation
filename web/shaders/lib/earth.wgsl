// The fields of a window of the map (web/src/fields.ts), a texel each, read at a point between the texels' centres
// as what is there (`Spot`, colour.wgsl): the map of the squares.

// The ground's height, m; the liquid's surface over it, m (below it: negative); the ground's slope east and south.
@group(0) @binding(2) var height_field: texture_2d<f32>;
// The ice, m; the ground's temperature, K; the share that is the sea's; the share that is a river.
@group(0) @binding(3) var matter_field: texture_2d<f32>;
// The true colour (red, green, blue), 0..1.
@group(0) @binding(4) var byte_field: texture_2d<f32>;
// The ground's own colour (red, green, blue), 0..1.
@group(0) @binding(8) var ground_field: texture_2d<f32>;
// The byte of the layer a view draws, and of the worn paths, 0..1.
@group(0) @binding(5) var view_field: texture_2d<f32>;
@group(0) @binding(6) var smooth_sampler: sampler;
// How wet the ground is, 0..1; the depth of the river through it, m; the share of the ground the plant cover leaves
// bare.
@group(0) @binding(7) var inside_field: texture_2d<f32>;
// The reflectance a tape's plants are drawn in: a green leaf's in the visible, about 0.05 red, 0.12 green and 0.04 blue
// (Gates and others 1965; Knipling 1970).
const TAPE_FOLIAGE = vec3f(0.05, 0.12, 0.04);
// Field `t` (32-bit floats, which a sampler may not blend) at uv, blended between its texels' centres, its edge texels
// held beyond them.
fn blend(t: texture_2d<f32>, uv: vec2f) -> vec4f {
  let size = vec2i(textureDimensions(t));
  let p = uv * vec2f(size) - 0.5;
  let i = vec2i(floor(p));
  let f = p - floor(p);
  let a = clamp(i, vec2i(0), size - 1);
  let b = clamp(i + 1, vec2i(0), size - 1);
  let top = mix(textureLoad(t, a, 0), textureLoad(t, vec2i(b.x, a.y), 0), f.x);
  let bottom = mix(textureLoad(t, vec2i(a.x, b.y), 0), textureLoad(t, b, 0), f.x);
  return mix(top, bottom, f.y);
}

// Field `t` at uv through its texels' centres as a Catmull-Rom spline (it keeps each texel's value, so a lake of one
// texel stays, and its slope runs on across the texels' edges, so the light on a slope drawn over many pixels a texel
// shows no creases there), its edge texels held beyond them.
fn spline(t: texture_2d<f32>, uv: vec2f) -> vec4f {
  let size = vec2i(textureDimensions(t));
  let p = uv * vec2f(size) - 0.5;
  let i = vec2i(floor(p));
  let f = p - floor(p);
  let f2 = f * f;
  let f3 = f2 * f;
  // The weights of the texels from i - 1 to i + 2 along each axis.
  var w = array<vec2f, 4>(0.5 * (-f3 + 2.0 * f2 - f), 0.5 * (3.0 * f3 - 5.0 * f2 + 2.0), 0.5 * (-3.0 * f3 + 4.0 * f2 + f), 0.5 * (f3 - f2));
  var sum = vec4f(0.0);
  for (var y = 0; y < 4; y++) {
    var row = vec4f(0.0);
    for (var x = 0; x < 4; x++) { row += w[x].x * textureLoad(t, clamp(i + vec2i(x - 1, y - 1), vec2i(0), size - 1), 0); }
    sum += w[y].y * row;
  }
  return sum;
}
// The spot at uv of the fields, whose ground's height and slope are those of `h` (`spline` of `height_field`), seen
// at `e` m a pixel.
fn spot_at(uv: vec2f, h: vec4f, e: f32) -> Spot {
  let m = blend(matter_field, uv);
  let inside = blend(inside_field, uv);
  let look = earth.look;
  var s: Spot;
  // The liquid over the ground blended between the texels, not as a spline: a spline would overshoot where a cell
  // that holds liquid meets one that holds none.
  let water = blend(height_field, uv).y;
  s.depth = max(water, 0.0);
  s.over = max(-water, 0.0);
  s.sea = m.z;
  s.wet = inside.x;
  s.t = m.y;
  s.snow = snow_of(m.x, s.t, look.y);
  s.ground = textureSampleLevel(ground_field, smooth_sampler, uv, 0.0).xyz;
  // A tape holds no kinds of cover: its plants are one foliage, as closed as their capture area closes the ground.
  s.cover = vec4f(TAPE_FOLIAGE, 1.0) * (1.0 - inside.z);
  s.z = h.x;
  s.slope = length(h.zw) * look.x;
  s.shade = hill_shade(h.zw, EXAGGERATE * look.x);
  s.sand = beach(s.sea, s.over, length(h.zw), look.z);
  s.river = m.w;
  s.river_depth = inside.y;
  s.at = earth.place.xy + uv * earth.place.zw;
  s.e = e;
  return s;
}
// Shaded colour `c` at uv in the view drawn (`in_view_of`).
fn in_view(c: Shaded, uv: vec2f) -> vec3f {
  let o = textureSampleLevel(view_field, smooth_sampler, uv, 0.0);
  return in_view_of(c, o.xy, textureSampleLevel(byte_field, smooth_sampler, uv, 0.0).xyz);
}
