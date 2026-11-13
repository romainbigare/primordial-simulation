# The flow-signs piece as committed: the final flow.wgsl without the far programs' guard, the final tint of fast water,
# and its docs.
S = '/tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/rv/'
t = open(S + 'final/web/shaders/lib/flow.wgsl').read()
a = """  // (The widest lanes of the widest bed do not show in a program made for draws from far: rock.wgsl `BED_MOST_M`.)
  if (unseen(2.0 * BED_MOST_M / LANES.x)) { return f; }
"""
assert t.count(a) == 1
open('web/shaders/lib/flow.wgsl', 'w').write(t.replace(a, ''))

p = 'web/shaders/land.wgsl'
t = open(p).read()
a = "const REFLECT_STREAK = 0.25; const FAST = 0.4; const FAST_WATER = vec3f(0.10, 0.36, 0.34);"
assert t.count(a) == 1
open(p, 'w').write(t.replace(a, "const REFLECT_STREAK = 0.25; const FAST = 0.25; const FAST_WATER = vec3f(0.06, 0.34, 0.36);"))

p = 'docs/numbers.md'
lines = open(p).read().split('\n')
k = [i for i, l in enumerate(lines) if l.startswith('| `BED_VARY` (0.22')]
assert len(k) == 1
a = "`STEEP_STONE` (0.12 to 0.5 m), `STREAK_W_M` (1.2 m), `STREAK_M` (8 m), `STREAK` (0.6), `WHITE_M` (4 m), `FOAM_STEEP` (0.55), `web/shaders/lib/river.wgsl`; `REFLECT_STREAK` (0.25), `STONES_OUT` (1.2), `web/shaders/land.wgsl`"
assert lines[k[0]].count(a) == 1
lines[k[0]] = lines[k[0]].replace(a, "`STEEP_STONE` (0.12 to 0.5 m), `web/shaders/lib/river.wgsl`; `STONES_OUT` (1.2), `web/shaders/land.wgsl`")
a = "; its edges ragged; a steep reach moves stones of 0.12 to 0.5 m; streaks along the flow and white water on steep reaches (drawing only)"
assert lines[k[0]].count(a) == 1
lines[k[0]] = lines[k[0]].replace(a, "; its edges ragged; a steep reach moves stones of 0.12 to 0.5 m (drawing only)")
lines.insert(k[0] + 2, "| `LANE_M` (0.45 m), `LANES` (3 to 12), `STREAK_M` (8 m), `STREAK` (0.55), `FLECK_M` (2 m), `FLECK` (0.3), `STEP_M` (8 m), `STEP_WAVE` (0.7), `STEP_FADE` (0.3), `STEP_PATCH` (2), `FOAM_STEEP` (0.9), `web/shaders/lib/flow.wgsl`; `REFLECT_STREAK` (0.25), `FAST` (0.25), `FAST_WATER`, `web/shaders/land.wgsl`; `FLOW_FPS` (6), `FLOW_M_S` (0.35 m/s), `FLOW_SHOWS_E` (0.6 m), `FLOW_PX` (4), `web/src/land.ts` | m; share; 1/s; m/s; pixels | the signs of a channel's flow, laid along its line: streaks in lanes 0.45 m wide (3 to 12 across half the water) and dashes 8 m long, many on a riffle and few on a pool; a riffle's white flecks 2 m long; on a steep reach white water from steps 8 m apart, on lines that waver by 0.7 of that, fading over 0.3 of the way to the next, in patches 2 steps long; fast water a quarter of the way to green-blue. They slide down the channel in 6 steps a second: 0.35 m/s on a pool, twice on a riffle, up to four times on a steep reach, while a channel in view shows them (a pixel of 0.6 m or less, water 4 pixels wide) and the map is drawn at the screen's own pixels (drawing only, the Earth view only) | step-pool reaches step every one to four widths (Montgomery and Buffington 1997); lowland streams run at 0.3 to 1 m/s (from memory) | tuned, by eye |")
open(p, 'w').write('\n'.join(lines))

p = 'README.md'
t = open(p).read()
a = """dark thread where it is deepest; a bar is pale gravel, wet at the water's edge, with the drift lines the falling
water leaves and tufts of plants on its top; a bank is a low scarp the sun lights or shades."""
b = """dark thread where it is deepest; a bar is pale gravel, wet at the water's edge, with the drift lines the falling
water leaves and tufts of plants on its top; a bank is a low scarp the sun lights or shades. The water shows which
way it runs (`web/shaders/lib/flow.wgsl`): streaks along the flow, many over a riffle and few on a pool, and on a
steep reach white water that starts at a step and fades down the flow. These signs slide down the channel in six
steps a second, faster where the reach is steeper, and come round with no jump; the page draws those frames only
while a channel in view shows them and it keeps up at the screen's own pixels, else none until something changes."""
assert t.count(a) == 1
open(p, 'w').write(t.replace(a, b))
