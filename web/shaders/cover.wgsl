// uses: draw.wgsl ragged.wgsl water.wgsl style.wgsl land.wgsl rock.wgsl cover_field.wgsl no_grass.wgsl sprites.wgsl tex.wgsl crown.wgsl cells.wgsl mark.wgsl models.wgsl cover_marks.wgsl
// The plant cover's marks, angled, each plant its model's sprite (lib/models.wgsl; lib/cover_marks.wgsl draws them).

// Plant q over dst at p, e m a pixel, as strongly as `fade`, where snow covers `snow` of the ground and the deepest shadow
// laid on the pixel is `shade`: its shadow and its model's sprites (models.wgsl).
fn plant(dst: ptr<function, vec4f>, shade: ptr<function, f32>, q: Plant, p: vec2f, e: f32, fade: f32, snow: f32) {
  let fit = model_fit(q);
  shadow(dst, shade, fade * model_shadow(q, fit, p, e));
  let m = clump_plant(q, fit, p, e, snow);
  if (m.a > 0.0) { over(dst, m.rgb, m.a * fade); }
}

@fragment fn fs(v: Varying) -> @location(0) vec4f { return marks_over(v); }
