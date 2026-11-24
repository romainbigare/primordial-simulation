// uses: draw.wgsl ragged.wgsl water.wgsl style.wgsl land.wgsl rock.wgsl cover_field.wgsl no_grass.wgsl sprites.wgsl snag_sprites.wgsl heights.wgsl tex.wgsl plant.wgsl cells.wgsl mark.wgsl models.wgsl fire.wgsl snag.wgsl
// The marks of the plant cover's cells in view, decided once a frame (lib/mark.wgsl `decide`): a texel a cell, each
// lattice's cells in rows of their own (`cover.rows`), drawn before the map (web/src/textures.ts `prepare`), each of
// its words (lib/cells.wgsl `mark_word`) into one of the marks' textures, all four at once.

struct MarkWords {
  @location(0) kind: vec4f,
  @location(1) shows: vec4f,
  @location(2) snow: vec4f,
  @location(3) box: vec4f,
}

@fragment fn fs(v: Varying) -> MarkWords {
  // (The channels' bends as the ground under the marks draws them, land.wgsl `bends_e`.)
  bends_e = cover.seen.x;
  let t = vec2i(floor(v.px));
  // The lattice whose rows hold the texel: the last that starts at or above it.
  var l = 0;
  for (var k = 1; k < COVER_LATTICES; k++) { if (f32(t.y) >= cover.rows[k]) { l = k; } }
  let m = decide(l, vec2f(f32(t.x - MARK_MARGIN), f32(t.y - i32(cover.rows[l]) - MARK_MARGIN)));
  return MarkWords(mark_word(m, 0), mark_word(m, 1), mark_word(m, 2), mark_word(m, 3));
}
