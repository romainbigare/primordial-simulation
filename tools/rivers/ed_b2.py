def edit(p, pairs):
    t = open(p).read()
    for a, b in pairs:
        assert t.count(a) == 1, (p, a[:70], t.count(a))
        t = t.replace(a, b)
    open(p, 'w').write(t)

edit('web/shaders/lib/river.wgsl', [
 ("""// A bar is washed gravel and sand: its tone varies by up to `BAR_MOTTLE` over patches `BAR_PATCH_M` across and finer;
// it is wet, so darker, by `BAR_WET` at the water's edge, drying over `BAR_WET_M` up the bar; the falling water leaves
// drift lines along the flow every `STRAND_M` up it, `STRAND_W` of that wide and `STRAND` darker, wavering along the
// reach over `STRAND_WAVE_M`; and plants take its top from `BAR_GROWN` of the way up to the bank, in tufts
// `BAR_TUFT_M` across.
const BAR_MOTTLE = 0.25; const BAR_PATCH_M = vec2f(2.5, 0.6);
const BAR_WET = 0.4; const BAR_WET_M = 0.7;
const STRAND_M = 0.9; const STRAND_W = 0.22; const STRAND = 0.22; const STRAND_WAVE_M = 8.0;""",
  """// A bar is washed gravel and sand: its tone varies by up to `BAR_MOTTLE` over patches `BAR_PATCH_M` across and finer;
// the falling water leaves drift lines along the flow every `STRAND_M` up it, `STRAND_W` of that wide and `STRAND`
// darker, wavering along the reach over `STRAND_WAVE_M`; and plants take its top from `BAR_GROWN` of the way up to the
// bank, in tufts `BAR_TUFT_M` across.
const BAR_MOTTLE = 0.2; const BAR_PATCH_M = vec2f(2.5, 0.6);
const STRAND_M = 0.9; const STRAND_W = 0.2; const STRAND = 0.28; const STRAND_WAVE_M = 8.0;"""),
 ("""// The tone of a bar at `at`, seen at `e` m a pixel, `along` m along its channel, at the reach's point `r`: its mottle,
// its wet rim and its drift lines (each where it shows).""", """// The tone of a bar at `at`, seen at `e` m a pixel, `along` m along its channel, at the reach's point `r`: its mottle
// and its drift lines (each where it shows)."""),
 ("""  t *= 1.0 - BAR_WET * exp(-r.edge / BAR_WET_M);
""", ""),
])
edit('web/shaders/land.wgsl', [
 ("""// A channel's bars are its bed's make washed (`BAR_LIFT` times as light), and sand where its flow moves no stones
// bigger than `BAR_SAND_STONE` m: gravel on most reaches, sand and silt on the gentlest. A bar rises from the water's
// edge at `BAR_RISE` (rise over run), which is how far up it the water's fringe wets it.
const BAR_LIFT = 1.5; const BAR_SAND_STONE = 0.03; const BAR_RISE = 0.5;""", """// A channel's bars are its bed's make washed (`BAR_LIFT` times as light, `BAR_PALE` of the way to pale sand), and sand
// where its flow moves no stones bigger than `BAR_SAND_STONE` m: gravel on most reaches, sand and silt on the
// gentlest. A bar rises from the water's edge at `BAR_RISE` (rise over run), which is how far up it the water's fringe
// wets and darkens it; past that fringe its gravel drains dry.
const BAR_LIFT = 1.7; const BAR_PALE = 0.35; const BAR_SAND_STONE = 0.03; const BAR_RISE = 0.6;"""),
 ("""  let washed = min(bed_make_at(at) * BAR_LIFT, vec3f(1.0));""", """  let washed = mix(min(bed_make_at(at) * BAR_LIFT, vec3f(1.0)), BEACH_SAND, BAR_PALE);"""),
 ("""      p.cover = vec4f(0.0);
      p.over = spots.bar_over;""", """      p.cover = vec4f(0.0);
      p.over = spots.bar_over;
      p.wet = 0.0;"""),
])
