// The ground's relief finer than its tiles (land.wgsl): the terrain's samples stand 125 m apart and the tiles draw the
// ground between them smooth, so a sample's worth of ground would be one soft swell. Here the drawing adds detail at
// every scale from `RELIEF_LONGEST_M` down to the pixel, fixed in the world: octaves of gradient noise each `RELIEF_STEP`
// times finer (draw.wgsl `gradient_noise`, whose slope runs on smoothly across its cells, so the light shows no grid),
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

// The relief at the pixel's place, seen at `e` m a pixel, on ground of slope `steep` (rise over run) whose bed shows
// by `bare` (0..1, rock.wgsl `bed_shows`) under `loose` m of loose ground: its height, m, and its slope east and south.
fn relief(e: f32, steep: f32, bare: f32, loose: f32) -> vec3f {
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
    let n = gradient_noise(lat(rot * (1.0 / wave), vec2f(0.0)), 70 + o);
    // Ridged on bare bed (1 - |n|: crests and ledges), smooth swells on loose ground; slope back in the world's axes.
    let ridge = vec3f(0.35 - abs(n.x), -sign(n.x) * n.yz);
    let v = mix(n, ridge, rough);
    out += shown * k * wave * vec3f(v.x, transpose(rot) * v.yz / wave);
    wave /= RELIEF_STEP;
    rot = turn * rot;
  }
  return out;
}
