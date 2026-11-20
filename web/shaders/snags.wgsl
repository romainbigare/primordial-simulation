// uses: draw.wgsl ragged.wgsl water.wgsl style.wgsl land.wgsl rock.wgsl sprites.wgsl heights.wgsl tex.wgsl fire.wgsl
// The dead trees fire left standing over the live map, over its plants (web/src/fire.ts; drawn only over the squares
// that hold them): as many as the core's dead walls standing (`FireAt.snag`, kg/m²) make, one at most in each cell of a
// lattice fixed in the world, each a sprite of web/textures/snags.png: a stand of dead stems, a charred trunk broken
// off, or a stump, more of them broken and leaning the longer since the burn. Seen from afar, their mean dark over the
// ground. Colours are the screen's shares, 0..1, premultiplied.

// One dead tree holds `SNAG_KG` of walls, from memory (a stem of about 20 cm and 10 m). A cell of the lattice is
// `SNAG_CELL` m across and holds one with the chance its share of the walls makes.
const SNAG_KG = 150.0; const SNAG_CELL = 8.0;
// The sprites (snags.png), each its width and height as shares of its quarter, its foot `SNAG_FOOT` of the quarter over
// the bottom (bake/snags.py): three charred trunks broken off (`KIND_TRUNK`), a stand of dead stems (`KIND_STEMS`), three
// charred stumps (`KIND_STUMP`); the first sprite of each kind and how many, and each kind's height drawn, m (its tallest
// part), `SNAG_VARY` either way.
const SNAG_FOOT = 2.0 / 256.0; const SNAG_VARY = 0.25;
var<private> SNAG_W: array<f32, 7> = array<f32, 7>(0.391, 0.289, 0.434, 0.984, 0.582, 0.688, 0.449);
var<private> SNAG_H: array<f32, 7> = array<f32, 7>(0.988, 0.988, 0.988, 0.539, 0.941, 0.996, 0.996);
const KIND_TRUNK = 0; const KIND_STEMS = 1; const KIND_STUMP = 2;
var<private> KIND_FROM: array<i32, 3> = array<i32, 3>(0, 3, 4);
var<private> KIND_N: array<i32, 3> = array<i32, 3>(3, 1, 3);
var<private> KIND_TALL: array<f32, 3> = array<f32, 3>(5.0, 9.0, 1.5);
// A fresh snag is a stand of stems with the chance `STEMS_FRESH`; the longer since the burn (its share of the most
// months the core tells apart) the more are stumps, and the more they lean, by up to `LEAN_MOST` radians.
const STEMS_FRESH = 0.45; const LEAN_MOST = 0.6;
// From afar, a snag darkens the ground it stands over by `SNAG_DARK` of its cover.
const SNAG_DARK = 0.5;

fn unit(c: vec2i, k: i32) -> f32 { return f32(lowbias(c, k)) * (1.0 / 4294967296.0); }

// The texel of snag sprite s at `uv` in its content (0..1, y down; outside: none), at `g` of its content a pixel: the
// screen's shares, and cover.
fn snag_texel(s: i32, uv: vec2f, g: f32) -> vec4f {
  if (any(uv < vec2f(0.0)) || any(uv > vec2f(1.0))) { return vec4f(0.0); }
  let size = vec2f(SNAG_W[s], SNAG_H[s]);
  let k = s % 4;
  let q = vec2f(f32(k % 2), f32(k / 2));
  let at = (q + vec2f(0.5 - 0.5 * size.x, 1.0 - size.y - SNAG_FOOT) + uv * size) * 0.5;
  return textureSampleGrad(detail_tex, detail_sampler, at, SNAGS_FROM + s / 4, vec2f(0.5 * g * size.x, 0.0), vec2f(0.0, 0.5 * g * size.y));
}

@fragment fn fs(v: Varying) -> @location(0) vec4f {
  pix_origin = fire.frame.xy;
  pix_local = fire.corner.xy + v.px * fire.frame.z;
  let at = pix_origin + pix_local;
  let e = fire.frame.w;
  let f = fire_at(at);
  if (f.snag <= 0.0 || standing(ground_at(at, false)) > FILM_M) { return vec4f(0.0); }
  let chance = min(f.snag * SNAG_CELL * SNAG_CELL / SNAG_KG, 1.0);
  let old = clamp(f.age / max(fire.air.y, 1.0), 0.0, 1.0);
  // Up close each snag in the cells whose snags reach the point (the point lies over a foot, or north of it within its
  // height as the map's angled view draws it); from afar their mean.
  let shows = as_thing(MAP_VIEW.y * KIND_TALL[KIND_TRUNK], e);
  var out = vec4f(0.0);
  if (shows > 0.0) {
    let cell = vec2i(floor(at / SNAG_CELL));
    // The cells whose snags reach the point: south of it within the tallest's height, either side within half the
    // widest's width.
    let most = KIND_TALL[KIND_STEMS] * (1.0 + SNAG_VARY);
    let down = i32(ceil(MAP_VIEW.y * most / SNAG_CELL));
    let across = i32(ceil(0.5 * most * SNAG_W[KIND_FROM[KIND_STEMS]] / SNAG_H[KIND_FROM[KIND_STEMS]] / SNAG_CELL));
    for (var dy = 0; dy <= down; dy++) {
      for (var dx = -across; dx <= across; dx++) {
        let c = cell + vec2i(dx, dy);
        if (unit(c, 41) >= chance) { continue; }
        let pick = unit(c, 42);
        var kind = KIND_TRUNK;
        if (pick < old) { kind = KIND_STUMP; } else if (pick < old + STEMS_FRESH * (1.0 - old)) { kind = KIND_STEMS; }
        let s = KIND_FROM[kind] + min(i32(unit(c, 43) * f32(KIND_N[kind])), KIND_N[kind] - 1);
        let tall = MAP_VIEW.y * KIND_TALL[kind] * mix(1.0 - SNAG_VARY, 1.0 + SNAG_VARY, unit(c, 44));
        let foot = (vec2f(c) + vec2f(unit(c, 45), unit(c, 46))) * SNAG_CELL;
        // The point in the sprite's frame, turned by its lean about its foot (its side the sprite's height a quarter).
        let lean = LEAN_MOST * old * (2.0 * unit(c, 47) - 1.0);
        let d = at - foot;
        let r = vec2f(cos(lean) * d.x + sin(lean) * d.y, -sin(lean) * d.x + cos(lean) * d.y);
        let side = tall / SNAG_H[s];
        let uv = vec2f(0.5 + r.x / (SNAG_W[s] * side), 1.0 + r.y / tall);
        let t = snag_texel(s, uv, e / side);
        if (t.a > 0.0) { out = vec4f(t.rgb * t.a, t.a) + out * (1.0 - t.a); }
      }
    }
  }
  let tall = KIND_TALL[KIND_TRUNK];
  let mean = SNAG_DARK * chance * 0.5 * SNAG_W[0] * MAP_VIEW.y * tall * tall / (SNAG_CELL * SNAG_CELL);
  return mix(vec4f(0.0, 0.0, 0.0, min(mean, 1.0)), out, shows) * draw.alpha;
}
