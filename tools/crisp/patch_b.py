import sys, shutil
W = sys.argv[1]
shutil.copy('/tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/crisp/relief.wgsl', W + '/web/shaders/lib/relief.wgsl')

def patch(path, pairs):
    s = open(path).read()
    for a, b in pairs:
        assert a in s, (path, a[:90])
        s = s.replace(a, b)
    open(path, 'w').write(s)

patch(W + '/web/shaders/land.wgsl', [
    ("// uses: draw.wgsl water.wgsl style.wgsl crag.wgsl colour.wgsl land.wgsl rock.wgsl cover_field.wgsl",
     "// uses: draw.wgsl water.wgsl style.wgsl crag.wgsl colour.wgsl land.wgsl rock.wgsl relief.wgsl cover_field.wgsl"),
    ("""  let cs = corners(at / land.map.x);
  let g = ground_at(at, true);
  let q = discharge_at(g);""", """  let cs = corners(at / land.map.x);
  var g = ground_at(at, true);
  // The loose ground with its patches, the bed it bares, and the relief finer than the tiles (relief.wgsl), which
  // moves the ground's height and slope: the shading's normal and the fine line of a shore.
  let loose = loose_seen(at, e);
  let bare = bed_shows(loose);
  let fine = relief(at, e, length(g.slope), bare, loose);
  g.h += fine.x;
  g.slope += fine.yz;
  let q = discharge_at(g);"""),
    ("""  p.loose = loose_at(at);""", """  p.loose = loose;"""),
    ("""  p.rock = bed_shows(p.loose, at);""", """  p.rock = bare;"""),
])

patch(W + '/web/shaders/lib/rock.wgsl', [
    ("""// The loose ground's depth departs from the samples' by up to this share over patches `PATCHY_M` across: a mantle is
// thicker in hollows and thinner on knobs, so the bed shows in outcrops, not along a smooth line.
const PATCHY = 0.5; const PATCHY_M = 40.0;""", """// The loose ground's depth departs from the samples' by up to this share over patches `PATCHY_M` across and in
// `PATCHY_OCTAVES` octaves each `PATCHY_STEP` times finer, each weighing `PATCHY_FALL` of the last and fading out as it
// shrinks to a few pixels (draw.wgsl `shows_at`): a mantle is thicker in hollows and thinner on knobs at every
// scale, so the bed shows in outcrops with ragged edges, not along a smooth line.
const PATCHY = 0.6; const PATCHY_M = 40.0; const PATCHY_OCTAVES = 4; const PATCHY_STEP = 2.9; const PATCHY_FALL = 0.6;
// Loose ground deeper than this, m, creeps into smooth swells; thinner, the bed's roughness shows through it
// (relief.wgsl).
const SMOOTH_LOOSE_M = 0.5;"""),
    ("""// The share of the bed that shows at `at` through `loose` m of loose ground on the mean (`PATCHY`).
fn bed_shows(loose: f32, at: vec2f) -> f32 {
  // Deeper than the thinnest a patch makes it reach the bed, none shows (and the noise need not be read).
  if (loose * (1.0 - PATCHY) >= HIDES_BED_M) { return 0.0; }
  return 1.0 - ease(0.0, HIDES_BED_M, loose * (1.0 + PATCHY * noise(at / PATCHY_M, 55).x));
}""", """// The depth of the loose ground at `at`, m, seen at `e` m a pixel, with its patches (`PATCHY`): the field every edge of
// bare bed is drawn from, the ground's and the plants' alike.
fn loose_seen(at: vec2f, e: f32) -> f32 {
  let mean = loose_at(at);
  // Where the thinnest the patches make it is deeper than shows the bed or its roughness, they change nothing drawn
  // (and no noise is read).
  if (mean * (1.0 - PATCHY) >= max(HIDES_BED_M, SMOOTH_LOOSE_M)) { return mean; }
  var n = 0.0;
  var size = PATCHY_M;
  var weight = 1.0;
  var sum = 0.0;
  for (var o = 0; o < PATCHY_OCTAVES; o++) {
    let shown = shows_at(size, e);
    n += weight * shown * noise_turned(at / size + f32(o) * 7.3, 55 + o);
    sum += weight;
    size /= PATCHY_STEP;
    weight *= PATCHY_FALL;
  }
  return mean * (1.0 + PATCHY * clamp(2.0 * n / sum, -1.0, 1.0));
}

// The share of the bed that shows through `loose` m of loose ground (`loose_seen`).
fn bed_shows(loose: f32) -> f32 { return 1.0 - ease(0.0, HIDES_BED_M, loose); }"""),
])
