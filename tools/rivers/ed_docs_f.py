import re

def between(t, start, end, new):
    i = t.index(start); j = t.index(end, i)
    return t[:i] + new + t[j:]

p = 'docs/tape.md'
t = open(p).read()
t = between(t, "On gentle ground the line swings about that curve in a meander belt", "Each\nchannel reaches as far as it could be wide",
"""The line wanders about that curve (`tape/src/ground/wander.rs`) and swings about its wander in the bends of a
meander belt (`bends.rs`, `belt.rs`). The samples place a valley's line no closer than a share of a sample, so the
line wanders up to 0.4 of a sample to each side (half of that on a hard bed), in long swings of no fixed length: a
smooth noise along the way down the valley, a corner of its lattice every 1.6 samples and a finer one 2.7 times as
often, 0.4 as strong, whose slope runs on across the corners (gradient noise, Perlin 2002), none at a head, a join and
a sink. A bed is shaped by its floods, so the bends are sized by the bed's width; a tile is made once, so that width
and the bed's depth are stand-ins: the channel law's at the drained area times `BED_RUNOFF`, 5 times (`FLOOD`: the
bankfull flood, Leopold and Maddock 1953; Dunne and Leopold 1978) `TYPICAL_RUNOFF` (8.5e-9 m/s, Earth's mean: 0.27 m a
year, Dai and Trenberth 2002). The bends swing 3.5 bed widths to each side of the wander where nothing holds them
(belts are 4 to 20 widths across, Williams 1986): all of it where the reach's slope (over 2 samples up and 2 down the
channel) is under 0.01; over 0.03 the bed is hard and 0.35 of it is left (pool-riffle reaches meander, step-pool ones
do not: Montgomery and Buffington 1997; Schumm 1985), and between the two the change is gradual. They swing no wider
than the valley's floor (4 bed depths over the rise of the ground across the valley, the samples square to the
channel's way over the same reach) and than the channel's reach less its bed; the whole sway no wider than 0.4 of the
radius the valley's own line turns on and than 0.8 of a sample; none for a bed under 1 m, under standing liquid or at
a sink. The bends are walked down each main way from its head, from one apex to the next (a half bend, 24 steps):
along a half bend the curvature runs smoothly from the first apex's to the next one's turned the other way (a cubic of
the heading, 0 at both apexes: a curvature that falls evenly along the line where both are alike, as measured bends
do: Langbein and Leopold 1966), so the line has no kink and its curvature no jump. Each apex takes its place and its
radius from the place's draws (a function of the seed, the sample the bends began at and the half bends since): it
reaches 0.2 to 1 of the belt's half width (the smaller the more often), times a swell that falls to 0.08 over 4 half
bends (most reaches bend fully, some run nearly straight), on the other side from the last; it turns on a radius of
1.4 to 5 bed widths, the tighter the more often (2 to 3 in most grown bends: Hickin and Nanson 1975), gentler where
the swell is low. A half bend is as long as carries the line from its apex to the next (found by halving, 3 to 24
widths, its least up to 3 times longer by its own draw), no longer than keeps its heading within 1.85 rad of the
valley's line and carries it 2.5 widths down the valley: past a right angle a bend turns back on itself in a loop. So
no two bends are alike in length or swing. On a hard bed an apex turns on 0.4 to 12 widths (most turns gentle, one in
ten sharp), the heading keeps within 0.8 rad, and a half bend's least length is up to 12 times longer by its draw: the
line runs in nearly straight stretches from one short sharp turn to the next. A piece's walk starts as the last one's
ended, so the line has no break from sample to sample, and along it runs the line's own length from its head; a
tributary's and a sink's last piece draw back to their wander at their end. A tile draws a sway (the bends, the
wander) where its length (a bend's 11 bed widths, Leopold and Wolman 1960; a wander's 3.2 samples) is 8 texels or
more, and none under 4 (between it grows): shorter, it moves the line by less than a texel. """)
t = between(t, "and one a shell may draw as a line wider than itself (its drained area at least `SHOW_M` times\nthe texel's size over 40) 2.5 texels too.", " Of the channels within reach,",
"""and one a shell may draw as a line wider than itself (its drained area at least `SHOW_M` times
the texel's size over 40) as far as half the widest line the shell draws it as on these texels (the line law, `LINE_PX`
1.8 css pixels times its discharge over the one that shows to the power 0.5, up to 7 times, where a css pixel takes 3
texels and the discharge is 10 times `TYPICAL_RUNOFF`'s, and 1.5 texels more).""")
t = between(t, "holds it to a few centimetres, and a shell lays along it only signs that come again after that length), and the line's\ncurvature", "A shell finds the sample\nagain",
"""holds it to a few centimetres, and a shell lays along it only signs that come again after that length). A texel
between two texels (left and right, or above and below) that both hold one other channel, which drains more, is given
to that channel (`mend.rs`: a small channel that crosses the reach of a larger one lies nearer over its scale in a
strip a texel wide, which would notch the line a shell draws the larger one as). Each texel also keeps the distance,
signed by the side, to its own channel's line without its bends (its curve and its wander alone; the line from the same
head; 60,000 where none reaches), and a texel that holds no line takes its sample from that one: from afar a shell
draws a channel as a line wider than its bends, about the line without them, and close up the difference of the two
distances is the bend's swing, by which it draws the pool at the outer bank of a bend and the bar inside it. """)
open(p, 'w').write(t)

