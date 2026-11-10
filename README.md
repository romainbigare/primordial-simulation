# Emergent World

A world that runs on five physical laws; everything else must emerge.

- `docs/handover.md`: the plan, the decisions and the rules of work
- `docs/laws.md`: how each law works now, with the code that does it
- `docs/numbers.md`: every free number, its unit and its source
- `docs/tape.md`: what a run records

```
cargo run --release -p cli --bin ew -- run --years 300 --frames-every 5 --out out/run       # simulate: the tape (series.csv, viewer frames) and a snapshot at the end
cargo run --release -p cli --bin ew -- run --years 300 --snapshots-every 100 --pictures-every 50 --months --out out/run   # also snapshots, PNG pictures and months.csv
cargo run --release -p cli --bin ew -- resume --from out/run/snap_00300.bin.gz --years 100 --out out/run   # go on from a snapshot
cargo run --release -p cli --bin ew -- view --from out/run/snap_00300.bin.gz --window 40,10,8,8 --px 800 --layer map --edges --out look.png   # a window of a snapshot's map as a PNG (fractions of a square allowed; cells finer than a pixel are seen)
cargo run --release -p cli --bin ew -- chem                                                   # chemistry 0 as a table
cargo run --release -p cli --bin ew -- numbers out/a out/b                                    # the numbers two runs differ in
cargo run --release -p cli --bin ew                                                           # every command and flag
cargo run --release -p cli --no-default-features --bin ew -- run ...                         # flow and change without threads (same hash)
cd web && npm run build && npm run wasm && python3 -m http.server --directory dist           # live viewer: open http://localhost:8000 (the core runs in a worker; seed and size on the page)
cd web && npm run build && cp ../out/run/world.ewf.gz dist/                                   # tape viewer: open http://localhost:8000/?f=world.ewf.gz, or drop a file on the page
```

Speed: `ew run --size 128 --years 1000` on 4 cores: 2.1 s per year, measured over 20 years (`--years 20 --every 1000 --frames-every 1000`; 2.2 s per year over the last 2 of them, 16,384 cells, no bodies yet), on 2026-11-01 with a load average of 1.9 (`uptime`). Every run prints its own speed line at the end.

A world's cells stay those as built for its whole life, whatever the zoom. Zoom is a matter of
drawing only: the map draws every zoom from the cells (below). The page shows the date, the months per second the world
reaches and the cells. While the world plays, the worker posts its state to the page at most four times a second
(`web/src/worker.ts` `FRAME_MS`): each frame costs the page a repaint of the map.

The map is drawn on the graphics card at the screen's own pixels: with WebGPU where the browser has it, else with
WebGL2. The shaders are written once in WGSL (`web/shaders/`); the build (`npm run build`) checks them and writes them
as GLSL ES 3.00 for WebGL2 too with naga, the shader translator of the wgpu project (`web/shade`, so the build needs
cargo), and puts both in the page. The page hands the card fields, not pictures (a tape's `web/src/fields.ts`, the live
map's tiles and table): of each square or texel the ground's height and slope, the liquid's surface over the ground, the ice, the ground's temperature, the
share that is sea and that is a river and the river's depth, how wet the ground is, the plant cover's byte, the true
colour and every layer. The card turns them into colours at each pixel of the screen (`web/shaders/lib/colour.wgsl`:
the Earth view, the true colour and the layer views, one rule for a tape's map of squares and for the live map),
with the light on the slopes. Water is water (`web/shaders/lib/water.wgsl`): a flat surface, its bed seen through it
and fading with depth as water absorbs red light most and blue least (Beer–Lambert), into the dark blue of deep water;
a little of the sky reflects from it and the sun glints on small ripples fixed in the world, which fade out as they
shrink below a few pixels. A sea, a lake and a river are the same water, a river as deep as its discharge makes it.
The ground goes from dry to wet continuously: soaked bare ground is darker (to half as bright), wet by how wet the core
says it is and by the capillary fringe over the liquid's surface (wet to e^-1 half a metre over it), so a flat shore
is a wide wet band and a steep one a thin line; near soaked, a film of liquid shines on it as mud, and at the
waterline it is the bed under no water, so the shore has no drawn line. It also names each square's class (sea, lake, forest...)
for the inspect panel. Avatars, the lines of latitude, the outlines and the scale bar are drawn on a 2D canvas over it.
While a frame takes the card longer than 16 ms, the map is drawn at fewer pixels (down to a quarter along each side)
and stretched onto the screen, the same picture, softer; once the view rests it is drawn again at the screen's own
pixels, a tile a frame, and shown when whole. The page never waits for the card: what it reads back (the classes, the
colours under the avatars) comes a frame or so later.

The live viewer draws every zoom from two things (`web/src/land.ts`, `web/shaders/land.wgsl`). **The ground's tiles**
(`tape::Ground`) never change: each holds 256 x 256 texels of a depth, from the depth at which one tile holds the
island down to texels of 1 m (closer, the finest is magnified), with the ground's height (the terrain itself, smooth
below a metre), the wetness index of the place and the channel nearest it, as 16-bit floats (10 bytes a texel, the
height to a millimetre). Workers of their own, each with its own copy of the core, make them (`web/src/tiles-worker.ts`;
two where the browser has more than four cores, else one), so the world never waits for a picture; the page keeps at
most 56 on the graphics card (38 MB) and no copy of its own (after a lost context it asks for them again), and asks for
those about the view, at the depth whose texels take no more than two css pixels, at the depth above and at the
coarsest, coarse before fine and nearest its middle first (`web/src/ground.ts`); a request that left the view is
dropped, and the tiles drawn least lately go first. A tile not made yet is drawn from its nearest ancestor, stretched: the picture is soft for
a moment, never the squares; a new tile fades in over it in a quarter of a second. **The cells' table**
(`tape::Table`) comes with each world frame: a row a square of what the laws inside a cell read. At each pixel the card
blends the table between the squares' centres and applies the laws inside a cell there: the ground's temperature is
its cell's less the air's fall with height times the pixel's height over the cell's mean; the frozen cover covers the
ground as its mean depth does (tanh of the depth over 8 mm of ice, about 3 cm of snow, Yang and others 1997), less
where the pixel is warmer than its melting point; liquid stands where the ground lies under the level of the cells about it, counting a cell only where its
lowest ground lies under the surface of the pixel's own cell (so the sea is flat and a cell's liquid is not drawn over
the lower ground beside it); the soil's liquid departs from its cell's by `soil_m` per unit of the wetness index over
the cell's level, within none and the pores full, as the core shares it; a channel shows where the pixel lies nearer
its centre line than half the width the channel law gives its discharge (its drained area times the cells' flux per
m² drained), its share of the pixel what the pixel's footprint covers, its bed a parabola cut into the ground (its
banks catch the light) of washed sand and gravel, the water deepest at its middle. A slope turned away from the sun is
lit by the sky alone, the less the steeper (an even sky, Liu and Jordan 1963), so shaded ground and snow keep their
shape. Nothing depends on the view or a tile, so a
place has the same colour after any pan or zoom. The stats line gives the tiles kept, their size and how many in view
are still to come, and which of WebGPU and WebGL2 draws. A tape holds no ground: its map is drawn from its squares.

