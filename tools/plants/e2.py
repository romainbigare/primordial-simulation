W = '/home/user/primordial-simulation/.claude/worktrees/agent-a257b588d76bf8a62/'
def edit(path, pairs):
    s = open(W + path).read()
    for a, b in pairs:
        assert a in s, (path, a[:70])
        s = s.replace(a, b)
    open(W + path, 'w').write(s)

edit('web/shaders/lib/colour.wgsl', [
("""// The colour of the ground of spot p, unlit: its own colour, tinted by the hand rules of cold, height and sand, and its bed
// where it shows; the plants on it, as far as liquid over their roots, snow and bare bed leave them (`stands_in`); and snow `snow`
// over all. The share of it that is bare, and its map class.""",
"""// The colour of the ground of spot p, unlit: its own colour, tinted by the hand rules of cold, height and sand, and its bed
// where it shows; its stones; snow `snow` over them; and over all the plants that stand out of it (its cover, as far as
// liquid over their roots leaves them, `above_water`; the cover's field holds how the snow buries them and how the bed
// holds them). The share of it that is bare, and its map class."""),
("""  if (p.rock > 0.5) { g.kind = ROCK; }
  let plants = p.cover.a * stands_in(p.depth, snow) * (1.0 - (1.0 - ON_BED) * p.rock);
  g.bare = 1.0 - plants;
  if (plants > 0.0) { g.rgb = mix(g.rgb, p.cover.rgb / p.cover.a, plants); }
  // Stones over it: scree, on bare bed, and in a channel; none on a beach.
  if (styled()) {""", """  if (p.rock > 0.5) { g.kind = ROCK; }
  let plants = p.cover.a * above_water(p.depth);
  g.bare = 1.0 - plants;
  // Stones over it: scree, on bare bed, and in a channel; none on a beach.
  if (styled()) {"""),
("""  if (plants >= 0.5) { g.kind = PLANTS; }
  g.rgb = mix(g.rgb, SNOW, snow);
  if (snow > 0.5) { g.kind = ICE; }
  return g;""", """  g.rgb = mix(g.rgb, SNOW, snow);
  if (snow > 0.5) { g.kind = ICE; }
  if (plants > 0.0) { g.rgb = mix(g.rgb, p.cover.rgb / p.cover.a, plants); }
  if (plants >= 0.5) { g.kind = PLANTS; }
  return g;"""),
])
s = open(W + 'web/shaders/lib/colour.wgsl').read()
import re
i = s.index('const ON_BED = 0.3;')
# show the comment above ON_BED to remove it with the const
j = s.rfind('\n\n', 0, i)
print(repr(s[j:i + len('const ON_BED = 0.3;') + 1]))
