import os
os.chdir('/home/user/primordial-simulation/.claude/worktrees/agent-a2f9648fe0863f025')
def edit(p, pairs):
    s = open(p).read()
    for a, b in pairs:
        assert s.count(a) == 1, (p, a)
        s = s.replace(a, b)
    open(p, 'w').write(s)
edit('web/shaders/lib/draw.wgsl', [
("""// Every program starts with `const LEAST_E`, the least metres a pixel takes in its draws (web/shade): 0, or for a
// program made once more for draws seen from farther, the metres it is made for (land.wgsl). Detail that cannot show
// at that size of a pixel is compiled out of it: the picture is the same, and the GPU runs and holds less code.

// Whether detail `size` m across shows at no pixel of the draws of this program (`shows_at` is 0 from 0.75 of its size
// a pixel on).
fn unseen(size: f32) -> bool { return 0.75 * size <= LEAST_E; }""",
"""// Every program starts with `const LEAST_E`, the least metres a pixel takes in its draws (web/shade): 0, or for a
// program made once more for draws seen from farther, the metres it is made for (land.wgsl). Detail that cannot show
// at that size of a pixel is compiled out of it: the picture is the same, and the GPU runs and holds less code. It is
// read through `least_e`, so the translation to GLSL keeps it by name and the programs made for each size differ in
// that line alone (web/shade, web/src/gpu.ts `programs`).
fn least_e() -> f32 { return LEAST_E; }

// Whether detail `size` m across shows at no pixel of the draws of this program (`shows_at` is 0 from 0.75 of its size
// a pixel on).
fn unseen(size: f32) -> bool { return 0.75 * size <= least_e(); }"""),
])
edit('web/shaders/lib/cover_field.wgsl', [
("  if (LEAST_E >= 2.0 * STAND_CALM_E) { return mean; }", "  if (least_e() >= 2.0 * STAND_CALM_E) { return mean; }"),
("  if (LEAST_E < LOW_FAR_FULL_E && far < 1.0) {", "  if (least_e() < LOW_FAR_FULL_E && far < 1.0) {"),
])
edit('web/shaders/lib/crag.wgsl', [
("  if (cliff > 0.0 && CLIFF_FROM * LEAST_E < 0.3 * STRATA_M) {", "  if (cliff > 0.0 && CLIFF_FROM * least_e() < 0.3 * STRATA_M) {"),
])
edit('web/shaders/lib/water.wgsl', [
("    if (LEAST_E >= 0.5 * wave) { break; }", "    if (least_e() >= 0.5 * wave) { break; }"),
])
