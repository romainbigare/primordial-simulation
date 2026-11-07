// uses: draw.wgsl
// A picture the renderer drew before into a texture (the map drawn at fewer pixels than the screen's while frames are
// slow, or drawn anew at the screen's own a tile at a time once the view rests), drawn onto the screen.

@group(0) @binding(1) var picture: texture_2d<f32>;
@group(0) @binding(2) var picture_sampler: sampler;

@fragment fn fs(v: Varying) -> @location(0) vec4f {
  return textureSampleLevel(picture, picture_sampler, v.uv, 0.0);
}
