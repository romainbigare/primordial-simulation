// The one maker of a stone of the map (colour.wgsl `ground_of`, land.wgsl): a block of rock lying on the ground, seen
// from above. Its outline is what `STONE_CUTS` planes leave of it, each at its own angle and distance from the centre
// (`STONE_CUT`), so no stone is a disc and no two are alike; it is longer one way than the other (`STONE_LONG`). Each
// plane is a face that falls from the stone's top (`STONE_TOP` of its radius high, tilted by up to `STONE_TILT`), and
// the faces meet over a width that says how worn the stone is: sharp edges on a block fresh from a cliff, round ones on a
// cobble a channel has rolled (`STONE_SOFT`, `STONE_FACE`: angular to rounded). The sun lights each face by how it is
// turned (draw.wgsl `LIGHT`), the stone throws a shadow on the ground away from the sun (`STONE_SHADOW`), and loose
// ground or snow about it buries its foot (`sunk`). It is its bed's colour (`STONE_TONE`, each stone a little its own,
// `STONE_TONES`) with the bands of the layers it broke from (`STONE_BANDS`). Stones lie one to a cell of two lattices
// turned from one another, in `STONE_OCTAVES` sizes each `STONE_STEP` times smaller than the last, the smaller the
// more of them (debris has many small pieces and few large: the largest cells hold `STONE_FEW` as often): fixed in the
// world, and each size drawn as itself from `STONE_PX` pixels across, below as the tone its stones give the ground.
const STONE_CUTS = 5; const STONE_CUT = 0.38; const STONE_LONG = 0.7; const STONE_TOP = 0.75; const STONE_TILT = 0.3;
const STONE_SOFT = vec2f(0.04, 0.45); const STONE_FACE = vec2f(1.7, 1.2); const STONE_RISE = 1.1;
const STONE_SHADOW = 0.3; const STONE_SHADOW_REACH = 0.4;
const STONE_TONE = 1.15; const STONE_TONES = 0.18; const STONE_BANDS = 0.12; const STONE_BAND_M = 0.35;
const STONE_OCTAVES = 3; const STONE_STEP = 2.2; const STONE_FEW = 0.35;
const STONE_GAP = 3.0; const STONE_SIZES = 0.4; const STONE_PX = 2.0;
// Snow lies on a stone's flat top (where its face is flatter than `SNOW_TOP`); a stone smaller than `SNOW_BURIES_M`
// is under the snow that covers its ground, one larger than twice that stands out of it.
const SNOW_TOP = vec2f(0.8, 0.95); const SNOW_BURIES_M = 0.7;
// The largest stone drawn, m (tape/src/ground/rock.rs `STONE_MOST_M`): no draw shows one from this many m a pixel on.
const STONE_MOST_M = 2.0;

// A stone at a point: the share of the pixel it covers, the light on it over the light on flat ground, and how flat it
// is there, 0 on a face .. 1 on its top.
struct StoneAt { a: f32, lit: f32, flat: f32 }

// The stone of random word `x` at `v`, the point from its centre in its radii, a pixel `px` radii across: `worn` from
// 0 (angular) to 1 (rounded), its foot `sunk` of its height in the ground.
fn stone_at(v: vec2f, px: f32, x: u32, worn: f32, sunk: f32) -> StoneAt {
  let y = x * 747796405u + 2891336453u;
  // The stone's own frame: turned, and squeezed across its long axis.
  let turn = f32(x & 255u) * (2.0 * PI / 256.0);
  let squeeze = 1.0 + STONE_LONG * f32((x >> 8u) & 15u) / 15.0;
  let rot = vec2f(cos(turn), sin(turn));
  let u = vec2f(rot.x * v.x + rot.y * v.y, (rot.x * v.y - rot.y * v.x) * squeeze);
  let soft = mix(STONE_SOFT.x, STONE_SOFT.y, worn);
  let steep = mix(STONE_FACE.x, STONE_FACE.y, worn);
  // Its height: the lowest of its tilted top and its faces, met softly; and that height's slope.
  var slope = STONE_TILT * (vec2f(f32((x >> 12u) & 31u), f32((x >> 17u) & 31u)) / 15.5 - 1.0);
  var h = STONE_TOP + dot(slope, u);
  for (var i = 0; i < STONE_CUTS; i++) {
    let bits = (y >> u32(6 * i)) & 63u;
    let a = (f32(i) + 0.8 * (f32(bits & 7u) / 7.0 - 0.5)) * (2.0 * PI / f32(STONE_CUTS));
    let n = vec2f(cos(a), sin(a));
    let face = steep * (1.0 - STONE_CUT * f32(bits >> 3u) / 7.0 - dot(n, u));
    let m = clamp(0.5 + 0.5 * (face - h) / soft, 0.0, 1.0);
    h = mix(face, h, m) - soft * m * (1.0 - m);
    slope = mix(-steep * n, slope, m);
  }
  // The slope back in the map's axes.
  let s = vec2f(slope.x, slope.y * squeeze);
  let grad = vec2f(rot.x * s.x - rot.y * s.y, rot.y * s.x + rot.x * s.y);
  let n = normalize(vec3f(-STONE_RISE * grad, 1.0));
  let over = h - sunk * STONE_TOP;
  return StoneAt(clamp(over / max(length(grad) * px, 1e-4) + 0.5, 0.0, 1.0), max(dot(n, LIGHT), 0.0) / LIGHT.z, n.z);
}

