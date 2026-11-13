def edit(p, pairs):
    t = open(p).read()
    for a, b in pairs:
        assert t.count(a) == 1, (p, a[:70], t.count(a))
        t = t.replace(a, b)
    open(p, 'w').write(t)

edit('web/shaders/lib/style.wgsl', [
 ("""// Lakes and rivers are metres deep where the sea is tens: each is coloured as if `LAKE_SCALE` times as deep plus
// `LAKE_SHALLOWEST_M`, no deeper than `LAKE_DEEPEST_M` (an open blue, not the ocean's dark), and lighter towards its
// shore: as if shallower where it is less than a rim's depth deep (`LAKE_RIM_M`), the shallower the nearer the shore
// (its square): a band as wide as the bed's slope makes it, wide where the bed is gentle and no narrower than
// `LAKE_RIM_PX` pixels where it is steep, so every shore has its light shallows.
const LAKE_SCALE = 1.5; const LAKE_SHALLOWEST_M = 5.0; const LAKE_DEEPEST_M = 45.0;
const LAKE_RIM_M = 0.3; const LAKE_RIM_PX = 2.5;""",
  """// Lakes and rivers are metres deep where the sea is tens: each is coloured as if `LAKE_SCALE` times as deep, and by
// one law (`river_seen`): lighter towards its shore, where it is less than a rim's depth deep (`LAKE_RIM_M`): a band
// as wide as the bed's slope makes it, wide where the bed is gentle and no narrower than `LAKE_RIM_PX` pixels where
// it is steep, so every shore has its light shallows. Under lake and river alike the bed shows `BED_SHOWN` of its
// light: it reads through the shallows, and the water's own colour stays.
const LAKE_SCALE = 1.5;
const LAKE_RIM_M = 0.3; const LAKE_RIM_PX = 2.5;
const BED_SHOWN = 0.8;"""),
])
edit('web/shaders/lib/river.wgsl', [
 ("""// In the style the bed under running water shows this share of its light: its cobbles read through the shallows.
const BED_SHOWN = 0.8;
""", ""),
])
edit('web/src/land.ts', [
 (""".vec4(s.across, s.down, s.depth, devicePixelRatio)""", """.vec4(s.across, s.down, s.depth, drawn)"""),
])
edit('tape/src/ground/channels.rs', [
 ("""/// The line a shell draws a channel as from afar: `LINE_PX` css pixels wide where its discharge is the one that shows
/// (`SHOW_M`), wider as its discharge over that to the power `LINE_GROW` (more steeply than the channel law widens a
/// channel), up to `LINE_MOST` times. A map's choice, set by eye: a stream is a hair and the main river many times
/// wider, a river widens down its way and below each tributary, and the line is as wide as a bed of a few metres
/// where a view with a scale bar of 50 m draws that bed itself.
pub const LINE_PX: f64 = 1.8;
pub const LINE_GROW: f64 = 0.5;
pub const LINE_MOST: f64 = 7.0;""",
  """/// The least width a shell draws a channel at: `LINE_PX` css pixels where its discharge is the one that shows
/// (`SHOW_M`), wider as its discharge over that to the power `LINE_GROW`, up to `LINE_MOST` times. A map's choice: a
/// channel narrower than this on screen stays in sight as a thin line, a stream a hair and a river a little more,
/// and no line is much wider than the bed it stands for (a channel is drawn as wide as it is wherever that is wider).
pub const LINE_PX: f64 = 1.0;
pub const LINE_GROW: f64 = 0.5;
pub const LINE_MOST: f64 = 2.5;"""),
])
