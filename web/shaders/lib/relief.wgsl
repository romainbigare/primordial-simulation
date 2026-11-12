// The ground's relief finer than its tiles (land.wgsl): the terrain's samples stand 125 m apart and the tiles draw the
// ground between them smooth, so a sample's worth of ground would be one soft swell. Here the drawing adds detail at
// every scale from `RELIEF_LONGEST_M` down to the pixel, fixed in the world: octaves of value noise each `RELIEF_STEP`
// times finer (gradient noise, whose slope runs on smoothly across its cells, so the light shows no grid: `relief_noise`),
// each turned from the last so they stand in no grid, each fading out as it shrinks to a few pixels
// (draw.wgsl `shows_at`; it would only shimmer). Its strength follows the world's numbers: deep loose ground creeps into smooth swells (rise per
// run `SWELL`), bare bed is rough (`CRAG`), its octaves ridged into crests and ledges, the more the steeper (a
// fractal surface whose slope is alike at every scale, as measured ground is over metres to kilometres: Mark and
// Aronson 1984). It moves the shading's normal and the fine line of every edge it meets (a shore wiggles at the metre
// scale), not the physics. Deterministic by world position: nothing moves as the view pans or zooms.

// The longest octave's wavelength, m, the ratio between octaves, and their number.
const RELIEF_LONGEST_M = 48.0; const RELIEF_STEP = 3.0; const RELIEF_OCTAVES = 6;
// The slope each octave adds (its height over its wavelength): on deep loose ground, on bare bed, and more on bare bed
// per unit of its slope (a face is rougher than a slab).
const SWELL = 0.025; const CRAG = 0.09; const CRAG_STEEP = 0.12;

// The gradient at lattice point c of stream k: two 16-bit halves of its random word, each -1..1 (no trigonometry: the
// gradients need not be of one length for the noise to show no grid).
fn relief_gradient(c: vec2i, k: i32) -> vec2f {
  let x = lowbias(c, k);
  return vec2f(f32(x & 0xffffu), f32(x >> 16u)) * (2.0 / 65535.0) - 1.0;
}

// Gradient noise at p (in its cells) of stream k: -0.7..0.7, and its slope along x and y, smooth with its slope
// (quintic easing, Perlin 2002; derivatives as Quilez 2017).
fn relief_noise(p: vec2f, k: i32) -> vec3f {
  let i = vec2i(floor(p));
  let f = p - floor(p);
  let u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
  let du = 30.0 * f * f * (f * (f - 2.0) + 1.0);
  let ga = relief_gradient(i, k);
  let gb = relief_gradient(i + vec2i(1, 0), k);
  let gc = relief_gradient(i + vec2i(0, 1), k);
  let gd = relief_gradient(i + vec2i(1, 1), k);
  let va = dot(ga, f);
  let vb = dot(gb, f - vec2f(1.0, 0.0));
  let vc = dot(gc, f - vec2f(0.0, 1.0));
  let vd = dot(gd, f - vec2f(1.0, 1.0));
  let m = va - vb - vc + vd;
  let value = va + u.x * (vb - va) + u.y * (vc - va) + u.x * u.y * m;
  let slope = ga + u.x * (gb - ga) + u.y * (gc - ga) + u.x * u.y * (ga - gb - gc + gd) + du * (u.yx * m + vec2f(vb, vc) - va);
  return vec3f(value, slope);
}

// The relief at `at`, m in the world, seen at `e` m a pixel, on ground of slope `steep` (rise over run) whose bed shows
// by `bare` (0..1, rock.wgsl `bed_shows`) under `loose` m of loose ground: its height, m, and its slope east and south.
fn relief(at: vec2f, e: f32, steep: f32, bare: f32, loose: f32) -> vec3f {
  var out = vec3f(0.0);
  // No octave shows: none of the noise is read (the fitted view pays nothing).
  if (shows_at(RELIEF_LONGEST_M, e) <= 0.0) { return out; }
  let rough = max(bare, 1.0 - ease(0.0, SMOOTH_LOOSE_M, loose));
  let k = mix(SWELL, CRAG + CRAG_STEEP * min(steep, 2.0), rough);
  var wave = RELIEF_LONGEST_M;
  var rot = mat2x2f(1.0, 0.0, 0.0, 1.0);
  let turn = mat2x2f(0.8, 0.6, -0.6, 0.8);
  for (var o = 0; o < RELIEF_OCTAVES; o++) {
    let shown = shows_at(wave, e);
    if (shown <= 0.0) { break; }
    let n = relief_noise(rot * at / wave, 70 + o);
    // Ridged on bare bed (1 - |n|: crests and ledges), smooth swells on loose ground; slope back in the world's axes.
    let ridge = vec3f(0.35 - abs(n.x), -sign(n.x) * n.yz);
    let v = mix(n, ridge, rough);
    out += shown * k * wave * vec3f(v.x, transpose(rot) * v.yz / wave);
    wave /= RELIEF_STEP;
    rot = turn * rot;
  }
  return out;
}
