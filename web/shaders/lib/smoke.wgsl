// The smoke of the fires as worked out once a world frame (smoke.wgsl) into a texture of four bytes a texel, a texel
// every `1 / SMOKE_PER_SQUARE` of a square (smoke spreads over hundreds of metres at least: fire.wgsl `SOURCE_SPREAD`
// is 0.4 of a square), read between texels here (`smoke_at`). A texel holds the column, g/m², on a log scale from
// 2^`SMOKE_LOG_LEAST` over `SMOKE_LOG_SPAN` doublings (0: none), and how far it came over the plume's reach, each in
// two bytes.

const SMOKE_PER_SQUARE = 8.0;
// The reach of a plume, m (smoke.wgsl).
const PLUME_M = 8000.0;
const SMOKE_LOG_LEAST = -30.0; const SMOKE_LOG_SPAN = 40.0;
@group(0) @binding(28) var smoke_tex: texture_2d<f32>;

// A share 0..1 in two bytes, and back.
fn two_bytes(u: f32) -> vec2f {
  let k = round(clamp(u, 0.0, 1.0) * 65535.0);
  return vec2f(floor(k / 256.0), k - 256.0 * floor(k / 256.0)) / 255.0;
}
fn of_two_bytes(b: vec2f) -> f32 { return (round(b.x * 255.0) * 256.0 + round(b.y * 255.0)) / 65535.0; }

// The column `g`, g/m², and the share of the plume's reach it came, packed in four bytes.
fn smoke_packed(g: f32, far: f32) -> vec4f {
  let u = select(0.0, (log2(max(g, 1e-30)) - SMOKE_LOG_LEAST) / SMOKE_LOG_SPAN, g > exp2(SMOKE_LOG_LEAST));
  return vec4f(two_bytes(max(u, 0.0)), two_bytes(far));
}

// The column and how far it came (as `smoke_packed` holds them) of texel c, held within the texture.
fn smoke_texel(c: vec2i) -> vec2f {
  let t = textureLoad(smoke_tex, clamp(c, vec2i(0), vec2i(textureDimensions(smoke_tex)) - 1), 0);
  let u = of_two_bytes(t.xy);
  return vec2f(select(0.0, exp2(SMOKE_LOG_LEAST + u * SMOKE_LOG_SPAN), u > 0.0), of_two_bytes(t.zw));
}

// The column of smoke at `at`, m from the map's north-west corner, g/m², and the mean distance from where it was
// sent, m: the texels about it blended, the distance weighted by the column.
fn smoke_at(at: vec2f) -> vec2f {
  let s = at / fire.map.x * SMOKE_PER_SQUARE - 0.5;
  let c = vec2i(floor(s));
  let f = fract(s);
  let w = vec4f((1.0 - f.x) * (1.0 - f.y), f.x * (1.0 - f.y), (1.0 - f.x) * f.y, f.x * f.y);
  var g = 0.0;
  var far = 0.0;
  for (var k = 0; k < 4; k++) {
    let t = smoke_texel(c + vec2i(k % 2, k / 2));
    g += w[k] * t.x;
    far += w[k] * t.x * t.y;
  }
  return vec2f(g, PLUME_M * far / max(g, 1e-30));
}