// What makes the stones of a place: the share of the cells that hold one, the largest's size, m, how worn they are
// (0 angular .. 1 rounded), how deep their feet are sunk (0..1 of their height), and the snow on their ground, 0..1.
struct Stony { density: f32, size: f32, worn: f32, sunk: f32, snow: f32 }

// The stones `s` at `at`, seen at `e` m a pixel, of bed `bed` (linear, unlit), stream `k` of the world's random numbers:
// their colour times their coverage (with the shadow they throw as black), and that coverage.
fn stones(at: vec2f, e: f32, s: Stony, bed: vec3f, k: i32) -> vec4f {
  var out = vec4f(0.0);
  if (s.density <= 0.0 || s.size <= 0.0) { return out; }
  let tone = bed * STONE_TONE;
  var size = s.size / pow(STONE_STEP, f32(STONE_OCTAVES - 1));
  // The smallest first, the larger over them.
  for (var o = STONE_OCTAVES - 1; o >= 0; o--) {
    let density = s.density * select(1.0, STONE_FEW, o == 0);
    // Under the snow that covers its ground, a small stone does not show.
    let stands = 1.0 - s.snow * (1.0 - ease(SNOW_BURIES_M, 2.0 * SNOW_BURIES_M, size));
    // Smaller than a few pixels: the share of the ground they cover, as a tone.
    var near = 0.0;
    if (!unseen(STONE_MOST_M / STONE_PX)) { near = ease(STONE_PX * e, 2.0 * STONE_PX * e, size); }
    let mean = 2.0 * density * stands * PI * 0.25 * 0.7 / (STONE_GAP * STONE_GAP) * (1.0 - near);
    out = mix(out, vec4f(tone, 1.0), mean);
    if (near > 0.0) {
      let side = STONE_GAP * size;
      for (var l = 0; l < 2; l++) {
        // The second lattice lies turned and half a cell off the first, so no rows of stones show.
        let q = select(at, vec2f(0.8 * at.x + 0.6 * at.y, 0.8 * at.y - 0.6 * at.x), l == 1) / side + f32(l) * 0.5;
        let c = floor(q);
        let x = lowbias(vec2i(c), k + 2 * o + l);
        if (f32(x & 1023u) / 1024.0 >= density) { continue; }
        let r = 0.5 * size * (1.0 + STONE_SIZES * (f32((x >> 20u) & 1023u) / 512.0 - 1.0));
        // It lies anywhere its cell holds it whole (with its shadow).
        let room = 0.5 - (1.0 + STONE_SHADOW_REACH) * r / side;
        let centre = c + 0.5 + room * (vec2f(f32((x >> 10u) & 31u), f32((x >> 15u) & 31u)) / 15.5 - 1.0);
        let v = (q - centre) * side / r;
        let w = x * 2654435761u;
        let st = stone_at(v, e / r, w, s.worn, s.sunk);
        // Its shadow on the ground, away from the sun.
        let shade = STONE_SHADOW * (1.0 - smoothstep(0.6, 1.1, length(v + STONE_SHADOW_REACH * LIGHT.xy / length(LIGHT.xy))));
        out = mix(out, vec4f(0.0, 0.0, 0.0, 1.0), shade * near * stands * (1.0 - st.a));
        if (st.a <= 0.0) { continue; }
        // Its own tone, and the bands of its rock across it.
        let own = 1.0 + STONE_TONES * (f32((w >> 8u) & 255u) / 127.5 - 1.0);
        let dir = vec2f(cos(f32(w >> 24u)), sin(f32(w >> 24u)));
        let bands = 1.0 + STONE_BANDS * sin(dot(v, dir) * r / STONE_BAND_M * 2.0 * PI + f32((w >> 16u) & 255u));
        let cap = s.snow * smoothstep(SNOW_TOP.x, SNOW_TOP.y, st.flat);
        let rgb = mix(lit_by(tone * own * bands, 0.35 + 0.65 * st.lit), SNOW, cap);
        out = mix(out, vec4f(rgb, 1.0), st.a * near * stands);
      }
    }
    size *= STONE_STEP;
  }
  return out;
}
