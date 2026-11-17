// uses: draw.wgsl ragged.wgsl water.wgsl style.wgsl land.wgsl rock.wgsl cover_field.wgsl no_grass.wgsl sprites.wgsl tex.wgsl crown.wgsl cells.wgsl mark.wgsl models.wgsl cover_marks.wgsl
// The plant cover's marks from above, and angled until the models are loaded, each plant a drawn crown of its kind's
// form (lib/crown.wgsl; lib/cover_marks.wgsl draws them).

// The sun from the north-west lays a shadow this share of the height to the south-east, but no farther than `SHADOW_OFF`
// of the crown's radius (so a tree stands on its shadow), soft over `SHADOW_SOFT` of its radius.
const SHADOW_RUN = 0.5; const SHADOW_OFF = 0.45; const SHADOW_SOFT = 0.35;
// A shadow's half width is at most its plant's reach over this: with its offset it stays within the reach.
const SHADOW_REACH = 1.5;
// Bark in the shade of its own crown: its walls' colour times this.
const BARK = 0.55;

// Plant q over dst at p, e m a pixel, as strongly as `fade`, where snow covers `snow` of the ground and the deepest shadow
// laid on the pixel is `shade`: its shadow, its stem (a tree), its body by its design (a closed crown lit from above
// carries the snow, `CROWN_SNOW`), and a bush's flowers or fruit.
fn plant(dst: ptr<function, vec4f>, shade: ptr<function, f32>, q: Plant, p: vec2f, e: f32, fade: f32, snow: f32) {
  let angled = cover.corner.w;
  // The shadow: from above to the south-east of its foot by its height (`SHADOW_OFF`); angled, flat on the ground at
  // its foot; within its reach with its offset (`SHADOW_REACH`): the pixel looks no farther for it (`lattice`).
  let half = min(0.5 * q.d, q.reach / SHADOW_REACH);
  let off = mix(min(min(SHADOW_RUN * q.h, SHADOW_OFF * half), max(q.reach - half, 0.0) / 0.7071), 0.3 * half, angled);
  let sv = (p - q.at - off * vec2f(0.7071)) / vec2f(half, half * mix(1.0, 0.5, angled));
  shadow(dst, shade, SHADOW * fade * (1.0 - smoothstep(1.0 - SHADOW_SOFT, 1.0, length(sv))));
  let bark = BARK * cover.walls.rgb;
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


@fragment fn fs(v: Varying) -> @location(0) vec4f { return marks_over(v); }
