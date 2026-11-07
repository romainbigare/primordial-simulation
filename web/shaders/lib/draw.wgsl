// A draw of the map's renderer (web/src/gpu/): a rectangle of the target filled by a program's fragment stage. Every
// program reads it at binding 0 and has this vertex stage: four corners as a triangle strip.

struct Draw {
  // The target's size, px; the sign of its rows (-1 where the back end keeps a target's rows bottom up, WebGL2 drawing
  // into a texture, so every target's first row is its top); how strongly the draw covers what is under it, 0..1.
  size: vec2f, flip: f32, alpha: f32,
  // The rectangle drawn, px from the target's top-left corner: x, y, w, h.
  rect: vec4f,
  // What the rectangle shows of its source, in the source's uv: from (x, y) at its top-left corner to (z, w).
  src: vec4f,
}
@group(0) @binding(0) var<uniform> draw: Draw;

struct Varying {
  @builtin(position) pos: vec4f,
  // The point of the source, uv, and of the target, px from its top-left corner.
  @location(0) uv: vec2f,
  @location(1) px: vec2f,
}

@vertex fn vs(@builtin(vertex_index) i: u32) -> Varying {
  let corner = vec2f(f32(i & 1u), f32(i >> 1u));
  let px = draw.rect.xy + corner * draw.rect.zw;
  var v: Varying;
  v.pos = vec4f(2.0 * px.x / draw.size.x - 1.0, (1.0 - 2.0 * px.y / draw.size.y) * draw.flip, 0.0, 1.0);
  v.uv = mix(draw.src.xy, draw.src.zw, corner);
  v.px = px;
  return v;
}

// 0 at a, 1 at b, smooth between (a may be above b).
fn ease(a: f32, b: f32, x: f32) -> f32 {
  let t = clamp((x - a) / (b - a), 0.0, 1.0);
  return t * t * (3.0 - 2.0 * t);
}

// A random word of the grid cell c and the stream k, fixed in the world (lowbias32, Wellons 2018).
fn lowbias(c: vec2i, k: i32) -> u32 {
  var x = (u32(c.x) * 0x9E3779B1u) ^ (u32(c.y) * 0x85EBCA77u) ^ (u32(k) * 0xC2B2AE3Du);
  x ^= x >> 16u; x *= 0x7feb352du; x ^= x >> 15u; x *= 0x846ca68bu; x ^= x >> 16u;
  return x;
}
