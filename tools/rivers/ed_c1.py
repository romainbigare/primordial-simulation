def edit(p, pairs):
    t = open(p).read()
    for a, b in pairs:
        assert t.count(a) == 1, (p, a[:70], t.count(a))
        t = t.replace(a, b)
    open(p, 'w').write(t)

# The old maker goes; its places and sizes stay in crag.wgsl.
C = 'web/shaders/lib/crag.wgsl'
t = open(C).read()
i = t.index("// The stones of `density` (the share of cells that hold one) and `size` m at `at`")
t = t[:i].rstrip('\n') + '\n'
open(C, 'w').write(t)
edit(C, [
 ("""// it is steep (slope alone never bares the ground); stones lie where the loose ground is thin on a steep slope (scree)
// or bare, and in a channel as big as its power moves (rock.wgsl `stone_of`). Fixed in the world: nothing moves as the
// view pans or zooms, and what is smaller than a pixel fades into its mean.""", """// it is steep (slope alone never bares the ground); stones lie where the loose ground is thin on a steep slope (scree)
// or bare, and in a channel as big as its power moves (rock.wgsl `stone_of`), each made by the one maker of a stone
// (stone.wgsl). Fixed in the world: nothing moves as the view pans or zooms, and what is smaller than a pixel fades
// into its mean."""),
 ("""// Stones: on a slope from `SCREE_FROM` to `CLIFF_FROM` where the loose ground is thinner than `SCREE_LOOSE_M` (the
// scree below a face), `SCREE_STONE_M` across, at most `SCREE` of the places they could lie; on bare bed, `STONES_ON_BED` of
// them; in a channel, `IN_CHANNEL` of them, as big as it moves (no smaller than `PEBBLE_M`). None on a beach.
const SCREE_FROM = 0.6; const SCREE_LOOSE_M = 1.0; const SCREE_STONE_M = 0.6; const SCREE = 0.7;
const STONES_ON_BED = 0.25; const IN_CHANNEL = 0.6; const PEBBLE_M = 0.05;
// A stone lies one to a cell `STONE_GAP` stones wide, its size differing by up to `STONE_SIZES` either way; it is
// its bed's colour times `STONE_TONE`, lit as a dome; it is drawn as itself from `STONE_PX` pixels across, below as
// the tone its stones give the ground.
const STONE_GAP = 2.5; const STONE_SIZES = 0.4; const STONE_TONE = 1.15; const STONE_PX = 2.0;
""", """// Stones (stone.wgsl): on a slope from `SCREE_FROM` to `CLIFF_FROM` where the loose ground is thinner than
// `SCREE_LOOSE_M` (the scree below a face), angular blocks up to `SCREE_STONE_M` across in at most `SCREE` of the
// places they could lie; on bare bed, `STONES_ON_BED` of them; none on a beach. A stone's foot is sunk by up to
// `SUNK_MOST` of its height where the loose ground about it is `SUNK_LOOSE_M` deep or more. In a channel's bed (where
// the spot holds its stones, `Spot.stony`), the largest are `STONE_SEEN` times the largest it moves (a map draws its
// gravel a size up), no smaller than `COBBLE_M`, sunk `BED_SUNK` in the bed's gravel.
const SCREE_FROM = 0.6; const SCREE_LOOSE_M = 1.0; const SCREE_STONE_M = 1.2; const SCREE = 0.7;
const STONES_ON_BED = 0.25; const SUNK_MOST = 0.55; const SUNK_LOOSE_M = 0.5;
const STONE_SEEN = 1.5; const COBBLE_M = 0.22; const BED_SUNK = 0.3;
"""),
])

