// uses: draw.wgsl water.wgsl style.wgsl crag.wgsl ice.wgsl colour.wgsl earth.wgsl
// A tape's map in view: the fields of its squares drawn in the view's colours at each pixel of the screen.

@fragment fn fs(v: Varying) -> @location(0) vec4f {
  let h = spline(height_field, v.uv);
  let rgb = in_view(colour(spot_at(v.uv, h, metres_a_pixel(v.uv))), v.uv);
  return vec4f(clamp(rgb / 255.0, vec3f(0.0), vec3f(1.0)) * draw.alpha, draw.alpha);
}
