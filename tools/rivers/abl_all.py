# Ablations of the land shader, each built and timed (abl.sh), the sources restored after each: python3 abl_all.py
import subprocess, sys
W = '/home/user/primordial-simulation/.claude/worktrees/agent-ab76b80ac68603520'
S = '/tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/rv'
def sub(path, a, b):
    s = open(W + '/' + path).read()
    assert s.count(a) == 1, (path, a[:60], s.count(a))
    open(W + '/' + path, 'w').write(s.replace(a, b))
NEAR = ('web/shaders/land.wgsl', '  let near = g.d < max(CHANNEL_NEAR * banks, line_most(e)) + 2.0 * e;', '  let near = false;')
C1 = ('web/shaders/lib/land.wgsl', '  let shaped = !unseen(BED_VARY * BED_MOST_M);', '  let shaped = false;')
C2a = ('web/shaders/lib/colour.wgsl', '    let ice = ice_cover(p.snow, p.depth, p.at, p.e);', '    let ice = 0.0;')
C2b = ('web/shaders/lib/colour.wgsl', '    let rgb = ice_on(water, ice, p.depth, p.at, p.e);', '    let rgb = water;')
C3 = ('web/shaders/lib/colour.wgsl', '  if (styled()) {\n    let thin = 1.0 - ease(0.0, SCREE_LOOSE_M, p.loose);', '  if (p.e < 0.0) {\n    let thin = 1.0 - ease(0.0, SCREE_LOOSE_M, p.loose);')
B1 = ('web/shaders/lib/river.wgsl', '  let wide = max(ch.bed, line);\n', '  if (s.dry.e > 0.0) { return out; }\n  let wide = max(ch.bed, line);\n')
B2 = ('web/shaders/lib/river.wgsl', '  if (styled() && p.stony > 0.0 && !unseen(STONE_MOST_M / STONE_PX)) {', '  if (p.e < 0.0 && p.stony > 0.0 && !unseen(STONE_MOST_M / STONE_PX)) {')
B3 = ('web/shaders/lib/river.wgsl', '  if (resolved > 0.0) {\n    // (Below a step', '  if (resolved > 1.0) {\n    // (Below a step')
runs = {'c1': [NEAR, C1], 'c2': [NEAR, C1, C2a, C2b], 'c3': [NEAR, C1, C2a, C2b, C3], 'b1': [B1], 'b2': [B2], 'b3': [B3]}
for name in sys.argv[1:] or runs:
    for e in runs[name]: sub(*e)
    try:
        print(subprocess.run(['bash', S + '/abl.sh', name], capture_output=True, text=True).stdout, flush=True)
    finally:
        subprocess.run(['git', 'checkout', 'web/shaders'], cwd=W)
