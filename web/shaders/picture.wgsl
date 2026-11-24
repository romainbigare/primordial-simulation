// uses: draw.wgsl water.wgsl style.wgsl
// A picture the renderer drew before into a texture (the map drawn at fewer pixels than the screen's while frames are
// slow, or drawn anew at the screen's own a tile at a time once the view rests, or of the area about a view that
// moves), drawn onto the screen, its colour premultiplied by how strongly it covers what is under it. Where the map
// is drawn out to the screen's edges (a phone, map.ts `fillScreen`), the draw reaches past the map, and there it shows
// the open sea (style.wgsl `open_sea_painted`): what touches the map's edge (the sea's ice) ends at it.

@group(0) @binding(1) var picture: texture_2d<f32>;
@group(0) @binding(2) var picture_sampler: sampler;
struct Keep {
  // The part of the picture that holds the map, uv: from (x, y) to (z, w); all of it (0 0 1 1) where the map is not
  // drawn out to the screen's edges.
  uv: vec4f,
}
@group(0) @binding(3) var<uniform> keep: Keep;

// Where the map is drawn out to the screen's edges, a point of it reads the picture no nearer the map's edge than this,
// texels: the edge's texels hold some of the page beyond.
const EDGE_TEXELS = 1.5;

// A picture fades out over this share of the screen's narrower side towards each of its edges that lies inside the
// screen, and over no more than the edge lies inside it: one that fills the screen has no fade, and a fade grows as the
// view moves the picture's edge in, so a softer picture under it shows through with no line and no jump.
const EDGE_FADE = 0.12;

// How strongly the picture covers the point `px` of the target: 1 inside, falling to 0 at its edges inside the screen.
fn edge_cover(px: vec2f) -> f32 {
  let lo = draw.rect.xy;
  let hi = draw.rect.xy + draw.rect.zw;
  let most = EDGE_FADE * min(draw.size.x, draw.size.y);
  let band_lo = clamp(lo, vec2f(0.0), vec2f(most));
  let band_hi = clamp(draw.size - hi, vec2f(0.0), vec2f(most));
  let c_lo = select(vec2f(1.0), clamp((px - lo) / max(band_lo, vec2f(1e-3)), vec2f(0.0), vec2f(1.0)), band_lo > vec2f(0.0));
  let c_hi = select(vec2f(1.0), clamp((hi - px) / max(band_hi, vec2f(1e-3)), vec2f(0.0), vec2f(1.0)), band_hi > vec2f(0.0));
  let c = c_lo * c_hi;
  return c.x * c.y;
}

@fragment fn fs(v: Varying) -> @location(0) vec4f {
  let c = edge_cover(v.px) * draw.alpha;
  let lo = keep.uv.xy;
  let hi = keep.uv.zw;
  let filling = any(keep.uv != vec4f(0.0, 0.0, 1.0, 1.0));
  let edge = select(vec2f(0.0), vec2f(EDGE_TEXELS) / vec2f(textureDimensions(picture)), filling);
  // (A picture shown smaller than it was drawn reads its mip levels, so it does not shimmer.)
  let map = textureSample(picture, picture_sampler, clamp(v.uv, lo + edge, max(hi - edge, lo + edge))).rgb;
  let past = any(v.uv < lo) || any(v.uv > hi);
  return vec4f(select(map, open_sea_painted(), past) * c, c);
}
