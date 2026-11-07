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

In the live viewer, zooming in on the map refines the world there: the page sends the window in view and the depth the
zoom calls for to the core as a shot of the camera's path, and the map shows the fine cells over the whole-map frame (a
budget of 1500 cells, none finer than depth 10: 7.8 m, a street). The world steps slower by itself while fine cells are
in view; the page shows the date, the months per second it reaches, the cells and the finest depth. Refinement runs
while the world plays. After 3 minutes without input the page sends the camera away (a recorded change of the camera),
so the world joins its fine cells and runs at the coarse pace again; the next zoom, pan or click brings it back.

At every zoom the live viewer draws the window in view 2 to 4 depths finer than its cells (`tape::fine`, a pixel of
1 to 2 screen pixels, at most 360,000 of them): the ground's height from the terrain below the cells, set on each
cell's own height, with hill shading; the liquid where its surface lies over that ground, so shores follow the ground;
snow by the ground's own temperature; the other layers blended between the cells' centres; rivers as lines along the
way their liquid runs. It is made again when the view rests and once a second while the world plays; the stats line
gives its size and the time the core took to make it and the page to paint it.

The Earth view draws the plant cover as textures at its real size (`web/src/vegetation.ts`, `web/src/textures.ts`),
over the fine raster where there is one and blended between the cells' centres as the map is: crowns where it is woody
(shrubs, woodland, forest, a jungle where it is also warm and wet), a tree's crown 3 to 15 m across by its walls per
m², and tufts where it is herbs (grass, steppe, tundra); as many marks as its capture area closes it (1 - e^-area). Each
kind is one tile made once and repeated, coloured by the cover and, close up, by the genes of the cell's heaviest slot
(hue by its light and leaf genes, darker if frost-hardy). A texture shows once its marks take about a pixel.

From 8 pixels a square, the live viewer draws the bodies and parts of each cell one by one at their real size, from
the numbers the core hands out (`tape::things`): a member is as long as 1.5 times the cube root of its volume (a body
as dense as water), each member at its own seeded place across its cell's real area, so it holds while the camera
moves. A member of 3 pixels or more is drawn as its shape (from 8 pixels with a rim that stands out from the ground),
smaller as a soft dot, and below a pixel its node is a stipple over its cell, so a herd shows from afar: denser and
stronger the more members a m² holds (alike at every zoom, from a faint patch to a strong one), lighter than dark ground
and darker than light ground, thinning at its edges. Parts are drawn at their own sizes, hidden below a pixel. Two
styles draw the shapes (the "Life" buttons, kept per browser): from above (a body with its head, wider the stronger) or
angled (a block on legs, as a small axonometric sprite). Hover or click one to see its numbers; the inspect panel gives a herd's count.
The map zooms to 16 pixels a metre.

By default the animals start with hand-set brain biases (`--neutral-eaters` for neutral brains) and the
light-eaters with no brain.
