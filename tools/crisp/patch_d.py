import sys
p = sys.argv[1] + '/web/shaders/land.wgsl'
s = open(p).read()

def rep(a, b):
    global s
    assert a in s, a[:80]
    s = s.replace(a, b)

rep("""// A channel shows as a line of at least `LINE_PX` css pixels once its discharge passes the discharge that shows in the
// view (`land.time.w` times the metres a css pixel takes: tape/src/ground/channels.rs `SHOW_M`), as a map draws its
// rivers; wider as its discharge grows past that, as the channel law widens a channel, up to `LINE_MOST` times; it
// fades in from `LINE_FADE` times less, so no channel springs up as the view zooms.
const LINE_PX = 1.2;
const LINE_MOST = 2.5;
const LINE_FADE = 4.0;""", """// A channel shows as a line of at least `LINE_PX` css pixels once its discharge passes the discharge that shows in the
// view (`land.time.w` times the metres a css pixel takes: tape/src/ground/channels.rs `SHOW_M`), as a map draws its
// rivers; wider as its discharge grows past that, as the channel law widens a channel, up to `LINE_MOST` times, so a
// stream is a hair and the main river many times wider; it fades in from `LINE_FADE` times less, so no channel springs
// up as the view zooms.
const LINE_PX = 1.2;
const LINE_MOST = 6.0;
const LINE_FADE = 4.0;
// A channel's bed is as wide as the flow that shapes it, its floods: the bankfull discharge, the flood of every year or
// two, about `FLOOD` times the mean flow in humid lands (Leopold and Maddock 1953; Wolman and Miller 1960; Dunne and
// Leopold 1978: 2 to 10 times). This month's water runs in it between bars of washed gravel, its thread wandering
// across the bed's free width over `WANDER` bed widths along it, splitting about a bar where it wanders farthest (bars
// alternate every few widths: Leopold and Wolman 1957). Where the bed takes under `BED_PX` pixels it is all water.
const FLOOD = 5.0;
const WANDER = 5.0;
const BED_PX = 3.0;""")

rep("""// What is at a point: the ground without the channel's water (`dry`), the channel's water (`wet`) and the share of the
// pixel that water covers; and the share of the pixel standing liquid covers (`pool`): the pixel's footprint across
// the line where the depth crosses `FILM_M`, so a shore is a smooth line at any zoom, not the pixels' steps.
struct Spots { dry: Spot, wet: Spot, share: f32, pool: f32 }""", """// What is at a point: the ground without the channel's water (`dry`), the channel's water (`wet`) and the share of the
// pixel that water covers; the share the bars of its bed cover (`bars`, the dry spot as washed gravel); and the share
// of the pixel standing liquid covers (`pool`): the pixel's footprint across the line where the depth crosses `FILM_M`,
// so a shore is a smooth line at any zoom, not the pixels' steps.
struct Spots { dry: Spot, wet: Spot, share: f32, bars: f32, pool: f32 }""")

rep("""  var out = Spots(p, p, 0.0, pool);""", """  var out = Spots(p, p, 0.0, 0.0, pool);""")

rep("""  let drawn = max(bed.half, line);
  out.share = tent((drawn - g.d) / e) - tent((-drawn - g.d) / e);
  let resolved = ease(e, 2.0 * e, bed.half);
  let x = min(g.d / bed.half, 1.0);""", """  // The bed (`FLOOD`) and the thread of this month's water in it, `off` m from the line.
  let bank = 0.5 * land.channel.x * pow(FLOOD * q, land.channel.y);
  let free = max(bank - bed.half, 0.0);
  let off = free * ease(-0.1, 0.5, noise_turned(at / (2.0 * WANDER * bank), 63));
  let thread = abs(g.d - off);
  let in_bed = tent((max(bank, line) - g.d) / e) - tent((-max(bank, line) - g.d) / e);
  let water = tent((bed.half - thread) / e) - tent((-bed.half - thread) / e);
  let open = ease(BED_PX * e, 2.0 * BED_PX * e, bank) * (1.0 - ease(bank, 2.0 * bank, line));
  out.share = mix(in_bed, water, open);
  out.bars = max(in_bed - out.share, 0.0);
  let resolved = ease(e, 2.0 * e, bed.half);
  let x = min(thread / bed.half, 1.0);""")

rep("""  let spots = land_spots(at, metres_a_pixel(v.uv));
  var c = colour(spots.dry);""", """  let spots = land_spots(at, metres_a_pixel(v.uv));
  var c = colour(spots.dry);
  // The bars of a channel's bed: washed gravel, bare, as high over the water as it is deep.
  if (spots.bars > 0.0) {
    var bar = spots.dry;
    bar.sand = 1.0;
    bar.cover = vec4f(0.0);
    bar.over = spots.wet.depth;
    let b = colour(bar);
    c = Shaded(mix(c.rgb, b.rgb, spots.bars), mix(c.m, b.m, spots.bars), c.kind);
  }""")
open(p, 'w').write(s)
