// The plant cover's marks over the live map (web/src/textures.ts), drawn at every frame by one of two programs that
// draw a plant each its own way (`plant`): cover.wgsl, angled, from rendered models; crowns.wgsl, from above (and angled
// until the models are loaded), from drawn crowns.
// The trees and bushes over the live map (web/src/textures.ts), at every frame, plant by plant where a plant takes a
// few pixels; where it takes less, the ground draws it as a field of the same colours (lib/cover_field.wgsl), and the
// herbs always so, as its sward. Each lattice of marks (web/src/canopy.ts `LATTICES`) lies fixed in the world, one mark
// a cell at a place and rank hashed from the cell (Worley 1996; pcg3d, Jarzynski and Olano 2020; lowbias32, draw.wgsl),
// so the marks hold while the camera moves and stand in no rows. A mark picks its kind at its own foot: of the four
// squares about it, blended as the map's other fields (land.wgsl `corners`), each kind's chance that a cell holds one of
// its plants (thicker and thinner over the field's patches, cover_field.wgsl `patchy`), less where the ground at the foot
// is drier than its cell's mean as the kind's activity law says and where its loose ground is thinner than its roots
// need (land.wgsl `rooted`, its chance raised where its square's ground is thin, so the square keeps its cover); a plant
// buried by the snow's depth against its height does not show, one standing in snow carries it on its crown; on the
// trees' lattice in stands of their kinds as the
// field far away has them (cover_field.wgsl `stand_pick`), elsewhere by the mark's own rank. So kinds mix mark by mark
// across a square's edge and a kind that needs wet ground gathers in the hollows. None stands where liquid stands over
// its roots or under snow. Its size, shape and colour are its kind's numbers (canopy.ts, web/src/style.ts,
// web/src/form.ts; lib/crown.wgsl draws its body): a tree is a crown of its kind's form over its stem and its shadow,
// leaning a little its own way; a bush a low knot of lobes with its flowers or fruit. Angled, each stands up on its
// foot. Points are in metres from the draw's top-left corner.
// Each mark is decided once a frame for its cell (lib/mark.wgsl, marks.wgsl) and drawn here.

// A shadow `a` deep over dst, where the deepest laid on this pixel so far is `shade`: shadows on one spot do not add up,
// the ground in the shade of several plants is as dark as under the deepest one, as the field far away counts the
// ground under any shadow once (canopy.ts `Paint.shade`).
fn shadow(dst: ptr<function, vec4f>, shade: ptr<function, f32>, a: f32) {
  if (a <= *shade) { return; }
  over(dst, vec3f(0.0), (a - *shade) / (1.0 - *shade));
  *shade = a;
}

// The marks of lattice l over dst at p, m from the draw's corner, e m a pixel, the deepest shadow laid on the pixel
// `shade`: of the cells about the pixel's whose marks reach its cell (`reach_at`), the marks that stand and cover it
// (`decided`).
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
      let packed = i32(m.x);
      let s = packed / kinds_n();
      plant(dst, shade, mark_plant(l, k, vec2i(s % n, s / n), packed % kinds_n()), p, e, fade * m.y, m.z);
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

