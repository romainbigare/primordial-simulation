def edit(p, pairs):
    t = open(p).read()
    for a, b in pairs:
        assert t.count(a) == 1, (p, a[:70], t.count(a))
        t = t.replace(a, b)
    open(p, 'w').write(t)

edit('web/shaders/lib/crag.wgsl', [
 ("const STONE_SEEN = 1.5; const COBBLE_M = 0.22; const BED_SUNK = 0.3;", "const STONE_SEEN = 1.5; const COBBLE_M = 0.45; const BED_SUNK = 0.3;"),
])
edit('web/shaders/lib/stone.wgsl', [
 ("const STONE_SHADOW = 0.45; const STONE_SHADOW_REACH = 0.45;", "const STONE_SHADOW = 0.3; const STONE_SHADOW_REACH = 0.4;"),
])
edit('web/shaders/lib/river.wgsl', [
 ("""  // The top of a bar is the plants' again, in tufts.
  let tufts = shows_at(BAR_TUFT_M, e);
  if (tufts > 0.0 && r.up > BAR_GROWN) {
    in_bed = max(in_bed * (1.0 - open * tufts * ease(BAR_GROWN, 1.0, r.up) * ease(-0.3, 0.3, noise_turned(at / BAR_TUFT_M, 79))), water);
  }""", """  // The top of a bar is the plants' again, in tufts with an edge of their own: more of them the nearer the bank.
  let tufts = shows_at(BAR_TUFT_M, e);
  if (tufts > 0.0 && r.up > BAR_GROWN) {
    let grown = noise_turned(at / BAR_TUFT_M, 79) + 1.2 * ease(BAR_GROWN, 1.0, r.up) - 0.6;
    in_bed = max(in_bed * (1.0 - open * tufts * tent(grown * BAR_TUFT_M / e)), water);
  }"""),
])
edit('web/shaders/lib/rock.wgsl', [
 ("""  let middle = outer * bend * (law * vary - water);""", """  let middle = outer * ease(0.0, POOL_AT_BANK, bend) * (law * vary - water);"""),
 ("""const BEND_FULL = 0.4; const POOL_NARROW = 0.2; const RIFFLE_WIDE = 0.3;""", """const BEND_FULL = 0.4; const POOL_NARROW = 0.2; const RIFFLE_WIDE = 0.3;
// The water lies against the outer bank from this much of a full bend on.
const POOL_AT_BANK = 0.6;"""),
])
