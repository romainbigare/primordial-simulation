def edit(p, pairs):
    t = open(p).read()
    for a, b in pairs:
        assert t.count(a) == 1, (p, a[:70], t.count(a))
        t = t.replace(a, b)
    open(p, 'w').write(t)

edit('docs/numbers.md', [
 ("`STONE_OCTAVES` (3), `STONE_STEP` (2.2), `STONE_FEW` (0.35), `STONE_GAP` (3)", "`STONE_OCTAVES` (3), `STONE_STEP` (2.2), `STONE_FEW` (0.5), `STONE_GAP` (3)"),
 ("`SNOW_BURIES_M` (0.7 m), `STONE_MOST_M` (2 m), `web/shaders/lib/stone.wgsl`;", "`SNOW_BURIES_M` (0.7 m), `STONE_MOST_M` (2 m), `WAKE_LONG` (2.2), `WAKE_SPREAD` (0.35), `WAKE_EDDY` (0.8), `web/shaders/lib/stone.wgsl`;"),
 ("`STONE_SEEN` (2.5), `COBBLE_M` (0.45 m), `BED_SUNK` (0.3), `web/shaders/lib/crag.wgsl`; `IN_CHANNEL` (0.6), `STONES_OUT` (1.8), `WORN_Q` (0.005 to 0.3 m³/s), `BAR_FINES` (0.5), `web/shaders/land.wgsl`",
  "`STONE_SEEN` (2.5), `COBBLE_M` (0.65 m), `BED_SUNK` (0.3), `web/shaders/lib/crag.wgsl`; `IN_CHANNEL` (0.6), `STONES_OUT` (1.8), `WORN_Q` (0.005 to 0.3 m³/s), `BAR_FINES` (0.5), `STONE_SMALL` (0.3 to 1.25), `STONE_STANDS` (0.3), `WAKE` (0.7), `web/shaders/land.wgsl`"),
 ("three sizes each 2.2 times smaller, the largest in 35% as many cells,", "three sizes each 2.2 times smaller, the largest in half as many cells,"),
 ("a channel's stones drawn 2.5 times the largest it moves, no smaller than 0.45 m, in 60% of their places on its bed, out to 1.8 of its bed's half widths beside a steep reach, round from 0.3 m³/s on, half as large at a bar's top (drawing only, the Earth view only)",
  "a channel's stones drawn 2.5 times the largest it moves, no smaller than 0.65 m, in 60% of their places on its bed, out to 1.8 of its bed's half widths beside a steep reach, round from 0.3 m³/s on, half as large at a bar's top; where they lie follows their sample's size, in a step from sample to sample, and each is 0.3 to 1.25 of it as the size the channel moves grows along it; a stone stands out of water shallower than 0.3 of its size, and fast water breaks white on it and trails 2.2 of its size down the flow, spreading, with calm water right behind it (drawing only, the Earth view only)"),
 ("| `STONE_M` (0.001, `tape/src/ground/rock.rs`) | m per m | the largest stone a channel moves per unit of √(area drained, m²) × slope (drawing only)",
  "| `STONE_M` (0.001, `tape/src/ground/rock.rs`) | m per m | the largest stone a channel moves per unit of √(area drained, m²) × the slope of its reach (`REACH_SAMPLES` up and down it; drawing only)"),
])
edit('docs/tape.md', [
 ("""√(the area it drains, m²) × its slope to the sample it runs to (the stream power the wear reads, K √A S; Bagnold 1966),""",
  """√(the area it drains, m²) × the slope of its reach, 2 samples up and down it (the stream power the wear reads, K √A S; Bagnold 1966),"""),
])
edit('README.md', [
 ("""of the smaller, its foot sunk in loose ground, under the snow unless it is large, then with snow on its top.""",
  """of the smaller, its foot sunk in loose ground, under the snow unless it is large, then with snow on its top. In a
channel a stone taller than the water is deep stands out of it, and fast water breaks white on it and trails behind
it down the flow."""),
])
