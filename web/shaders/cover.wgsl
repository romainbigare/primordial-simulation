// uses: draw.wgsl water.wgsl style.wgsl land.wgsl rock.wgsl cover_field.wgsl no_grass.wgsl tex.wgsl crown.wgsl mark.wgsl models.wgsl
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

// The sun from the north-west lays a shadow this share of the height to the south-east, but no farther than `SHADOW_OFF`
// of the crown's radius (so a tree stands on its shadow), soft over `SHADOW_SOFT` of its radius.
const SHADOW_RUN = 0.5; const SHADOW_OFF = 0.45; const SHADOW_SOFT = 0.35;
// A shadow lets through the sky's light and what crowns scatter, about half of the sun's (canopy.ts `CROWN_SHADOW`).
const SHADOW = 0.42;
// Snow lies on a crown's upper, lit side (its light from `SNOW_LIT.x` to `SNOW_LIT.y` times flat ground's) up to this
// share of its colour, as closed as the crown is (a bare one holds little), in the colour of fresh snow (colour.wgsl
// `SNOW`).
// Bark in the shade of its own crown: its walls' colour times this.
const BARK = 0.55;
const CROWN_SNOW = 0.8; const SNOW_LIT = vec2f(0.9, 1.15); const CROWN_SNOW_RGB = vec3f(0.90, 0.92, 0.95);


// Plant q over dst at p, e m a pixel, as strongly as `fade`, where snow covers `snow` of the ground: its shadow, its stem
// (a tree), its body by its design (a closed crown lit from above carries the snow, `CROWN_SNOW`), and a bush's flowers
// or fruit.
fn plant(dst: ptr<function, vec4f>, q: Plant, p: vec2f, e: f32, fade: f32, snow: f32) {
  let angled = cover.corner.w;
  let half = min(0.5 * q.d, q.reach / 1.1);
  // The shadow: from above to the south-east of its foot by its height (`SHADOW_OFF`); angled, flat on the ground at its
  // foot.
  let off = mix(min(min(SHADOW_RUN * q.h, SHADOW_OFF * half), max(q.reach - half, 0.0) / 0.7071), 0.3 * half, angled);
  let sv = (p - q.at - off * vec2f(0.7071)) / vec2f(half, half * mix(1.0, 0.5, angled));
  over(dst, vec3f(0.0), SHADOW * fade * (1.0 - smoothstep(1.0 - SHADOW_SOFT, 1.0, length(sv))));
  let bark = BARK * cover.walls.rgb;
  // Angled, a plant is its model's sprite (models.wgsl) once the models are loaded.
  if (angled > 0.5 && models_ready()) {
    let m = model_plant(q, p, e, snow);
    if (m.a > 0.0) { over(dst, m.rgb, m.a * fade); }
    return;
  }
  var body: Body;
  if (q.design == TREE) {
    // The stem: angled from its foot to its crown (`stem`), leaning with it; from above its top through an open crown.
    if (angled > 0.5) {
      let s = stem(q, p, e);
      if (s.a > 0.0) { over(dst, mark_colour(bark * s.tone, s.lit, s.rim), q.wood * s.a * fade); }
    } else {
      let w = max(0.5 * q.stem, 0.35 * e);
      over(dst, bark, q.wood * (1.0 - q.closed) * (1.0 - smoothstep(w - e, w, length(p - q.at))) * fade);
    }
    body = crown(q, p, e);
  } else {
    body = bush(q, p, e);
  }
  if (body.a > 0.0) {
    let snowy = CROWN_SNOW * snow * q.closed * (1.0 - body.wood) * smoothstep(SNOW_LIT.x, SNOW_LIT.y, body.lit);
    let leaf = mix(q.rgb, CROWN_SNOW_RGB, snowy);
    over(dst, mark_colour(mix(leaf, bark, body.wood) * body.tone, body.lit, body.rim), body.a * fade);
  }
  if (q.blooms > 0.0) { bush_dots(dst, q, p, e, fade); }
}

// The marks of lattice l over dst at p, m from the draw's corner, e m a pixel: of the cells about the pixel, the marks
// that stand (`decided`) and may reach it.
fn lattice(dst: ptr<function, vec4f>, p: vec2f, l: i32, e: f32) {
  let g = cover.grid[l];
  let fade = cover_look.shown[l];
  // Skip a lattice none of whose marks stands on any square this pixel's marks could stand on.
  if (fade <= 0.0 || field_at(cover.corner.xy + p, l, cover.near[l].x).a <= 0.0) { return; }
  let side = g.z;
  let cell = floor(g.xy + p / side);
  // A mark reaches no farther than a cell's side from its foot, and angled as far north as its tallest stands.
  let rise = g.w * side;
  let n = i32(land.map.y);
  for (var j = -1; j <= 1 + i32(g.w); j++) {
    for (var i = -1; i <= 1; i++) {
      let k = cell + vec2f(f32(i), f32(j));
      let h = place(cover.base[l].xy + vec2i(k), l);
      let d = p - (k - g.xy) * side - h.xy * side;
      if (abs(d.x) > side + e || d.y > side + e || d.y < -side - rise - e) { continue; }
      let m = decided(l, k);
      if (m.x < 0.0) { continue; }
      let packed = i32(m.x);
      let s = packed / kinds_n();
      plant(dst, mark_plant(l, k, vec2i(s % n, s / n), packed % kinds_n()), p, e, fade * m.y, m.z);
    }
  }
}

@fragment fn fs(v: Varying) -> @location(0) vec4f {
  let e = cover.corner.z;
  let p = (v.px - draw.rect.xy) * e;
  var dst = vec4f(0.0);
  for (var l = 0; l < COVER_LATTICES; l++) { lattice(&dst, p, l, e); }
  // Linear light turned into the screen's share once (draw.wgsl `srgb`), in the style (style.wgsl `painted`),
  // premultiplied.
  if (dst.a <= 0.0) { return vec4f(0.0); }
  return vec4f(painted(srgb(dst.rgb / dst.a)), 1.0) * dst.a * cover_look.look.y;
}

