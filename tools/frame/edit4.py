import os
os.chdir('/home/user/primordial-simulation/.claude/worktrees/agent-a2f9648fe0863f025')
def edit(p, pairs):
    s = open(p).read()
    for a, b in pairs:
        assert s.count(a) == 1, (p, a)
        s = s.replace(a, b)
    open(p, 'w').write(s)

edit('web/shaders/lib/draw.wgsl', [
("""// The share of detail `size` m across that shows at `e` m a pixel: none below a pixel.
fn shows_at(size: f32, e: f32) -> f32 { return 1.0 - ease(0.5 * size, 1.5 * size, 2.0 * e); }""",
"""// Every program starts with `const LEAST_E`, the least metres a pixel takes in its draws (web/shade): 0, or for a
// program made once more for draws seen from farther, the metres it is made for (land.wgsl). Detail that cannot show
// at that size of a pixel is compiled out of it: the picture is the same, and the GPU runs and holds less code.

// Whether detail `size` m across shows at no pixel of the draws of this program (`shows_at` is 0 from 0.75 of its size
// a pixel on).
fn unseen(size: f32) -> bool { return 0.75 * size <= LEAST_E; }

// The share of detail `size` m across that shows at `e` m a pixel: none below a pixel.
fn shows_at(size: f32, e: f32) -> f32 {
  if (unseen(size)) { return 0.0; }
  return 1.0 - ease(0.5 * size, 1.5 * size, 2.0 * e);
}"""),
])
edit('web/shaders/land.wgsl', [
("""// uses: draw.wgsl water.wgsl style.wgsl crag.wgsl colour.wgsl land.wgsl rock.wgsl relief.wgsl cover_field.wgsl
""", """// uses: draw.wgsl water.wgsl style.wgsl crag.wgsl colour.wgsl land.wgsl rock.wgsl relief.wgsl cover_field.wgsl
// least metres a pixel: 1 3
"""),
("""// (land.wgsl), in the view's colours.
""", """// (land.wgsl), in the view's colours. Made once more for draws whose pixels take at least 1 m and 3 m (web/shade,
// draw.wgsl `LEAST_E`): seen from afar, the detail of the ground and the plants that cannot show is compiled out.
"""),
])
edit('web/shaders/lib/water.wgsl', [
("""  for (var k = 0; k < RIPPLE_OCTAVES; k++) {
    let shown = 1.0 - ease(wave / 8.0, wave / 2.0, e);""", """  for (var k = 0; k < RIPPLE_OCTAVES; k++) {
    // An octave fades out by half its wavelength a pixel: none shows in a draw of pixels that large (draw.wgsl
    // `LEAST_E`).
    if (LEAST_E >= 0.5 * wave) { break; }
    let shown = 1.0 - ease(wave / 8.0, wave / 2.0, e);"""),
])
edit('web/shaders/lib/crag.wgsl', [
("""  var tone = (1.0 + MOTTLE * noise(at / MOTTLE_M, 57).x) * (1.0 + (ROCK_RELIEF - 1.0) * (shade - 1.0));
  let slab_n = noise(at / SLAB_M, 58).x;
  let slabs = (1.0 + SLAB_TONES * noise(at / SLAB_M + 17.0, 59).x - CRACK_DARK * crack(slab_n, CRACK_W));
  let cliff = cliff_of(steep);
  var face = 1.0;
  if (cliff > 0.0) {""", """  var tone = (1.0 + MOTTLE * noise(at / MOTTLE_M, 57).x) * (1.0 + (ROCK_RELIEF - 1.0) * (shade - 1.0));
  // The slabs and their cracks, read only where they show.
  let slab_shows = shows_at(SLAB_M * CRACK_W * 4.0, e);
  var slabs = 1.0;
  if (slab_shows > 0.0) { slabs = 1.0 + SLAB_TONES * noise(at / SLAB_M + 17.0, 59).x - CRACK_DARK * crack(noise(at / SLAB_M, 58).x, CRACK_W); }
  let cliff = cliff_of(steep);
  var face = 1.0;
  // A face is no less steep than `CLIFF_FROM`, so its pixels span at least that much height: its strata are gone where
  // that is 0.3 of their spacing, and its cracks where they are under a pixel (draw.wgsl `LEAST_E`).
  if (cliff > 0.0 && CLIFF_FROM * LEAST_E < 0.3 * STRATA_M) {"""),
("""  tone *= mix(mix(1.0, slabs, shows_at(SLAB_M * CRACK_W * 4.0, e)), face, cliff);""",
 """  tone *= mix(mix(1.0, slabs, slab_shows), face, cliff);"""),
])
edit('web/shaders/lib/cover_field.wgsl', [
("""  let mean = f.rgb / f.a;
  let shows = shows_at(m, e) * (1.0 - ease(STAND_CALM_E, 2.0 * STAND_CALM_E, e));
  if (shows <= 0.0) { return mean; }""", """  let mean = f.rgb / f.a;
  if (LEAST_E >= 2.0 * STAND_CALM_E) { return mean; }
  let shows = shows_at(m, e) * (1.0 - ease(STAND_CALM_E, 2.0 * STAND_CALM_E, e));
  if (shows <= 0.0) { return mean; }"""),
("""  var dst = vec4f(0.0);
  if (far < 1.0) {""", """  var dst = vec4f(0.0);
  if (LEAST_E < LOW_FAR_FULL_E && far < 1.0) {"""),
])
