// uses: draw.wgsl earth.wgsl
// The map in view: the fields of a window drawn in the view's colours at each pixel of the screen.

@fragment fn fs(v: Varying) -> @location(0) vec4f {
  let h = spline(height_field, v.uv);
  let rise = fwidth(h.y);
  let rgb = in_view(colour(spot_at(v.uv, h, rise)), v.uv);
  let a = draw.alpha * edge_fade(v.uv);
  return vec4f(clamp(rgb / 255.0, vec3f(0.0), vec3f(1.0)) * a, a);
}