p = 'docs/numbers.md'
rows = open(p).read().split('\n')
def row(prefix, new):
    hits = [i for i, r in enumerate(rows) if r.startswith(prefix)]
    assert len(hits) == 1, (prefix, len(hits))
    rows[hits[0]] = new
row("| `LINE_PX` (1.2)", "| `LINE_PX` (1.8), `LINE_GROW` (0.5), `LINE_MOST` (7), `CSS_TEXELS_MOST` (3), `LINE_MARGIN` (1.5 texels), `tape/src/ground/channels.rs`; `LINE_FADE` (4), `LINE_BENDS` (2), `web/shaders/land.wgsl`; `UNBENT_SLOPE` (0.3 to 1.2), `web/shaders/lib/rock.wgsl` | css pixels; none; none; texels; texels; none; none; none | a channel past the discharge that shows is a line at least this wide, wider as its discharge over that grows to the power `LINE_GROW` up to `LINE_MOST` times, and fades in from `LINE_FADE` times less discharge; a tile holds a line as far as half the widest the law makes it where a css pixel takes `CSS_TEXELS_MOST` texels, and `LINE_MARGIN` more; a line wider than its bed is drawn toward the channel's line without its bends, all the way from `LINE_BENDS` times the bed's width (the blend of the two distances taken over its slope, within `UNBENT_SLOPE`) (drawing only) | none: a line of a map, plain on a screen of either pixel ratio; a river widens down its way and below each tributary, a stream is a hair and the main river several times wider; the line is as wide as a bed of a few metres where a view with a scale bar of 50 m draws that bed itself; a line wider than about a sixth of a bend's length (bends are 11 widths long) would swallow the bend | tuned, by eye |")
row("| `BEND_WIDTHS` (11), `BELT_WIDTHS` (8)", "| `BEND_WIDTHS` (11), `BELT_WIDTHS` (3.5), `HARD_SHARE` (0.35), `WANDER_SAMPLES` (0.4), `WANDER_HARD` (0.5), `BENDS_BELOW` (0.01), `STRAIGHT_ABOVE` (0.03), `WALL_DEPTHS` (4), `TURN_SHARE` (0.4), `BEND_MOST` (0.8 samples), `BENDS_FROM_M` (1 m), `REACH_SAMPLES` (2), `tape/src/ground/bends.rs`; `WANDER_EVERY` (1.6 samples), `WANDER_FINE` (2.7), `WANDER_FINE_SHARE` (0.4), `tape/src/ground/wander.rs`; `BEND_FROM` (4) and `BEND_TEXELS` (8 texels), `STEP_TEXELS` (0.5), `CALM_STEPS` (16), `tape/src/ground/lines.rs` | bed widths; share; samples; slope; bed depths; m; texels | a drawn channel's sway: its bends' wavelength, by which a tile draws them (none under `BEND_FROM` texels, all from `BEND_TEXELS`); how far they swing to each side of its wander where nothing holds them, and the share left on a hard bed (a reach steeper than `STRAIGHT_ABOVE`; all of it under `BENDS_BELOW`); how far the line wanders to each side of its valley's line, the share left on a hard bed, and the wander's lattice (a corner every `WANDER_EVERY` samples, a finer one `WANDER_FINE` times as often and `WANDER_FINE_SHARE` as strong); the bends held where the ground across the valley stands `WALL_DEPTHS` bed depths over the channel, the sway within `TURN_SHARE` of the radius the valley's line turns on and `BEND_MOST` of a sample; none for a bed under `BENDS_FROM_M`; the slope and the floor read over `REACH_SAMPLES` up and down the channel; a line's straight pieces no shorter than `STEP_TEXELS`, a piece without bends in at most `CALM_STEPS` (drawing only) | meander wavelength 7 to 15 widths, about 11 (Leopold and Wolman 1960); belts 4 to 20 widths across (Williams 1986); pool-riffle channels lie below a slope of about 0.015, step-pool and cascade ones above about 0.03 (Montgomery and Buffington 1997); a bank a few times as high as the bed is deep is cut back by the bends, a higher one is the valley's wall; the samples place a valley's line no closer than a share of a sample | anchored |")
row("| `STEPS` (24), `HEADING_MOST` (1.85 rad)", "| `STEPS` (24), `HEADING_MOST` (1.85 rad), `RADIUS_WIDTHS` (1.4 to 5), `REACH_SHARE` (0.2 to 1), `REACH_SKEW` (1.4), `FAR_LEAST` (0.3), `SWELL_BENDS` (4), `SWELL_LEAST` (0.08), `ARC_WIDTHS` (3 to 24), `ARC_VARY` (3), `ADVANCE_WIDTHS` (2.5), `HARD_RADIUS` (0.4 to 12), `HARD_HEADING` (0.8 rad), `HARD_ARC` (1.5), `HARD_ADVANCE` (1.2), `HARD_VARY` (12), `STRAIGHT_WIDTHS` (6), `ARC_HALVINGS` (14), `tape/src/ground/belt.rs`; `ALONG_WRAP_M` (128 m), `tape/src/ground/channels.rs` | rad; bed widths; share; half bends | the walk of a channel's bends from apex to apex: the steps of a half bend; the most its heading turns from the valley's line; an apex's radius, its reach as a share of the belt's half width (the smaller the more often) and the least it lies across from the last; the belt's swell along the way; a half bend's length (its least up to `ARC_VARY` times longer by its own draw), the least it carries the line down the valley, and a straight stretch's pieces; the same on a hard bed, where most turns are gentle and one in ten is sharp; the length a texel's place along its channel is kept modulo (drawing only) | bends turn on 2 to 3 widths (Hickin and Nanson 1975) and their curvature falls evenly along them (Langbein and Leopold 1966); step-pool and cascade reaches run nearly straight between sharp turns (Montgomery and Buffington 1997) | tuned, by eye |")
i = [k for k, r in enumerate(rows) if r.startswith("| `BED_VARY` (0.22 over")]
assert len(i) == 1
r = rows[i[0]]
a = "`INNER_MOST` (0.8), `BEND_FULL` (0.4), "
assert a in r
r = r.replace(a, "`BEND_OFF` (0.5 to 3 half widths), ")
a = "a bed's width varies by up to 22% along it and reaches no farther inside a bend than 0.8 of the radius its line turns on; in a full bend (curvature times bed width 0.4) the water is a pool"
assert a in r
r = r.replace(a, "a bed's width varies by up to 22% along it; a bend shows from where the line has swung half the bed's half width from its line without bends and is full from 3 of them; in a full bend the water is a pool")
rows[i[0]] = r
open(p, 'w').write('\n'.join(rows))

p = 'README.md'
t = open(p).read()
a = "and none a mesh of rills), wider as its discharge grows, to the power 0.75 up to 4 times, so the network's order reads."
assert a in t
t = t.replace(a, "and none a mesh of rills), wider as its discharge grows, to the power 0.5 up to 7 times, so the network's order reads\nand a river widens down its way; a line wider than its bed runs along the channel's line without its bends, which\ncome in as the view nears.")
a = "bare of plants, and on gentle ground it swings in a meander belt\n(bends of many sizes, some reaches nearly straight, here and there a loop: `tape/src/ground/belt.rs`)."
assert a in t
t = t.replace(a, "bare of plants; its line wanders about its valley's and on gentle ground swings in a meander belt\n(bends of many sizes, some reaches nearly straight, here and there a loop; on a steep reach nearly straight stretches\nbetween short sharp turns: `tape/src/ground/belt.rs`).")
open(p, 'w').write(t)
