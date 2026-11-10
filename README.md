# Emergent World

A world that runs on five physical laws; everything else must emerge.

- `docs/handover.md`: the plan, the decisions and the rules of work
- `docs/laws.md`: how each law works now, with the code that does it
- `docs/numbers.md`: every free number, its unit and its source
- `docs/tape.md`: what a run records

```
cargo run --release -p cli --bin ew -- run --years 300 --frames-every 5 --out out/run       # simulate: the tape (series.csv, viewer frames) and a snapshot at the end
cargo run --release -p cli --bin ew -- run --years 300 --snapshots-every 100 --pictures-every 50 --months --out out/run   # also snapshots, PNG pictures and months.csv
cargo run --release -p cli --bin ew -- run --size 64 --roster --years 50 --cells 4060 --finest 4 --camera cam.txt --out out/zoom   # refine under a budget of cells, following a camera path
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

Refinement (`docs/laws.md` 0): with `--cells N` the cells of most interest split and the quietest join at each
month's start, the map holding at most N cells and none finer than `--finest` (4: 500 m on an island). `--camera FILE`
gives the camera's path, one line a change: `MONTH X Y W H DEPTH` (from month MONTH on, the window from column X and
row Y, W x H squares, asks for cells of DEPTH; a square is depth 2), or `MONTH` alone (the camera goes away). The path
is an input of the run: the snapshot and the tape keep it, so the same seed and path give the same world.

The live viewer's world has no budget of cells: its cells stay those as built, whatever the zoom. Zoom is a matter of
drawing only: the map draws every zoom from the cells (below). The page shows the date, the months per second the world
reaches and the cells. While the world plays, the worker posts its state to the page at most four times a second
(`web/src/worker.ts` `FRAME_MS`): each frame costs the page a repaint of the map.

The map is drawn on the graphics card at the screen's own pixels: with WebGPU where the browser has it, else with
WebGL2. The shaders are written once in WGSL (`web/shaders/`); the build (`npm run build`) checks them and writes them
as GLSL ES 3.00 for WebGL2 too with naga, the shader translator of the wgpu project (`web/shade`, so the build needs
cargo), and puts both in the page. The page hands the card fields, not pictures (`web/src/fields.ts`): of each square
or pixel the ground's height and slope, the liquid's surface over the ground, the ice, the ground's temperature, the
share that is sea and that is a river and the river's depth, how wet the ground is, the plant cover's byte, the true
colour and every layer. The card turns them into colours at each pixel of the screen (`web/shaders/lib/earth.wgsl`:
the Earth view, the true colour and the layer views, one rule for the map of the squares and for the finer window),
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

At every zoom the live viewer draws the window in view finer than its cells (`tape::fine`, a pixel of 1 to 2 screen
pixels and no smaller than 5 cm, smoothed up closer, at most 360,000 of them), and a fifth of the view about it, so a pan
or a zoom out shows it until the next comes: the ground's height, the terrain itself (the samples the cells' heights
and their liquid stand on, smooth below a metre), with hill shading; the liquid each cell holds standing at the level
that fills the terrain drawn inside the cell with its volume from the lowest point up, as the core's level does, drawn where
the ground lies below that level, so a shore follows the ground inside the cells and moves with the level; how wet the ground is and the plant cover of each slot inside each cell (`tape/src/fine/inside.rs`); snow by the ground's own temperature; each value of the cells the view reads (`web/src/fine.ts` `FINE_LAYERS`, the layer shown, the worn paths) as one smooth field, built level by level of the
cells' tree, so a cell that holds what its parent held shows no edge and a refined region looks like the cells about it
(`tape/src/fine/pyramid.rs`); rivers as lines along the way their liquid runs. The worker makes its fields; it is made
again when the view rests and, while the world plays, no sooner than a second and than nine times the time the last
one took, so it takes at most about a tenth of the worker's time, and each new window fades in over the last in a
quarter of a second, over the map of the squares, its outer edge faded too, so the picture never jumps; the stats line
gives its size, the time the core took to make it and the worker its fields, and which of WebGPU and WebGL2 draws.

The Earth view draws the plant cover as textures at its real size (`web/src/vegetation.ts`, `web/src/textures.ts`),
over the fine raster where there is one and blended between the cells' centres as the map is; in the fine raster each
slot of a cell is a stand of its own kinds where its own cover lies (so a kind that likes wet ground stands in the wet
ground and the shallows, and stops where its cover stops), the stands closing the ground together by their summed
capture area, each by its share of it; a mark under liquid deeper than it is wide is not drawn: crowns where it is woody
(shrubs, woodland, forest, a jungle where it is also warm and wet), a tree's crown 3 to 15 m across by its walls per
m², and tufts where it is herbs (grass, steppe, tundra); as many marks as its capture area closes it (1 - e^-area). A
kind gives way to the next over a range of each threshold, so kinds mix mark by mark, never at an edge. The marks are
drawn per pixel on the graphics card at every frame, a pass of the map's renderer: each kind's marks lie on a grid
fixed in the world, one mark a grid cell at a place, size and tone hashed from the cell, so they hold while the camera
moves and never repeat or stand in rows. They
are coloured by the cover and, close up, by the genes of the cell's heaviest slot (hue by its light and leaf genes,
darker if frost-hardy). A kind's marks fade in as they grow from 0.7 to 2 pixels. A pan, a zoom or a new frame shows
the marks at once, never a plain colour in their place.

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
