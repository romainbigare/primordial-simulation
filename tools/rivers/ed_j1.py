P = 'web/shaders/land.wgsl'
t = open(P).read()

def take(t, start, end):
    i = t.index(start); j = t.index(end, i)
    return t[:i] + t[j:], t[i:j]

# The channel's stones of a spot: a stage of its own.
old = t[t.index("  // Its channel's stones: on its bed, and beside it where the reach is steep."):t.index("  // The bed's colour where it shows or stones of it lie:")]
new_call = """  // Its channel's stones (`stony`), and the scree's place below the bed that sheds it: where the loose ground deepens
  // down the slope.
  let moved = select(0.0, mix(stone_of(reach), stone_of(next.s), next.share), ch.bed > 0.0);
  let deepens = vec2f(dpdx(loose), dpdy(loose));
  p = stony(p, g, ch, q, moved, max(STONE_SEEN * stone_of(reach), COBBLE_M));
  p.below = mix(BELOW_LEAST, 1.0, 0.5 - 0.5 * dot(deepens, g.slope) / max(length(deepens) * length(g.slope), 1e-9));
  p.snow_m = st.snow_m;
"""
t = t.replace(old, new_call)
stony = """// The spot `s` with its channel's stones: on the bed of channel `ch` (of discharge `q`, on ground `g`), and beside it
// where the reach is steep, with the walls and the fallen blocks of a steep reach. The largest drawn takes its size
// from the point's own sample (`cobble` m, in a step where two samples meet: the size sets where the stones lie); what
// each is of it follows the size the channel moves there (`moved` m), which grows evenly along it.
fn stony(s: Spot, g: Tiled, ch: Channel, q: f32, moved: f32, cobble: f32) -> Spot {
  var p = s;
  p.stone = STONE_STEP * cobble;
  p.small = clamp(max(STONE_SEEN * moved, COBBLE_M) / cobble, STONE_SMALL.x, STONE_SMALL.y);
  let steep_reach = ease(STEEP_STONE.x, STEEP_STONE.y, moved);
  let beside = steep_reach * (1.0 - ease(ch.bed, STONES_OUT * ch.bed, g.d));
  p.stony = IN_CHANNEL * ease(0.0, BAR_SAND_STONE, moved) * select(beside, 1.0, g.d < ch.bed);
  p.blocks = BLOCKS * steep_reach * select(beside, 1.0, g.d < ch.bed);
  p.rock = max(p.rock, WALL * beside);
  p.cover *= 1.0 - WALL * beside;
  p.worn = ease(log(WORN_Q.x), log(WORN_Q.y), log(max(q, 1e-9))) * (1.0 - STEEP_SHARP * steep_reach);
  return p;
}

"""
# The river's own parts move to lib/river.wgsl.
t, line_part = take(t, "// A channel shows as a line of at least `land.line.x` css pixels", "// A channel's bed is as wide as its floods make it (lib/rock.wgsl `banks_of`).")
t, bed_px = take(t, "// A channel's bed is as wide as its floods make it (lib/rock.wgsl `banks_of`).", "// The water within `EDGE_M` of its edge is lighter")
t, edge = take(t, "// The water within `EDGE_M` of its edge is lighter", "// Liquid standing shallower than this over the drawn ground")
t, river = take(t, "// The spots `s` of a point with its channel `ch` of discharge `q`, which moves stones of `stone` m", "// The colours a pixel mixes, each over the last by the share of the pixel it covers")
i = t.index("// What is at `at`, m from the map's north-west corner, seen at `e` m a pixel: its ground and its state")
t = t[:i] + stony + t[i:]
open(P, 'w').write(t)

R = 'web/shaders/lib/river.wgsl'
r = open(R).read()
r = r.rstrip('\n') + '\n\n' + line_part.rstrip('\n') + '\n' + bed_px.rstrip('\n') + '\n' + edge.rstrip('\n') + '\n\n' + river.rstrip('\n') + '\n'
open(R, 'w').write(r)
