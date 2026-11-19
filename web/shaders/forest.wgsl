// uses: draw.wgsl
// The plant cover's marks from the tiles they were drawn into once (web/src/forest.ts): tiles fixed in the world, each
// the cover program's picture (cover.wgsl) of its square of ground at its band's metres a texel, kept with a gutter of a
// texel each side so a pixel between two tiles reads the same texels from either. A pixel reads the tile its point lies
// in from the page of the tiles in view; where a tile is not drawn yet, nothing (the live cover draws there).

struct Forest {
  // Metres a pixel of the target (x) and the side of a tile, m (y); the page's tiles across and down (zw).
  size: vec4f,
  // The corner of the page's first tile, m from the target's pixel `origin.zw` (xy).
  origin: vec4f,
}
@group(0) @binding(1) var<uniform> forest: Forest;
// Of each tile in view, the layer of `forest_tiles` it is drawn in (x; -1: not drawn yet).
@group(0) @binding(2) var forest_page: texture_2d<f32>;
@group(0) @binding(3) var forest_tiles: texture_2d_array<f32>;
@group(0) @binding(4) var forest_sampler: sampler;

// A tile's texels inside its gutter along each side, and of the gutter (web/src/forest.ts `TILE`, `GUTTER`).
const TILE = 256.0; const GUTTER = 1.0;

@fragment fn fs(v: Varying) -> @location(0) vec4f {
  let at = ((v.px - forest.origin.zw) * forest.size.x - forest.origin.xy) / forest.size.y;
  let slot = vec2i(floor(at));
  if (any(slot < vec2i(0)) || any(slot >= vec2i(forest.size.zw))) { return vec4f(0.0); }
  let layer = textureLoad(forest_page, slot, 0).x;
  if (layer < 0.0) { return vec4f(0.0); }
  let uv = ((at - vec2f(slot)) * TILE + GUTTER) / (TILE + 2.0 * GUTTER);
  return textureSampleLevel(forest_tiles, forest_sampler, uv, i32(layer), 0.0);
}
