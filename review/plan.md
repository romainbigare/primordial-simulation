# The plan toward a world that zooms

For the owner. Simple English. This plan comes from a review of `main` against the spec (`docs/handover.md`), an
adversarial review of that review, and a second review that proposed one tree instead of three levels. It replaces the
order in `review/top-30.md`. Where it and the spec differ, the spec is changed in the same work package and says so.

## 1. The diagnosis, in short

- The climate, the land and simple life work, and the books close.
- The detail went into the physiology of one body, not into space and time.
- The world has one grain of space (2 km squares, cells of 1, 2 or 4 squares fixed at the start) and the bodies one
  pace (a month). Nothing refines or collapses while a run goes on.
- A held part changes nothing a body does (`laws.md` 3.3), so tools cannot emerge at any grain. This is missing
  physics (spec 9.4), and it comes before any change of grain.

## 2. The shape: one tree, one body, one budget

This is the spec's own design (5.2, 7.1, 7.2: one graph whose cells split and join; the size follows a time budget).

1. **One kind of cell at every size.** Any cell splits into 4 children or joins back into one. The same five laws run
   on every cell; the cell's size is an input of each law (adaptive mesh refinement, Berger and Oliger 1984).
2. **One kind of life: a group of `n` members** (super-individuals, Scheffer et al. 1995; `Body::n()` today). Far
   away one group holds a million members in a big cell; close up it splits until `n` is 1. What only makes sense for
   one body (two parents, holding a part) works where `n` is small, by the law's own threshold, not by a layer.
3. **The step follows the size.** A law's step lets nothing cross more than one cell. Big cells step once a month,
   small ones more often. At a border the fine side sums what it sends and the coarse side books that sum once (a flux
   register, Berger and Colella 1989): this is what a chunk and its queue already do (`life::Chunk`, `life::cross`).
