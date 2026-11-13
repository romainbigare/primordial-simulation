def edit(p, pairs):
    t = open(p).read()
    for a, b in pairs:
        assert t.count(a) == 1, (p, a[:70], t.count(a))
        t = t.replace(a, b)
    open(p, 'w').write(t)

edit('tape/src/ground/bends.rs', [
 ("""        let joint = |j: u32, runs: bool| if runs { 0.5 * (self.lean[k] + self.lean[j as usize]) } else { 0.0 };
        let most = joint(s.main[k], up) + (joint(s.to[k], on) - joint(s.main[k], up)) * t;""",
  """        let joint = |j: u32, runs: bool| if runs { 0.5 * (self.lean[k] + self.lean[j as usize]) } else { 0.0 };
        // (A head has no line to meet: it leans as its own sample does, so a rill of a sample or two wanders too.)
        let from = if up { joint(s.main[k], up) } else { self.lean[k] };
        let most = from + (joint(s.to[k], on) - from) * t;"""),
 ("""    /// none where the line starts or ends (a head; a join, a sink).""", """    /// none where the line ends (a join, a sink); a head leans as its own sample does."""),
])

edit('tape/src/ground/shore.rs', [
 ("""const SHORE_M: f64 = 4.0;""", """pub const SHORE_M: f64 = 4.0;"""),
])
edit('tape/src/ground/mod.rs', [
 ("""pub use bends::{FLOOD, STATION_EXP};""", """pub use bends::{FLOOD, STATION_EXP};
pub use shore::SHORE_M;"""),
])
edit('tape/src/lib.rs', [("PONDS_EXACT, SHOW_M, STATION_EXP,", "PONDS_EXACT, SHORE_M, SHOW_M, STATION_EXP,")])

edit('tape/src/table.rs', [
 ("""A sample of a pond
    /// carries no less than its main way upstream brings: a river runs on along a body's floor to its
    /// lowest sample, and a shell draws that run where the body is drawn dry or its shore lies farther than the
    /// samples' (under the body's liquid it does not show).""",
  """A sample of a pond
    /// that holds no liquid, or whose ground lies within `SHORE_M` under its pond's level, carries no less than its
    /// main way upstream brings: a river runs on along a dry body's floor, and to the shore a shell draws, which
    /// lies up to that far from the samples' (`ground::shore`). Deeper under a body's liquid no channel runs."""),
 ("""        let s = w.terrain_samples();""", """        let (s, levels) = (w.terrain_samples(), w.pond_levels());
        let shore = |k: usize| { let l = levels[s.pond[k] as usize]; l.is_nan() || s.h[k] >= l - SHORE_M };"""),
 ("""            if s.pond[k] != NONE && s.main[k] != NONE {""", """            if s.pond[k] != NONE && s.main[k] != NONE && shore(k) {"""),
 ("""use crate::ground::sample_index;""", """use crate::ground::{sample_index, SHORE_M};"""),
])

edit('docs/tape.md', [
 ("""none where that holds no liquid; a
sample of a pond carries no less than its main way upstream brings, so a river's line runs on along a body's floor and
a shell draws it where the body is drawn dry or its shore lies farther than the samples').""",
  """none where that holds no liquid; a
sample of a pond that holds no liquid, or whose ground lies within 4 m (`SHORE_M`) under its pond's level, carries no
less than its main way upstream brings, so a river's line runs on along a dry body's floor and to the shore a shell
draws; deeper under a body's liquid no channel runs)."""),
])
t = open('docs/tape.md').read()
a = "1.8 css pixels times its discharge over the one that shows to the power 0.5, up to 7 times"
assert t.count(a) == 1
t = t.replace(a, "1 css pixel times its discharge over the one that shows to the power 0.5, up to 2.5 times")
a = "from afar a shell\ndraws a channel as a line wider than its bends, about the line without them, and close up the difference of the two\ndistances is the bend's swing"
if a in t:
    t = t.replace(a, "the difference of the two\ndistances is the bend's swing")
