// uses: draw.wgsl
// What lies under the map: the page's map background, and the map's rectangle in grey casting a soft shadow on it.

struct Backdrop {
  // The background (display colour, 0..1); the map's rectangle, px: x, y, w, h.
  bg: vec4f, map: vec4f,
  // The shadow: how far it falls south, px; its blur (the deviation of a Gaussian), px; how dark it is at most.
  shadow: vec4f,
}
@group(0) @binding(1) var<uniform> backdrop: Backdrop;

const GREY = vec3f(0.533);

// The share of a Gaussian of deviation 1 below x (an approximation of the error function: Winitzki 2008).
fn below(x: f32) -> f32 {
  let y = x * 0.70710678;
  let a = 0.147;
  let y2 = y * y;
  return 0.5 + 0.5 * sign(y) * sqrt(1.0 - exp(-y2 * (1.2732395 + a * y2) / (1.0 + a * y2)));
}

@fragment fn fs(v: Varying) -> @location(0) vec4f {
  let r = backdrop.map;
  let s = backdrop.shadow;
  let q = v.px - vec2f(0.0, s.x);
  let shade = (below((q.x - r.x) / s.y) - below((q.x - r.x - r.z) / s.y)) * (below((q.y - r.y) / s.y) - below((q.y - r.y - r.w) / s.y));
  var rgb = backdrop.bg.rgb * (1.0 - s.z * shade);
  let inside = v.px.x >= r.x && v.px.x < r.x + r.z && v.px.y >= r.y && v.px.y < r.y + r.w;
  if (inside) { rgb = GREY; }
  return vec4f(rgb, 1.0);
}
