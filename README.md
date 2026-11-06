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
budget of 1500 cells, none finer than depth 4). The world steps slower by itself while fine cells are in view; the page
shows the date, the months per second it reaches, the cells and the finest depth. Refinement runs while the world plays.

Close up (a square of the map at 64 pixels or more), the live viewer draws what lies in each cell one by one, from
the numbers the core hands out (`tape::things`): animals sized by a member's mass (a log scale), bulkier and longer-legged
the stronger, furred by their coat gene, with fangs if they bite bodies, young ones smaller; a herd (a node) as some of
its members with its count; the cover as grass, shrubs or trees by its walls gene, as many as its density; parts as
blocks of their sizes and walls' colour, hollow ones open, hard ones stone. Hover or click one to see its numbers.
Farther away the map shows density and colour only.

By default the animals start with hand-set brain biases (`--neutral-eaters` for neutral brains) and the
light-eaters with no brain.
