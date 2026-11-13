p = 'docs/numbers.md'
lines = open(p).read().split('\n')
k = [i for i, l in enumerate(lines) if l.startswith('| `BED_VARY` (0.22')]
assert len(k) == 1
lines[k[0]] = "| `BED_VARY` (0.22 over `BED_VARY_M` 32 and 8 m), `INNER_MOST` (0.8), `BEND_FULL` (0.4), `POOL_NARROW` (0.2), `RIFFLE_WIDE` (0.3), `POOL_AT_BANK` (0.6), `POOL_SKEW` (0.8), `web/shaders/lib/rock.wgsl`; `RAG_M` (9, 2.6 and 0.8 m), `RAG_SHARE` (0.3), `RAG_MOST_M` (3 m), `STEEP_STONE` (0.12 to 0.5 m), `STREAK_W_M` (1.2 m), `STREAK_M` (8 m), `STREAK` (0.6), `WHITE_M` (4 m), `FOAM_STEEP` (0.55), `web/shaders/lib/river.wgsl`; `REFLECT_STREAK` (0.25), `STONES_OUT` (1.2), `web/shaders/land.wgsl` | m; share | a bed's width varies by up to 22% along it and reaches no farther inside a bend than 0.8 of the radius its line turns on; in a full bend (curvature times bed width 0.4) the water is a pool 20% narrower, against the outer bank from 0.6 of a full bend on, its deepest leaning to that bank, over a bar inside; between bends a riffle 30% wider; its edges ragged; a steep reach moves stones of 0.12 to 0.5 m; streaks along the flow and white water on steep reaches (drawing only) | pools at the bends and riffles between them, five to seven widths apart (Leopold and Wolman 1957, 1960; Keller and Melhorn 1978) | tuned, by eye |"
lines.insert(k[0] + 1, "| `BAR_MOTTLE` (0.2 over `BAR_PATCH_M` 2.5 and 0.6 m), `STRAND_M` (0.9 m), `STRAND_W` (0.2), `STRAND` (0.28), `STRAND_WAVE_M` (8 m), `BAR_GROWN` (0.55), `BAR_TUFT_M` (1.1 m), `BANK_M` (0.4 m), `BANK_SHARE` (0.12), `BANK_SHADE` (0.45), `BANK_LOW` (0.25), `web/shaders/lib/river.wgsl`; `BAR_LIFT` (1.7), `BAR_PALE` (0.35), `BAR_SAND_STONE` (0.03 m), `BAR_RISE` (0.6), `EDGE_M` (0.35 m), `EDGE_LIGHT` (0.25), `BED_SHOWN` (0.8), `web/shaders/land.wgsl`; `RIVER_SEEN_M` (14 m) and `RIVER_SEEN_POW` (1.6), `web/shaders/lib/style.wgsl` | m; share | a bar is its bed's make washed and paled toward sand, sand where no stone over 3 cm moves, mottled, wet by the water's fringe as it rises from its edge, with drift lines along the flow every 0.9 m up it and tufts of plants on its top; a bank is a scarp 0.4 m and 12% of the bed's half width wide, lit or shaded by up to 45% as it is turned to the sun, a quarter of that on the low slope inside a bend; the water is coloured as its own depth as a lake's and 14 m more where it is as deep as the channel law makes its middle (that share to the power 1.6), lighter by a quarter within 0.35 m of its edge, over its bed's stones at 80% of their light (drawing only, the Earth view only) | none: a river from above shows light shallows over its bed, a dark thread at the outer bank of each bend and pale bars inside | tuned, by eye |")
open(p, 'w').write('\n'.join(lines))

p = 'README.md'
t = open(p).read()
a = """1953): a pool against the outer bank of a bend over a bar of washed gravel on its inside, a wider riffle between
bends; what runs through it grows evenly from sample to sample, so it widens downstream and below each tributary
without a step."""
b = """1953): a pool against the outer bank of a bend over a bar of washed gravel on its inside, a wider riffle between
bends; what runs through it grows evenly from sample to sample, so it widens downstream and below each tributary
without a step. In the Earth view the water is coloured by its bed's shape, light shallows over the bed's stones and a
dark thread where it is deepest; a bar is pale gravel, wet at the water's edge, with the drift lines the falling
water leaves and tufts of plants on its top; a bank is a low scarp the sun lights or shades."""
assert t.count(a) == 1
open(p, 'w').write(t.replace(a, b))
