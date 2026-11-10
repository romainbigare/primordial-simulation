// uses: draw.wgsl water.wgsl colour.wgsl land.wgsl
// The live map in view: at each pixel of the screen the ground from its tiles and the state from the cells' table
// (land.wgsl), in the view's colours.

@fragment fn fs(v: Varying) -> @location(0) vec4f {
  let at = earth.place.xy + v.uv * earth.place.zw;
  let cs = corners(at / land.map.x);
  let rgb = in_view_of(colour(land_spot(at, ground_at(at), metres_a_pixel(v.uv))), blended(VIEW, cs).xy, blended(TRUTH, cs).xyz);
  return vec4f(clamp(rgb / 255.0, vec3f(0.0), vec3f(1.0)) * draw.alpha, draw.alpha);
}
