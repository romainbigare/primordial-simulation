// uses: draw.wgsl ragged.wgsl water.wgsl style.wgsl land.wgsl rock.wgsl cover_field.wgsl no_grass.wgsl sprites.wgsl snag_sprites.wgsl heights.wgsl tex.wgsl plant.wgsl cells.wgsl mark.wgsl models.wgsl fire.wgsl snag.wgsl
// The plant cover's marks over the live map (web/src/textures.ts): the trees and bushes, at every frame, plant by plant
// where a plant takes a few pixels; where it takes less, the ground draws it as a field of the same colours
// (lib/cover_field.wgsl), and the herbs always so, as its sward. Each lattice of marks (web/src/canopy.ts `LATTICES`)
// lies fixed in the world, one mark a cell at a place and rank hashed from the cell (Worley 1996; pcg3d, Jarzynski and
// Olano 2020; lowbias32, draw.wgsl), so the marks hold while the camera moves and stand in no rows. A mark picks its
// kind at its own foot: of the four squares about it, blended as the map's other fields (land.wgsl `corners`), each
// kind's chance that a cell holds one of its plants (thicker and thinner over the field's patches, cover_field.wgsl
// `patchy`), on the saturated ground at the foot (land.wgsl `saturated_at`) and on the drier ground as densely as the
// core splits the kind's plants between its land's saturated strips and its rest (mark.wgsl `dense_at`), and less where
// its loose ground is thinner than its roots need (land.wgsl `rooted`, its chance raised where its square's ground is
// thin, so the square keeps its cover); a plant buried by the snow's depth against its height does not show, one
// standing in snow carries it on its crown; each kind claims cells by its own draw (on the trees' lattice in stands of
// their kinds as the field far away has them, cover_field.wgsl `stand_pick`), and where several claim one the tallest
// plant stands. So kinds mix mark by mark across a square's edge and a kind the law holds on the strips gathers in
// the hollows. None stands where liquid stands over its roots or under snow. Its size, shape and colour are its kind's
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

// The dead tree decided for cell k of the trees' lattice l (mark.wgsl `dead_decided`, `dead_at`), over dst at p, m
// from the draw's corner, e m a pixel, as strongly as `fade`.
fn dead_tree(dst: ptr<function, vec4f>, shade: ptr<function, f32>, p: vec2f, l: i32, k: vec2f, e: f32, fade: f32) {
  let seen = snag_seen(dead_at(l, k).q, p - mark_foot(l, k), e, cover.reach[l].x * cover.grid[l].z);
  shadow(dst, shade, fade * seen.shadow);
  if (seen.rgba.a > 0.0) { over(dst, seen.rgba.rgb, seen.rgba.a * fade); }
}

// The marks of lattice l over dst at p, m from the draw's corner, e m a pixel, the deepest shadow laid on the pixel
// `shade`: of the cells about the pixel's whose marks reach its cell (`reach_at`), the marks that stand and cover it
// (`decided`): living plants, and dead trees where fire left dead walls standing (mark.wgsl `dead_decided`), in one
// order, row by row from the north.
fn lattice(dst: ptr<function, vec4f>, shade: ptr<function, f32>, p: vec2f, l: i32, e: f32) {
  let fade = cover_look.shown[l];
  if (fade <= 0.0) { return; }
  let cell = floor(cover.grid[l].xy + p / cover.grid[l].z);
  let reach = reach_at(l, cell);
  let n = i32(land.map.y);
  for (var j = reach.least.y; j <= reach.most.y; j++) {
    for (var i = reach.least.x; i <= reach.most.x; i++) {
      let k = cell + vec2f(f32(i), f32(j));
      let m = decided(l, k, p - mark_foot(l, k), e);
      if (m.x < 0.0) { continue; }
      if (m.x >= dead_code()) { dead_tree(dst, shade, p, l, k, e, fade * m.y); continue; }
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