else:
    i = t.index("draws a channel as a line wider than its bends, about the line without them")
    print('tape.md wording differs near', t[i - 80:i + 200])
open('docs/tape.md', 'w').write(t)

p = 'docs/numbers.md'
rows = open(p).read().split('\n')
def row(prefix, new):
    hits = [i for i, r in enumerate(rows) if r.startswith(prefix)]
    assert len(hits) == 1, (prefix, len(hits))
    rows[hits[0]] = new
row("| `LINE_PX` (1.8)", "| `LINE_PX` (1), `LINE_GROW` (0.5), `LINE_MOST` (2.5), `CSS_TEXELS_MOST` (3), `LINE_MARGIN` (1.5 texels), `tape/src/ground/channels.rs`; `LINE_FADE` (4), `FRINGE_PX` (0.5 to 2), `web/shaders/lib/river.wgsl` | css pixels; none; none; texels; texels; none; pixels | the least width a channel is drawn at once it passes the discharge that shows: `LINE_PX`, wider as its discharge over that grows to the power `LINE_GROW` up to `LINE_MOST` times, fading in from `LINE_FADE` times less discharge; a channel is drawn as wide as its bed wherever that is wider (width on screen = the larger of the two), so its width changes evenly with the zoom and with nothing else; a tile holds a line as far as half the widest the law makes it where a css pixel takes `CSS_TEXELS_MOST` texels, and `LINE_MARGIN` more; a channel wets the ground beside it only where its water is `FRINGE_PX` pixels wide or more (drawing only) | none: a line of a map stays in sight at 1 to 2.5 css pixels, and no line is much wider than the bed it stands for | tuned, by eye |")
row("| `MOUTH_UNDER_M` (0.5 m)", "| `MOUTH_UNDER_M` (0.5 m), `MOUTH_MOST` (4.5 samples), `MOUTH_STEP` (0.1 samples), `SHORE_M` (4 m), `tape/src/ground/shore.rs`; `MOUTH_FLARE` (1.5), `MOUTH_OVER_M` (1.5 m), `web/shaders/lib/rock.wgsl` | m; samples; share | a river's mouth: a line that ends in standing liquid runs on down the drawn shore's offset to where it lies `MOUTH_UNDER_M` under the level, no farther than `MOUTH_MOST`; a pond's sample within `SHORE_M` under its level (the band the drawn shore lies in) or of a dry pond carries its river's flow; the bed and its water widen by `MOUTH_FLARE` where the ground lies at the liquid's level (none from `MOUTH_OVER_M` over it), and no plant roots in the widened bed; nothing of a channel is drawn under standing liquid (drawing only) | none: the samples' shore and the smooth shore drawn lie up to a few samples apart; a channel widens where it meets standing water | tuned, by eye |")
i = [k for k, r in enumerate(rows) if r.startswith("| `CLIFF_FROM` (0.9)")]
assert len(i) == 1
r = rows[i[0]]
a = "a lake or a river coloured as if `LAKE_SCALE` times as deep (drawing only)"
assert a in r, r[:700]
rows[i[0]] = r.replace(a, "a lake or a river coloured as if `LAKE_SCALE` times as deep, by one law (`river_seen`: a lake's shore is a river's shallows, its open water a river's deep thread, the bed showing `BED_SHOWN` 0.8 of its light under both) (drawing only)")
open(p, 'w').write('\n'.join(rows))

t = open('README.md').read()
a = "wider as its discharge grows, to the power 0.5 up to 7 times, so the network's order reads\nand a river widens down its way; a line wider than its bed runs along the channel's line without its bends, which\ncome in as the view nears."
assert t.count(a) == 1
t = t.replace(a, "a little wider as its discharge grows (to the power 0.5, up to 2.5 times); a channel is drawn as wide as its bed\nwherever that is wider than its line, so its width changes evenly with the zoom.")
open('README.md', 'w').write(t)
