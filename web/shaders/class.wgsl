// uses: draw.wgsl earth.wgsl
// The map class of each square (a pixel a square, at its centre) as its red byte, for the page to read back.

@fragment fn fs(v: Varying) -> @location(0) vec4f {
  let h = spline(height_field, v.uv);
  let rise = fwidth(h.y);
  return vec4f(f32(colour(spot_at(v.uv, h, rise)).kind) / 255.0, 0.0, 0.0, 1.0);
}
