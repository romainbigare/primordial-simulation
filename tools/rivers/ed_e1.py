def edit(p, pairs):
    t = open(p).read()
    for a, b in pairs:
        assert t.count(a) == 1, (p, a[:70], t.count(a))
        t = t.replace(a, b)
    open(p, 'w').write(t)

edit('web/shaders/lib/rock.wgsl', [
 ("""// A bed's width varies along it by up to `BED_VARY` either way, over lengths of `BED_VARY_M` (the second half as
// strong), each a whole share of the length the place along a channel is kept modulo.
const BED_VARY = 0.22; const BED_VARY_M = vec2f(32.0, 8.0);""", """// A bed's width varies along it by up to `BED_VARY` either way, over lengths of `BED_VARY_M` (the second half as
// strong), each a whole share of the length the place along a channel is kept modulo.
const BED_VARY = 0.22; const BED_VARY_M = vec2f(32.0, 8.0);
// Where a program is made for draws seen from afar (draw.wgsl `LEAST_E`), a bed's half width is taken as no more than
// this, m (the bed of a river of some 300 m³/s): what of a bed that wide cannot show at the program's pixels is
// compiled out of it.
const BED_MOST_M = 60.0;"""),
 ("""  let vary = 1.0 + BED_VARY * (noise_reach(g.along, BED_VARY_M.x, 0.0, 68) + 0.5 * noise_reach(g.along, BED_VARY_M.y, 0.0, 69)) / 1.5;""",
  """  var vary = 1.0;
  if (!unseen(BED_VARY * BED_MOST_M)) { vary += BED_VARY * (noise_reach(g.along, BED_VARY_M.x, 0.0, 68) + 0.5 * noise_reach(g.along, BED_VARY_M.y, 0.0, 69)) / 1.5; }"""),
])
edit('web/shaders/land.wgsl', [
 ("""  let open = ease(BED_PX * e, 2.0 * BED_PX * e, banks_of(q)) * (1.0 - ease(banks_of(q), 2.0 * banks_of(q), line));
  let r = reach_at(at, e, g, ch, line, open, stone);""", """  var open = 0.0;
  // (No bed takes `BED_PX` pixels in a program made for draws from this far: rock.wgsl `BED_MOST_M`.)
  if (BED_PX * least_e() < BED_MOST_M) { open = ease(BED_PX * e, 2.0 * BED_PX * e, banks_of(q)) * (1.0 - ease(banks_of(q), 2.0 * banks_of(q), line)); }
  let r = reach_at(at, e, g, ch, line, open, stone);"""),
 ("""  let banks = mix(1.0, bank_light(e, g, ch, r), open);""", """  var banks = 1.0;
  if (open > 0.0) { banks = mix(1.0, bank_light(e, g, ch, r), open); }"""),
 ("""  out.flow = resolved * flow_signs(e, g.along, x, ch, r.steep);""", """  if (resolved > 0.0) { out.flow = resolved * flow_signs(e, g.along, x, ch, r.steep); }"""),
])
edit('web/shaders/lib/river.wgsl', [
 ("""  let wide = BANK_M + BANK_SHARE * c.bed;
  let shown = shows_at(2.0 * wide, e);
  if (shown <= 0.0) { return 1.0; }""", """  let wide = BANK_M + BANK_SHARE * c.bed;
  if (unseen(2.0 * (BANK_M + BANK_SHARE * BED_MOST_M))) { return 1.0; }
  let shown = shows_at(2.0 * wide, e);
  if (shown <= 0.0) { return 1.0; }"""),
])
edit('web/shaders/lib/flow.wgsl', [
 ("""  let lanes = clamp(c.water / LANE_M, LANES.x, LANES.y);
  let shown = shows_at(2.0 * c.water / lanes, e);
  if (shown <= 0.0) { return f; }""", """  // (The widest lanes of the widest bed do not show in a program made for draws from far: rock.wgsl `BED_MOST_M`.)
  if (unseen(2.0 * BED_MOST_M / LANES.x)) { return f; }
  let lanes = clamp(c.water / LANE_M, LANES.x, LANES.y);
  let shown = shows_at(2.0 * c.water / lanes, e);
  if (shown <= 0.0) { return f; }"""),
])
