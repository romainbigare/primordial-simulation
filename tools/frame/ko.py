# Knock one part out of the land shader (in the current directory, a copy of web/shaders), for the cost of each part.
import sys
what = sys.argv[1:]
def edit(path, a, b):
    s = open(path).read()
    assert a in s, (path, a)
    open(path, 'w').write(s.replace(a, b))
for w in what:
    if w == 'spline':
        edit('lib/land.wgsl', '  if (!spline) { return', '  if (true) { return')
    elif w == 'table':  # the colour planes read once, not blended
        edit('lib/land.wgsl', 'fn blended(k: i32, c: Corners) -> vec4f {\n', 'fn blended(k: i32, c: Corners) -> vec4f {\n  if (k >= SURFACE) { return cell(k, c.at[0]); }\n')
    elif w == 'cover':
        edit('land.wgsl', 'let cf = cover_field(at, e, Beneath(soaked(p), shore_over, length(g.slope), loose, fwidth(loose), st.snow, st.snow_m));', 'let cf = CoverAt(vec4f(0.0), 0.0);')
    elif w == 'extras':
        edit('land.wgsl', 'if (spots.bars > 0.0) {', 'if (false) {')
        edit('land.wgsl', 'if (spots.pool > 0.0 && spots.pool < 1.0) {', 'if (false) {')
    elif w == 'river':
        edit('land.wgsl', 'if (spots.share > 0.0) {', 'if (false) {')
    elif w == 'rockface':
        edit('lib/colour.wgsl', 'if (p.rock > 0.0) { bed = rock_face(', 'if (false) { bed = rock_face(')
    elif w == 'stones':
        edit('lib/colour.wgsl', '  if (styled()) {\n    let thin', '  if (false) {\n    let thin')
    elif w == 'loose':
        edit('land.wgsl', 'let loose = loose_seen(at, e);', 'let loose = 1.0;')
    elif w == 'relief':
        edit('land.wgsl', 'let fine = relief(at, e, length(g.slope), bare, loose);', 'let fine = vec3f(0.0);')
    elif w == 'state':
        edit('land.wgsl', 'let st = state_on(at, g, cs, bed.depth);', 'var st: State; st.over = 1000.0; st.t = 290.0; st.wet = 0.5;')
    elif w == 'dead':  # a branch never taken, with sixteen noise reads in it
        edit('land.wgsl', '  // The true colour is the cells', '  if (earth.view.x > 100.0) { c.rgb += vec3f(noise(at * 1.00, 0).x + noise(at * 1.37, 1).x + noise(at * 1.74, 2).x + noise(at * 2.11, 3).x + noise(at * 2.48, 4).x + noise(at * 2.85, 5).x + noise(at * 3.22, 6).x + noise(at * 3.59, 7).x + noise(at * 3.96, 8).x + noise(at * 4.33, 9).x + noise(at * 4.70, 10).x + noise(at * 5.07, 11).x + noise(at * 5.44, 12).x + noise(at * 5.81, 13).x + noise(at * 6.18, 14).x + noise(at * 6.55, 15).x); }\n  // The true colour is the cells')
    elif w == 'tier3':  # what cannot show at 3 m a pixel, compiled out
        edit('lib/draw.wgsl', 'fn shows_at(size: f32, e: f32) -> f32 { return', 'const LEAST_E = 3.0;\nfn shows_at(size: f32, e: f32) -> f32 { if (0.75 * size <= LEAST_E) { return 0.0; } return')
        edit('lib/water.wgsl', '    let shown = 1.0 - ease(wave / 8.0, wave / 2.0, e);', '    var shown = 1.0 - ease(wave / 8.0, wave / 2.0, e);\n    if (LEAST_E >= 0.5 * RIPPLE_LONGEST_M) { shown = 0.0; }')
        edit('lib/crag.wgsl', '  if (cliff > 0.0) {', '  if (cliff > 0.0 && CLIFF_FROM * LEAST_E < 0.3 * STRATA_M) {')
        edit('lib/crag.wgsl', '  let slab_n = noise(at / SLAB_M, 58).x;\n  let slabs = (1.0 + SLAB_TONES * noise(at / SLAB_M + 17.0, 59).x - CRACK_DARK * crack(slab_n, CRACK_W));', '  let slab_shows = shows_at(SLAB_M * CRACK_W * 4.0, e);\n  var slabs = 1.0;\n  if (slab_shows > 0.0) { slabs = (1.0 + SLAB_TONES * noise(at / SLAB_M + 17.0, 59).x - CRACK_DARK * crack(noise(at / SLAB_M, 58).x, CRACK_W)); }')
    elif w == 'sward':
        edit('lib/cover_field.wgsl', '    dst = sward(at, e, b, trees.a, thick, hold.w);', '    dst = vec4f(0.0);')
    elif w == 'lowl':
        edit('lib/cover_field.wgsl', '      if (cover_look.sides[l] > 0.0 && cover_look.shown[l] < 1.0) {', '      if (false) {')
    elif w == 'treesf':
        edit('lib/cover_field.wgsl', '  dst = lattice_field(at, e, STANDS, trees, b, thick, hold[STANDS], dst);', '')
    elif w == 'held':
        edit('lib/cover_field.wgsl', '  var hold = held_at(at, b);', '  var hold = vec4f(1.0);')
    elif w == 'farline':
        edit('lib/cover_field.wgsl', 'const LOW_LAYER = 9; const LOW_FAR_E = 0.75; const LOW_FAR_FULL_E = 1.0;', 'const LOW_LAYER = 9; const LOW_FAR_E = 0.45; const LOW_FAR_FULL_E = 0.6;')
        edit('land.wgsl', '// least metres a pixel: 0.5 1 3', '// least metres a pixel: 0.5 0.6 1 3')
    else:
        raise SystemExit('unknown ' + w)