4. **Detail follows a budget.** A fixed count of active cells (a number of the run, never the machine's clock). Each
   cell has an interest score: the camera, how fast things change there, brains, parts and bonds, rare events. Each
   month the cells of the highest score split and those of the lowest join, until the budget is full.
5. **Zoom slows time by itself.** The one world clock moves at the pace of the finest cell. Looking at a street, the
   world runs slowly; the rest of the planet stays coarse and costs little.

Three conditions keep this one law at every grain:

- **Rates per second.** A body's learning, its discount and its brain's cost are per second, not per decision, so a
  body that decides every hour in a fine cell and one that decides every month in a coarse cell learn and pay alike
  over a year. This changes spec 6.5 (a brain paid per decision).
- **The same seed and the same camera path give the same world.** The camera is an input of the run, recorded in the
  tape, as a god action is (3.8). A run with no camera (the world search, M7) refines by the other terms of the score.
- **A floor on the step for long runs.** A run of many years sets the finest size its year budget allows, so a
  headless run of 1,000 years never falls to hour steps.

What is hard: every law must take the cell size and the step as inputs (moving, meeting, taking, rain); splitting a
group must place its members by the ground and the habitat, with draws keyed by stable ids; joining loses detail, so
what is worth keeping (a structure, a part, an individual of a big brain) raises its cell's score.

## 3. Decisions

| # | Decision | Why |
|---|---|---|
| D1 | Go on toward the zoomable world (spec 13.1, option 1) | The owner's instruction |
| D2 | The thread rule stays: neither the count of threads nor of chunks changes the world (spec 12.5, CLAUDE.md 3.6) | The spec asks for it; the chunk is the flux register a border needs |
| D3 | One tree of cells, one life model, the step by the cell's size, rates of bodies per second | Section 2; spec 5.2, 7 |
| D4 | Refinement by an interest score under a budget of cells; the camera is a recorded input | Spec 3.8 and the world search |
| D5 | The live viewer (core as wasm in a worker) comes before M6 | Watching is principle 4; changes spec 8.4 |
| D6 | Ids come from the cell's serial (a number that never changes), not from its index | Stable under split and join, and needs no provisional ids in a parallel pass |
| D7 | The coarse and fine forms must give similar patterns (spec 5.2), checked by `tape/tests/scale.rs` | Spec 5.2 |

## 4. Work packages, in order

Every package with code is done only when CLAUDE.md 6.3 passes: build, all tests, clippy with no warning, the wasm
build (**the gate**). A pure clean-up also keeps the fingerprint and `series.csv` of short runs bit for bit. A change
of a law is its own commit with its effect on a long run.

| WP | Goal | Done when | Risk |
|---|---|---|---|
| 0 | The M3 watch report: the 64 x 64 roster for 300 years, with and without `--null` (`docs/reports/m3.md`) | merged | low |
| 1 | The decisions written into the spec (5.2, 6.5, 7, 8.4, 13.1) | merged | low |
| 2 | **Physics for tools**: a held part's material changes what an act achieves (one law of Bond) | the gate; a scene in `core/tests/bond.rs`; 300 roster years keep the books; null run compared | medium |
| 3 | **The short body** (one law change): keep the coat, `KEEP_T`, `COOL_T`, `frost_k`, the one take law | the gate; the books; kinds alive at year 300 | medium |
| 4 | **Ids by cell serial, one cell table with a remap** (clean-up) | the gate; bit for bit | medium |
| 5 | **The live viewer**: the core in a web worker | the gate; a small island steps in a browser; its speed | low to medium |
| 6 | **A sparse tree of cells**: the map stores only the cells that exist, any depth below the air block; tape frames as a raster of a window at its finest grain | the gate; bit for bit on today's sizes | medium |
| 7 | **Split and join of cells, Flow only, two sizes**: exact books (amounts, heat, a `relief` term for potential energy), recorded as events, on a small world without life | a round trip keeps every element and the energy within 1e-9; a resumed run equals an unbroken one | high |
| 8 | **The step by the size for Flow**: fine cells sub-step inside the month, the border by a flux register | the books; threads change nothing; fine and coarse climate within noise | high |
| 9 | **Rates of bodies per second** (law change): learning, discount and brain cost per second; every body law takes cell size and step as inputs (moving, meeting, sweep, spread) | the gate; a 2 km world runs as before within noise | medium |
| 10 | **Life across split and join**: a group splits by area and habitat, joins by `nodes::add`; parts and bonds go with the cell that holds their place | the books across a round trip; `scale.rs` for animals | high |
| 11 | **The interest score and the budget**; the camera as a recorded input; the floor on the step | the same seed and camera path give the same world | medium |
| 12 | **Down to 50 m and short steps where the score is high**; watch for the first part used, the first shelter | a watch report | research |
| 13 | **`analyze/`**: three detectors (lasting clusters, lasting lanes, repeated structures), fact cards | the gate | low |
| 14 | The planet as the coarse tree with refined places; then god mode; then the world search | later rounds | high |

WP0, WP2, WP4 and WP5 run side by side; WP3 follows WP2 (both change bodies).

## 5. Where each package stands

| WP | State |
|---|---|
| 0, 1 | Done: `docs/reports/m3.md`; the spec holds the decisions |
| 2 | Done: a held edge harder than what it breaks concentrates force; a body may pick up the ground's loose rock; what lies alone is buried (5 mm a year) |
| 3 | Done: no melting plateau and no height in bodies; little speed (two thirds of a body's month is its decision) |
| 4 | Done: ids by cell serial; `World::remap` moves every per-cell number in one place |
| 5 | Done: the core as wasm in a worker; zoom refines where the player looks |
| 6 | Done: a sparse tree of cells down to 1/2^24 of an air block, the relief below the terrain's samples a midpoint displacement |
| 7 | Done: split and join with exact books; the quarters take the world's own terrain, paid by each cell's relief |
| 8 | Done: Flow agrees between fine and coarse cells; only the bed's store sub-steps |
| 9 | Done: learning, discount and brain cost per second; acts take time, a mover eats for the rest of its step; a body decides at its cell's step and lives in sub-steps of a week; children are born through the update; a young node's members mature one by one |
| 10 | Done: life shares out and gathers across split and join |
| 11 | Done: refinement by an interest score under a budget (a cell's score falls as it splits, so it settles); the camera a recorded input; one linking a month |
| 12 | Done: `docs/reports/fine.md`: nothing new appears at fine grain; grain gives more decisions, not new kinds of decision. The gaps (shelter in the cold, building, tools) point at perception, memory and learning |
| 13 | Done: `analyze/` with three detectors |
| 14 | Later rounds |

Open items: parts pile up (2e4 to 3e5 on a 64 x 64 island by year 300; each costs work monthly); the animals' numbers
swing by orders of magnitude between seeds, by whether a boom of one kind comes; refinement still costs about 4.6 ms a
change; a settled 64 x 64 year costs about 1 s on 4 cores, 3 to 7 s with refinement; in a debug build one Flow test
rounds a store's heat to -3e-5 J.

The next decision is the owner's (spec 13.1): a fixed time on Choose (what bodies sense, remember and learn), or
the planet (WP14).
