// uses: draw.wgsl ragged.wgsl water.wgsl style.wgsl land.wgsl rock.wgsl cover_field.wgsl no_grass.wgsl sprites.wgsl snag_sprites.wgsl heights.wgsl tex.wgsl plant.wgsl cells.wgsl mark.wgsl models.wgsl fire.wgsl snag.wgsl
// The plant cover's marks over the live map (web/src/textures.ts): the trees and bushes, at every frame, plant by plant
// where a plant takes a few pixels; where it takes less, the ground draws it as a field of the same colours
// (lib/cover_field.wgsl), and the herbs always so, as its sward. Each lattice of marks (web/src/canopy.ts `LATTICES`)
// lies fixed in the world, one mark a cell at a place and rank hashed from the cell (Worley 1996; pcg3d, Jarzynski and
// Olano 2020; lowbias32, draw.wgsl), so the marks hold while the camera moves and stand in no rows. A mark picks its
// kind at its own foot: of the four squares about it, blended as the map's other fields (land.wgsl `corners`), each
// kind's chance that a cell holds one of its plants (thicker and thinner over the field's patches, cover_field.wgsl
// `patchy`), less where the ground at the foot is drier than its cell's mean as the kind's activity law says and where
// its loose ground is thinner than its roots need (land.wgsl `rooted`, its chance raised where its square's ground is
// thin, so the square keeps its cover); a plant buried by the snow's depth against its height does not show, one
// standing in snow carries it on its crown; each kind claims cells by its own draw (on the trees' lattice in stands of
// their kinds as the field far away has them, cover_field.wgsl `stand_pick`), and where several claim one the tallest
// plant stands. So kinds mix mark by mark across a square's edge and a kind that needs wet ground gathers in the
// hollows. None stands where liquid stands over its roots or under snow. Its size, shape and colour are its kind's
// numbers (canopy.ts, web/src/style.ts, web/src/form.ts): each plant is its model's sprite (lib/models.wgsl) standing up
// on its foot over its shadow, leaning a little its own way. Points are in metres from the draw's top-left corner.
// Each mark is decided once a frame for its cell (lib/mark.wgsl, marks.wgsl) and drawn here.

// Paint colour rgb with coverage a over dst (premultiplied).
fn over(dst: ptr<function, vec4f>, rgb: vec3f, a: f32) { *dst = vec4f(rgb * a, a) + *dst * (1.0 - a); }

// A shadow `a` deep over dst, where the deepest laid on this pixel so far is `shade`: shadows on one spot do not add up,
// the ground in the shade of several plants is as dark as under the deepest one, as the field far away counts the
// ground under any shadow once (canopy.ts `Paint.shade`).
fn shadow(dst: ptr<function, vec4f>, shade: ptr<function, f32>, a: f32) {
  if (a <= *shade) { return; }
  over(dst, vec3f(0.0), (a - *shade) / (1.0 - *shade));
  *shade = a;
}

// Whether cell k of lattice l holds no mark (`decide` put none in it), within the cells its texture holds.
fn mark_empty(l: i32, k: vec2f) -> bool {
  let t = mark_texel(l, k);
  let size = vec2i(textureDimensions(marks_kind));
  if (t.x < 0 || t.x >= size.x || t.y < 0 || t.y >= size.y) { return false; }
  return word_of(textureLoad(marks_kind, t, 0)) == 0u;
}

// The share of the cells about cell k of lattice l (it and its eight neighbours) that hold no mark.
fn empty_about(l: i32, k: vec2f) -> f32 {
  var n = 0.0;
  for (var j = -1; j <= 1; j++) { for (var i = -1; i <= 1; i++) { n += select(0.0, 1.0, mark_empty(l, k + vec2f(f32(i), f32(j)))); } }
  return n / 9.0;
}

