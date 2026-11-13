p = 'README.md'
t = open(p).read()
a = """dark ledges along the contours, cracks and a strong light and shade; scree lies on steep thin ground, stones on bare
bed and in a channel as big as its power moves (`web/shaders/lib/crag.wgsl`)."""
b = """dark ledges along the contours, cracks and a strong light and shade; scree lies on steep thin ground, stones on bare
bed and in a channel as big as its power moves (`web/shaders/lib/crag.wgsl`). One maker draws every stone
(`web/shaders/lib/stone.wgsl`): a block cut by five planes, so no stone is a disc, longer one way than the other, each
face lit as it is turned to the sun, with a shadow on the ground, in its bed's colour with the bands of its layers;
angular on scree and bare bed, the rounder the more a channel carries (the travel wears it), in three sizes with more
of the smaller, its foot sunk in loose ground, under the snow unless it is large, then with snow on its top."""
assert t.count(a) == 1
open(p, 'w').write(t.replace(a, b))

p = 'docs/numbers.md'
lines = open(p).read().split('\n')
k = [i for i, l in enumerate(lines) if l.startswith('| `STONE_M` (0.001, `tape/src/ground/rock.rs`)')]
assert len(k) == 1
lines.insert(k[0] + 1, "| `STONE_CUTS` (5), `STONE_CUT` (0.38), `STONE_LONG` (0.7), `STONE_TOP` (0.75), `STONE_TILT` (0.3), `STONE_SOFT` (0.04 to 0.45), `STONE_FACE` (1.7 to 1.2), `STONE_RISE` (1.1), `STONE_SHADOW` (0.3 out to `STONE_SHADOW_REACH` 0.4), `STONE_TONE` (1.15), `STONE_TONES` (0.18), `STONE_BANDS` (0.12 every `STONE_BAND_M` 0.35 m), `STONE_OCTAVES` (3), `STONE_STEP` (2.2), `STONE_FEW` (0.35), `STONE_GAP` (3), `STONE_SIZES` (0.4), `STONE_PX` (2), `SNOW_TOP` (0.8 to 0.95), `SNOW_BURIES_M` (0.7 m), `STONE_MOST_M` (2 m), `web/shaders/lib/stone.wgsl`; `SCREE_STONE_M` (1.2 m), `SCREE` (0.7), `STONES_ON_BED` (0.25), `SUNK_MOST` (0.55 from `SUNK_LOOSE_M` 0.5 m), `STONE_SEEN` (2.5), `COBBLE_M` (0.45 m), `BED_SUNK` (0.3), `web/shaders/lib/crag.wgsl`; `IN_CHANNEL` (0.6), `STONES_OUT` (1.8), `WORN_Q` (0.005 to 0.3 m³/s), `BAR_FINES` (0.5), `web/shaders/land.wgsl` | none; share; m; m³/s | the one maker of a stone: five planes cut its outline, each up to 38% in from its radius, the stone up to 1.7 times as long as wide, its top 0.75 of its radius high and tilted; its edges sharp to round and its faces steep to gentle as it is worn; its shadow, its tone and its rock's bands; three sizes each 2.2 times smaller, the largest in 35% as many cells, a cell 3 stones wide, drawn as itself from 2 pixels; snow lies on its flat top and buries a stone under 0.7 m; scree's blocks up to 1.2 m in 70% of their places, 25% on bare bed, sunk up to 55% in loose ground 0.5 m deep; a channel's stones drawn 2.5 times the largest it moves, no smaller than 0.45 m, in 60% of their places on its bed, out to 1.8 of its bed's half widths beside a steep reach, round from 0.3 m³/s on, half as large at a bar's top (drawing only, the Earth view only) | debris is angular at its source and rounds within a few km of travel in a channel (Sternberg 1875; Krumbein 1941, from memory); it has many small pieces and few large (a power law of sizes) | tuned, by eye |")
open(p, 'w').write('\n'.join(lines))
