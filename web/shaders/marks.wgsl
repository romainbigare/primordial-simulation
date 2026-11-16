// uses: draw.wgsl water.wgsl style.wgsl land.wgsl rock.wgsl cover_field.wgsl no_grass.wgsl tex.wgsl crown.wgsl mark.wgsl
// The marks of the plant cover's cells in view, decided once a frame (lib/mark.wgsl `decide`): a texel a cell, each
// lattice's cells in rows of their own (`cover.rows`), drawn before the map (web/src/textures.ts `prepare`), one of its
// words (`MarkWord`) into each of the marks' textures.

// The word of the marks drawn (lib/mark.wgsl `mark_word`).
struct MarkWord { word: vec4i }
@group(0) @binding(2) var<uniform> mark_word_drawn: MarkWord;

@fragment fn fs(v: Varying) -> @location(0) vec4f {
  let t = vec2i(floor(v.px));
  // The lattice whose rows hold the texel: the last that starts at or above it.
  var l = 0;
  for (var k = 1; k < COVER_LATTICES; k++) { if (f32(t.y) >= cover.rows[k]) { l = k; } }
  let m = decide(l, vec2f(f32(t.x - MARK_MARGIN), f32(t.y - i32(cover.rows[l]) - MARK_MARGIN)));
  return mark_word(m, mark_word_drawn.word.x);
}
