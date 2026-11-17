// uses: draw.wgsl cells.wgsl
// Of each cell of the plant cover in view, the cells about it whose marks reach into it (lib/cells.wgsl `reach_of`),
// worked out once a frame after the marks are decided (marks.wgsl; web/src/textures.ts `prepare`), a texel a cell as
// the marks' are, so a pixel of the cover (cover.wgsl) looks only at those.

@fragment fn fs(v: Varying) -> @location(0) vec4f {
  let t = vec2i(floor(v.px));
  // The lattice whose rows hold the texel: the last that starts at or above it.
  var l = 0;
  for (var k = 1; k < COVER_LATTICES; k++) { if (f32(t.y) >= cover.rows[k]) { l = k; } }
  return word_bytes(reach_of(l, vec2f(f32(t.x - MARK_MARGIN), f32(t.y - i32(cover.rows[l]) - MARK_MARGIN))));
}
