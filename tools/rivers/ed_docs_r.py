def edit(p, pairs):
    t = open(p).read()
    for a, b in pairs:
        assert t.count(a) == 1, (p, a[:70], t.count(a))
        t = t.replace(a, b)
    open(p, 'w').write(t)

edit('docs/tape.md', [
 ("""250 m (the floor's share f eased from 1 to 0, its weight f / (1 - f) against the ground's 1, the lines near a texel
summed), the floor running""", """250 m (the floor's share f eased from 1 to 0, its weight f / (1 - f) against the ground's 1; each straight piece of
a line gives a texel its floor at the texel's nearest point of it with that weight, as much as it is long over a
quarter of 250 m, and the pieces near a texel are summed, so the floor is smooth across the ground and has no step
where a line bends), the floor running"""),
 ("""Chaikin 1974), as many each way as the stem has, so a head, the end of a stem at a body and the sample a tributary
joins keep their place""", """Chaikin 1974), as many each way as the stem has (a stem runs on through a body's samples, along its floor), so a
head, a stem's end and the sample a tributary joins keep their place"""),
 ("""smooth noise along the way down the valley, a corner of its lattice every 1.6 samples and a finer one 2.7 times as
often, 0.4 as strong, whose slope runs on across the corners (gradient noise, Perlin 2002), none at a head, a join and
a sink.""", """smooth noise along the way down the valley, a corner of its lattice every 1.6 samples and four finer octaves, each
2.7 times as often as the last and 0.4 as strong (a channel takes those whose corners lie 8 of its bed's widths apart
or more, fully from 16: a wide bed's finer turns are its bends', and a rill turns every few metres), the sum taken
twice and held smoothly within the most, whose slope runs on across the corners (gradient noise, Perlin 2002), none at
a head, a join and a sink. Every channel wanders, a rill and a body's floor too: none runs straight."""),
 ("""radius the valley's own line turns on and than 0.8 of a sample; none for a bed under 1 m, under standing liquid or at
a sink.""", """radius the valley's own line turns on and than 0.8 of a sample; no sway at a sink, and no bends (the wander alone)
for a bed under 1 m and under standing liquid."""),
 ("""holds it to a few centimetres, and a shell lays along it only signs that come again after that length). A texel
between two texels (left and right, or above and below) that both hold one other channel, which drains more, is given
to that channel (`mend.rs`: a small channel that crosses the reach of a larger one lies nearer over its scale in a
strip a texel wide, which would notch the line a shell draws the larger one as).""",
  """holds it to a few centimetres, and a shell lays along it only signs that come again after that length). A texel
within 2 texels, to its left and to its right or above and below, of texels that both hold one other channel, which
drains more (with none but its own channel's between), is given to that channel (`mend.rs`: a small channel that
crosses the reach of a larger one lies nearer over its scale in a strip a texel or two wide, which would notch the line
a shell draws the larger one as)."""),
])

p = 'docs/numbers.md'
rows = open(p).read().split('\n')
def sub(prefix, pairs):
    hits = [i for i, r in enumerate(rows) if r.startswith(prefix)]
    assert len(hits) == 1, (prefix, len(hits))
    r = rows[hits[0]]
    for a, b in pairs:
        assert r.count(a) == 1, (a, r.count(a))
        r = r.replace(a, b)
    rows[hits[0]] = r
sub("| `FLOOD` (5), `tape/src/ground/bends.rs`; `STATION_EXP` (0.26), `web/shaders/lib/rock.wgsl`;", [
 ("| `FLOOD` (5), `tape/src/ground/bends.rs`; `STATION_EXP` (0.26), `web/shaders/lib/rock.wgsl`;", "| `FLOOD` (5) and `STATION_EXP` (0.26), `tape/src/ground/bends.rs`;"),
])
sub("| `VALLEY_FROM` (1 km²) and `VALLEY_M` (250 m), `CHORD_SHARE` (1/32)", [
 ("`CHORD_SHARE` (1/32), `tape/src/ground/valley.rs` | m²; m; share |", "`CHORD_SHARE` (1/32), `PIECE_SHARE` (0.25), `tape/src/ground/valley.rs`; `STRIP` (2 texels), `tape/src/ground/mend.rs` | m²; m; share; share; texels |"),
 ("and the shortest piece of a line a valley follows, as a share of that (drawing only)", "the shortest piece of a line a valley follows, as a share of that, and the length of a piece that weighs as one whole line at a point (the pieces about a point add up, so the floor has no step where a line bends); the widest strip of a small channel across a larger one's reach that is given to the larger one (drawing only)"),
])
sub("| `BEND_WIDTHS` (11), `BELT_WIDTHS` (3.5)", [
 ("`WANDER_EVERY` (1.6 samples), `WANDER_FINE` (2.7), `WANDER_FINE_SHARE` (0.4), `tape/src/ground/wander.rs`", "`WANDER_EVERY` (1.6 samples), `WANDER_OCTAVES` (5), `WANDER_FINE` (2.7), `WANDER_FINE_SHARE` (0.4), `TURN_WIDTHS` (8), `WANDER_GAIN` (2), `tape/src/ground/wander.rs`"),
 ("`STEP_TEXELS` (0.5), `CALM_STEPS` (16)", "`STEP_TEXELS` (0.5), `CALM_STEPS` (48)"),
 ("and the wander's lattice (a corner every `WANDER_EVERY` samples, a finer one `WANDER_FINE` times as often and `WANDER_FINE_SHARE` as strong);", "and the wander's lattice (a corner every `WANDER_EVERY` samples, `WANDER_OCTAVES` octaves each `WANDER_FINE` times as often as the last and `WANDER_FINE_SHARE` as strong, a channel taking those whose corners lie `TURN_WIDTHS` of its bed's widths apart or more, their sum taken `WANDER_GAIN` times); every channel wanders, and a bed under `BENDS_FROM_M` or under standing liquid has no bends walked;"),
 ("none for a bed under `BENDS_FROM_M`; the slope", "the slope"),
])
sub("| `LANE_M` (0.45 m)", [
 ("`STEP_POOL` (0.6), `POOL_CALM` (0.8), `web/shaders/lib/flow.wgsl`", "`STEP_POOL` (0.6), `POOL_CALM` (0.8), `STEP_EDGE_M` (0.4 m), `web/shaders/lib/flow.wgsl`"),
 ("fading over 0.3 of the way to the next,", "beginning over 0.4 m (the place along a channel is known to a few centimetres, and a sharper edge would draw the texels' lattice) and fading over 0.3 of the way to the next,"),
])
sub("| `MOUTH_UNDER_M` (0.5 m)", [
 ("`FAN_DEPTH_M` (1.5 m), `web/shaders/land.wgsl`", "`FAN_DEPTH_M` (1.5 m), `FAN_BARE` (0.2), `web/shaders/lib/rock.wgsl`"),
 ("within `FAN_WIDTHS` of its bed's half widths of its line (drawing only)", "within `FAN_WIDTHS` of its bed's half widths of its line; no plant roots in the widened bed nor where `FAN_BARE` of a fan lies (drawing only)"),
])
open(p, 'w').write('\n'.join(rows))
