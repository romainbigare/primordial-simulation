// A fan of blades as a tuft of grass or a fern's rosette (cover_field.wgsl `sward`, floor.wgsl `carpet`); its places come
// from the lattice of the pixel (draw.wgsl `lat`).

// A fan of blades (a tuft of grass, a fern's rosette) in cell `l` of stream k, present with probability `fill`, its
// radius from `radius.x` to `radius.y` cell widths (at most 0.35), `blades` lobes round: x the share of the point it
// covers, y its tone (a dark heart, light tips, each blade its own), z its own hue 0..1.
fn fan(l: Lat, k: i32, fill: f32, radius: vec2f, blades: f32) -> vec3f {
  let h = lowbias(l.i, k);
  let u = vec4f(f32(h & 255u), f32((h >> 8u) & 255u), f32((h >> 16u) & 255u), f32(h >> 24u)) / 255.0;
  if (u.w >= fill) { return vec3f(0.0, 1.0, 0.5); }
  // Each tuft leans (its cell's shape squeezed by `aspect`), has its own count of blades and unequal ones: two lobe
  // patterns of different counts, so no two look alike and none is a star.
  var d = (l.f - (vec2f(0.5) + (u.xy - 0.5) * 0.3)) / mix(radius.x, radius.y, u.z);
  d.y /= 0.6 + 0.4 * fract(u.x * 13.7);
  let t = length(d);
  let count = floor(0.6 * blades + 0.4 * blades * u.w / fill);
  let a = atan2(d.y, d.x);
  let lobe = pow(0.5 + 0.5 * cos(count * a + 6.2832 * u.x), 2.5) * (0.6 + 0.4 * (0.5 + 0.5 * cos((count + 3.0) * a + 5.0 * u.y)));
  let reach = 0.3 + 0.7 * lobe;
  return vec3f(1.0 - smoothstep(0.75 * reach, reach, t), mix(0.55, 1.25, smoothstep(0.0, 0.9, t)) * (0.8 + 0.4 * lobe), u.y);
}
