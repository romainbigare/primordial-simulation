// uses: draw.wgsl ragged.wgsl water.wgsl style.wgsl land.wgsl rock.wgsl sprites.wgsl heights.wgsl tex.wgsl fire.wgsl
// The dead trees fire left standing over the live map, over its plants (web/src/fire.ts; drawn only over the squares
// that hold them): as many as the core's dead walls standing (`FireAt.snag`, kg/m²) make, one at most in each cell of a
// lattice fixed in the world, each a sprite of web/textures/snags.png (a charred trunk, dead stems or, more of them the
// longer since the burn, a stump broken off), leaning more the longer since the burn. Seen from afar, their mean dark
// over the ground. Colours are the screen's shares, 0..1, premultiplied.

// One dead tree standing holds `SNAG_KG` of walls, and stands `SNAG_TALL` m tall (`SNAG_VARY` either way): a stem of
// about 20 cm and 12 m, from memory. A cell of the lattice is `SNAG_CELL` m across and holds one with the chance its
// share of the walls makes.
const SNAG_KG = 100.0; const SNAG_TALL = 12.0; const SNAG_VARY = 0.3; const SNAG_CELL = 6.0;
// The sprites (snags.png): their count, the trunks' and the stems' first, the stumps' first; each one's width and
// height as shares of its quarter, its foot `SNAG_FOOT` of the quarter over the bottom (bake/snags.py).
const SNAG_SPRITES = 7; const SNAG_STUMPS = 4; const SNAG_FOOT = 2.0 / 256.0;
var<private> SNAG_W: array<f32, 7> = array<f32, 7>(0.391, 0.289, 0.434, 0.984, 0.582, 0.688, 0.449);
var<private> SNAG_H: array<f32, 7> = array<f32, 7>(0.988, 0.988, 0.988, 0.539, 0.941, 0.996, 0.996);
// A stump stands `STUMP_TALL` of a standing snag's height. A dead tree is a stump with the chance the months since the
// burn are of the most the core tells apart, and leans by up to `LEAN_MOST` radians as far.
const STUMP_TALL = 0.3; const LEAN_MOST = 0.6;
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
  let shows = as_thing(MAP_VIEW.y * SNAG_TALL, e);
  var out = vec4f(0.0);
  if (shows > 0.0) {
    let cell = vec2i(floor(at / SNAG_CELL));
    let reach = i32(ceil(MAP_VIEW.y * SNAG_TALL * (1.0 + SNAG_VARY) / SNAG_CELL));
    for (var dy = 0; dy <= reach; dy++) {
      for (var dx = -1; dx <= 1; dx++) {
        let c = cell + vec2i(dx, dy);
        if (unit(c, 41) >= chance) { continue; }
        let stump = unit(c, 42) < old;
        let s = select(i32(unit(c, 43) * f32(SNAG_STUMPS)), SNAG_STUMPS + i32(unit(c, 43) * f32(SNAG_SPRITES - SNAG_STUMPS)), stump);
        let tall = MAP_VIEW.y * SNAG_TALL * mix(1.0 - SNAG_VARY, 1.0 + SNAG_VARY, unit(c, 44)) * select(1.0, STUMP_TALL, stump);
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
  let mean = SNAG_DARK * chance * 0.5 * SNAG_W[0] * MAP_VIEW.y * SNAG_TALL * SNAG_TALL / (SNAG_CELL * SNAG_CELL);
  return mix(vec4f(0.0, 0.0, 0.0, min(mean, 1.0)), out, shows) * draw.alpha;
}