On the live map the Earth view draws the plant cover plant by plant from its numbers, with no classes
(`web/src/vegetation.ts`, `web/src/canopy.ts`, `web/shaders/cover.wgsl`). With each frame the core sends, for each
square, up to 4 kinds of cover (the slots of one starting kind joined) and of each the plants its mass makes
(`tape/src/stands.rs`): as many a m² as the self-thinning law gives (Yoda and others 1963), as tall as the buckling
limit lets their stems stand (McMahon 1973), each crown as wide as the ground it holds, in patches that cover as much
of the ground as its leaves close; and its genes and state this month. A plant is drawn on one of three lattices fixed
in the world (cells of 0.1, 0.5 and 2.5 m), where its marks take a few pixels; where they take less, the same plants
are a field of their colours with the grain their count gives, so the far and the near pictures agree. A mark picks
its kind at its own foot from the four squares about it (blended as the map is) by the kinds' chances there, fewer
where the ground is drier than its cell's mean as each kind's activity law says, by its own random rank: kinds mix
mark by mark across a square's edge, and a kind that needs wet ground gathers in the hollows. None stands where liquid
stands over its roots (0.3 m, Jackson and others 1996) or under snow. From above a crown is a dome or a cone by its
height against its width, its edge in lobes as broad as its leaves, open where its leaf area is low, over its stem and
its shadow; angled, it stands on its stem. Its colour comes from its numbers: darker with more light capacity per
capture area and thicker leaves, greyer for dry ground, bluer for hardiness to frost; paler short of fuel, faded when
dormant, brown where frozen. Chemistry 0 gives every part of the cover one reflectance, so the base hue is the
viewer's. Under the plants the ground has its own colour (its loose solids, else its bonded ones: the `ground_r`,
`ground_g`, `ground_b` layers), darker where wet, under liquid and snow; the green comes only from the plants. A tape
holds only totals per square and no kinds: its map draws the plants as one foliage as closed as their capture area
closes the ground, without marks.

From 64 pixels a square, the live viewer draws the bodies and parts of each cell one by one at their real size, from
the numbers the core hands out (`tape::things`): a member is as long as 1.5 times the cube root of its volume (a body
as dense as water), each member at its own seeded place across its cell's real area, so it holds while the camera
moves. A member of 3 pixels or more is drawn as its shape (from 8 pixels with a rim that stands out from the ground),
smaller as a soft dot. One or two doublings of the zoom before a member takes a pixel, its node shows as a light, soft
blob over its cell, a little stronger the more members a m² holds, lighter than dark ground and darker than light
ground; it fades in as the zoom comes near. Farther out no herd is drawn. Parts are drawn at their own sizes, hidden below a pixel. Two
styles draw the shapes (the "Life" buttons, kept per browser): from above (a body with its head, wider the stronger) or
angled (a block on legs, as a small axonometric sprite). Hover or click one to see its numbers; the inspect panel
gives a herd's count. The map zooms to 80 pixels a metre, so one animal shows its shape and can be picked.

By default the animals start with hand-set brain biases (`--neutral-eaters` for neutral brains) and the
light-eaters with no brain.