// The dead tree of empty cell k of the trees' lattice l (snag.wgsl), over dst at p, m from the draw's corner, e m a
// pixel, as strongly as `fade`: as many as the walls standing at its foot make over the empty share about it.
fn dead_tree(dst: ptr<function, vec4f>, shade: ptr<function, f32>, p: vec2f, l: i32, k: vec2f, e: f32, fade: f32) {
  let side = cover.grid[l].z;
  let foot = mark_foot(l, k);
  let f = fire_at(cover.corner.xy + foot);
  let id = cover.base[l].xy + vec2i(k);
  let per_cell = f.snag * side * side / SNAG_KG;
  let draw_ = snag_unit(id, 41);
  if (draw_ >= per_cell / EMPTY_LEAST || draw_ >= per_cell / max(empty_about(l, k), EMPTY_LEAST)) { return; }
  let tallest = (cover.grid[l].w + cover.reach[l].x) * side / MAP_VIEW.y;
  let seen = snag_seen(snag_of(id, clamp(f.age / max(fire.air.y, 1.0), 0.0, 1.0), tallest), p - foot, e);
  shadow(dst, shade, fade * seen.shadow);
  if (seen.rgba.a > 0.0) { over(dst, seen.rgba.rgb, seen.rgba.a * fade); }
}

// The marks of lattice l over dst at p, m from the draw's corner, e m a pixel, the deepest shadow laid on the pixel
// `shade`: of the cells about the pixel's whose marks reach its cell (`reach_at`), the marks that stand and cover it
// (`decided`). Where fire left dead walls standing (fire.wgsl; none where no fire is known), the trees' lattice also
// looks at all the cells a plant on it could reach the pixel from, and its empty ones hold dead trees (`dead_tree`),
// drawn in the same order, row by row from the north, as the living.
fn lattice(dst: ptr<function, vec4f>, shade: ptr<function, f32>, p: vec2f, l: i32, e: f32) {
  let fade = cover_look.shown[l];
  if (fade <= 0.0) { return; }
  let cell = floor(cover.grid[l].xy + p / cover.grid[l].z);
  let reach = reach_at(l, cell);
  let n = i32(land.map.y);
  let dead = l == STANDS && fire.map.x > 0.0 && fire_at(cover.corner.xy + p).snag > 0.0;
  var least = reach.least;
  var most = reach.most;
  if (dead) {
    let r = i32(cover.reach[l].x);
    least = vec2i(-r);
    most = vec2i(r, r + i32(cover.grid[l].w));
  }
  for (var j = least.y; j <= most.y; j++) {
    for (var i = least.x; i <= most.x; i++) {
      let k = cell + vec2f(f32(i), f32(j));
      if (dead && mark_empty(l, k)) { dead_tree(dst, shade, p, l, k, e, fade); continue; }
      let m = decided(l, k, p - mark_foot(l, k), e);
      if (m.x < 0.0) { continue; }
      // Its square, kind and place among the kind's groups (mark.wgsl `decide`).
      let packed = i32(m.x);
      let kd = packed / (GROUPS * PLACE_STEPS);
      let s = kd / kinds_n();
      // Its share that shows and the depth of the liquid over its foot (mark.wgsl `wade_word`).
      let wade = wade_of(m.y);
      plant(dst, shade, mark_plant(l, k, vec2i(s % n, s / n), kd % kinds_n(), place_of(packed % (GROUPS * PLACE_STEPS)), wade.y), p, e, fade * wade.x, m.z);
    }
  }
}

// The marks over the pixel `v`, premultiplied.
fn marks_over(v: Varying) -> vec4f {
  let p = (v.px - draw.rect.xy) * cover.corner.z;
  // Detail is chosen by the screen's pixel, so a picture drawn at fewer pixels is the same picture.
  let e = cover.seen.x;
  var dst = vec4f(0.0);
  var shade = 0.0;
  for (var l = 0; l < COVER_LATTICES; l++) { lattice(&dst, &shade, p, l, e); }
  // Linear light turned into the screen's share once (draw.wgsl `srgb`), in the style (style.wgsl `painted`),
  // premultiplied.
  if (dst.a <= 0.0) { return vec4f(0.0); }
  return vec4f(painted(srgb(dst.rgb / dst.a)), 1.0) * dst.a * cover_look.look.y;
}

// Plant q over dst at p, e m a pixel, as strongly as `fade`, where snow covers `snow` of the ground and the deepest shadow
// laid on the pixel is `shade`: its shadow and its model's sprites (models.wgsl).
fn plant(dst: ptr<function, vec4f>, shade: ptr<function, f32>, q: Plant, p: vec2f, e: f32, fade: f32, snow: f32) {
  let fit = model_fit(q);
  shadow(dst, shade, fade * model_shadow(q, fit, p, e));
  let m = clump_plant(q, fit, p, e, snow);
  if (m.a > 0.0) { over(dst, m.rgb, m.a * fade); }
}

@fragment fn fs(v: Varying) -> @location(0) vec4f { return marks_over(v); }
