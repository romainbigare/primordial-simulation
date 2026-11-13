# Write web/shaders/land.wgsl as it is without piece C's hunks (the stones) to OUT, by reversing ed_c1.py's edits of it.
# python3 split_bc.py OUT
import sys
t = open('web/shaders/land.wgsl').read()
pairs = [
 ("""// A channel's stones lie on its bed and its bars, out to this many of its bed's half widths from its line.
const STONES_OUT = 1.2;""", """// A channel's stones lie on its bed and its bars, in `IN_CHANNEL` of the places they could, fewer where it moves
// only sand (`BAR_SAND_STONE`); beside a steep reach they lie out to `STONES_OUT` of its bed's half widths from its
// line, thinning. A channel wears its stones round as it carries them: angular where it carries `WORN_Q.x` m³/s (a
// head's rill), round from `WORN_Q.y` (the travel grows with the area drained, and so does the discharge). Up a bar
// they are finer: `BAR_FINES` smaller at its top.
const IN_CHANNEL = 0.6; const STONES_OUT = 1.8; const WORN_Q = vec2f(0.005, 0.3); const BAR_FINES = 0.5;"""),
 ("""  p.stone = select(0.0, stone_of(g.channels.xy), ch.bed > 0.0 && g.d < STONES_OUT * ch.bed);
  // The bed's colour where it shows or its stones lie.
  if (p.rock > 0.0 || p.stone > 0.0) { p.bed = bed_make_at(at); }""", """  // Its channel's stones: on its bed, and beside it where the reach is steep.
  p.stone = select(0.0, stone_of(g.channels.xy), ch.bed > 0.0);
  let beside = ease(STEEP_STONE.x, STEEP_STONE.y, p.stone) * (1.0 - ease(ch.bed, STONES_OUT * ch.bed, g.d));
  p.stony = IN_CHANNEL * ease(0.0, BAR_SAND_STONE, p.stone) * select(beside, 1.0, g.d < ch.bed);
  p.worn = ease(log(WORN_Q.x), log(WORN_Q.y), log(max(q, 1e-9)));
  // The bed's colour where it shows or stones of it lie: a channel's, and the scree's on thin loose ground.
  if (p.rock > 0.0 || p.stony > 0.0 || loose < SCREE_LOOSE_M) { p.bed = bed_make_at(at); }"""),
 ("""  out.bar = gravel * bar_tone(at, e, g.along, r);
  out.bar_over = BAR_RISE * r.edge;""", """  out.bar = gravel * bar_tone(at, e, g.along, r);
  out.bar_over = BAR_RISE * r.edge;
  out.dry.stone *= 1.0 - BAR_FINES * r.up;"""),
 ("crag.wgsl ice.wgsl", "crag.wgsl stone.wgsl ice.wgsl"),
]
for a, b in pairs:
    assert t.count(b) == 1, b[:60]
    t = t.replace(b, a)
open(sys.argv[1], 'w').write(t)
