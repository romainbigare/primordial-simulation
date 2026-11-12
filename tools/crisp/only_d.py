# Make web/shaders/land.wgsl hold part D only: undo patch_b's pairs for it, and lake_seen's old call.
W = '/home/user/primordial-simulation/.claude/worktrees/agent-abbbaac876ed7e5e9'
src = open('/tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/crisp/patch_b.py').read()
pairs = []
def patch(path, ps):
    if path.endswith('/web/shaders/land.wgsl'):
        pairs.extend(ps)
body = src.split("import sys, shutil", 1)[1]
body = body.replace("W = sys.argv[1]", "").replace("shutil.copy(", "(lambda *a: None)(")
body = body.split("def patch(path, pairs):")[0] + body.split("open(path, 'w').write(s)", 1)[1]
exec(body, {'patch': patch, 'W': W})
p = W + '/web/shaders/land.wgsl'
s = open(p).read()
for a, b in pairs:
    assert b in s, b[:60]
    s = s.replace(b, a)
s = s.replace("lake_seen(p.depth, RIVER_RIM_M)", "lake_seen(p.depth, 0.0, p.e, RIVER_RIM_M)")
s = s.replace("Beneath(soaked(p), p.canopy, loose_at(at), st.snow, st.snow_m)", "Beneath(soaked(p), p.canopy, p.loose, st.snow, st.snow_m)")
open(p, 'w').write(s)