edit('web/shaders/lib/colour.wgsl', [
 ("""  // The reflectance of its bed; the share of the bed that shows (rock.wgsl `bed_shows`); the depth of its loose
  // ground, m; and the largest stone its channel moves there, m (0: none).
  bed: vec3f, rock: f32, loose: f32, stone: f32,""", """  // The reflectance of its bed; the share of the bed that shows (rock.wgsl `bed_shows`); the depth of its loose
  // ground, m; and of its channel there: the largest stone it moves, m, the share of the places that hold one of its
  // stones (0: none), and how round it has worn them, 0..1.
  bed: vec3f, rock: f32, loose: f32, stone: f32, stony: f32, worn: f32,"""),
 ("""// The colour of the ground of spot p, unlit: its own colour, tinted by the hand rules of cold, height and sand, and its bed
// where it shows; its stones; snow `snow` over them; and over all the plants that stand out of it""", """// The colour of the ground of spot p, unlit: its own colour, tinted by the hand rules of cold, height and sand, and its bed
// where it shows; snow `snow` over it; its stones, as far as they stand out of the snow; and over all the plants that
// stand out of it"""),
 ("""  // Stones over it: scree, on bare bed, and in a channel; none on a beach.
  if (styled()) {
    let thin = 1.0 - ease(0.0, SCREE_LOOSE_M, p.loose);
    let scree = max(SCREE * ease(SCREE_FROM, CLIFF_FROM, steep) * thin, STONES_ON_BED * p.rock) * (1.0 - p.sand);
    let channel = select(0.0, IN_CHANNEL, p.stone > 0.0) * (1.0 - p.sand);
    // The scree's stones, then the channel's, from one place in the code.
    for (var k = 0; k < 2; k++) {
      let on = k == 1;
      let st = stones(p.at, p.e, select(scree * (1.0 - plants), channel, on), select(SCREE_STONE_M, max(p.stone, PEBBLE_M), on), p.bed, 61 + k);
      g.rgb = mix(g.rgb, st.rgb / max(st.a, 1e-4), st.a);
    }
  }
  g.rgb = mix(g.rgb, SNOW, snow);
  // Snow is not bare soil: it takes neither the soil's warmth nor its darkening when wet (`soil`).
  g.bare *= 1.0 - snow;
  if (snow > 0.5) { g.kind = ICE; }""", """  g.rgb = mix(g.rgb, SNOW, snow);
  // Snow is not bare soil: it takes neither the soil's warmth nor its darkening when wet (`soil`).
  g.bare *= 1.0 - snow;
  if (snow > 0.5) { g.kind = ICE; }
  // Stones over it (stone.wgsl): scree, on bare bed, and in a channel; none on a beach.
  if (styled()) {
    let thin = 1.0 - ease(0.0, SCREE_LOOSE_M, p.loose);
    let scree = max(SCREE * ease(SCREE_FROM, CLIFF_FROM, steep) * thin, STONES_ON_BED * p.rock) * (1.0 - p.sand);
    let sunk = SUNK_MOST * ease(0.0, SUNK_LOOSE_M, p.loose);
    // The scree's angular blocks, then the channel's worn stones, from one place in the code.
    for (var k = 0; k < 2; k++) {
      var s = Stony(scree * (1.0 - plants), SCREE_STONE_M, 0.0, sunk, snow);
      if (k == 1) { s = Stony(p.stony * (1.0 - p.sand), max(STONE_SEEN * p.stone, COBBLE_M), p.worn, BED_SUNK, snow); }
      let st = stones(p.at, p.e, s, p.bed, 61 + 8 * k);
      g.rgb = mix(g.rgb, st.rgb / max(st.a, 1e-4), st.a);
    }
  }"""),
])
edit('web/shaders/lib/earth.wgsl', [
 ("""  s.stone = 0.0;""", """  s.stone = 0.0;
  s.stony = 0.0;
  s.worn = 0.0;"""),
])
for prog in ['class', 'earth', 'land']:
    p = 'web/shaders/%s.wgsl' % prog
    t = open(p).read()
    assert t.count('crag.wgsl ice.wgsl') == 1
    open(p, 'w').write(t.replace('crag.wgsl ice.wgsl', 'crag.wgsl stone.wgsl ice.wgsl', 1))

edit('web/shaders/lib/stone.wgsl', [
 ("""const STONE_GAP = 2.5; const STONE_SIZES = 0.4; const STONE_PX = 2.0;""", """const STONE_GAP = 3.0; const STONE_SIZES = 0.4; const STONE_PX = 2.0;"""),
])
edit('web/shaders/land.wgsl', [
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
  // The bed's colour where it shows or its stones lie.
  if (p.rock > 0.0 || p.stony > 0.0) { p.bed = bed_make_at(at); }"""),
 ("""  out.bar = gravel * bar_tone(at, e, g.along, r);
  out.bar_over = BAR_RISE * r.edge;""", """  out.bar = gravel * bar_tone(at, e, g.along, r);
  out.bar_over = BAR_RISE * r.edge;
  out.dry.stone *= 1.0 - BAR_FINES * r.up;"""),
])
