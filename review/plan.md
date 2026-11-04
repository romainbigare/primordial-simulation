# The plan toward a world that zooms

For the owner. Simple English. This plan comes from a review of `main` against the spec (`docs/handover.md`), then an
adversarial review of that review. It replaces the order in `review/top-30.md`. Where it and the spec differ, the spec
is changed in the same work package and says so.

## 1. The diagnosis, in short

- The climate, the land and simple life work, and the books close.
- The detail went into the physiology of one body, not into space and time.
- The world has one grain of space (2 km squares, cells of 1, 2 or 4 squares fixed at the start) and the bodies one
  pace (a month). Nothing refines or collapses while a run goes on.
- A held part changes nothing a body does (`laws.md` 3.3), so tools cannot emerge at any grain. This is missing
  physics (spec 9.4), and it comes before any change of grain.
- Nothing has been watched since M3.

## 2. Decisions

| # | Decision | Why |
|---|---|---|
| D1 | Go on toward the zoomable world (spec 13.1, option 1), after the M3 watch report | The owner's instruction |
| D2 | The thread rule stays: neither the count of threads nor of chunks changes the world (spec 12.5, CLAUDE.md 3.6) | The spec asks for it; the chunk is also the shape a region with its own steps needs |
| D3 | How often a body decides is a property of the body, the same at every level, never of the level | Spec 5.2 and 7: the speed tools give the same patterns; spec 6.5 keeps the cost per decision |
| D4 | A refinement is triggered by a rule of the world, and every refinement is a recorded event. The camera may ask for one only as a recorded input, like a god action | Spec 3.8 (same seed, same history) and the M7 world search, which has no camera |
| D5 | One life model at every grain: population nodes and individuals as today, on cells of any size | Spec 7.1: one set of rules for nodes and individuals |
| D6 | The live viewer (core as wasm in a worker) comes before M6 | Watching is principle 4; the spec put it at M6 (8.4), this changes 8.4 |
| D7 | Whether a level finer than 2 km is worth it is decided by a probe on a branch (WP3), not assumed | The reviewer's point: version 1 died at 1 ha and a day (spec 2, lesson 5) |

## 3. Work packages, in order

Every package with code is done only when CLAUDE.md 6.3 passes: build, all tests, clippy with no warning, the wasm
build (**the gate**). A pure clean-up also keeps the fingerprint and `series.csv` of short runs bit for bit. A change
of a law is its own commit with its effect on a long run.

| WP | Goal | Done when | Risk |
|---|---|---|---|
| 0 | The M3 watch report: the 64 x 64 roster for 300 years, with and without `--null`; trails, imitation, gifts, rooms in cold times, held parts, with pictures (`docs/reports/m3.md`) | merged | low |
| 1 | The decisions above written into the spec (13.1, 7.3, 7.4, 8.4) | merged | low |
| 2 | **Physics for tools**: a held part's material changes what an act achieves (one law of Bond, physical or anchored numbers) | the gate; a scene in `core/tests/bond.rs`; 300 roster years keep the books; null run compared | medium |
| 3 | **Grain probe**, on a branch only: a small island of fine squares with a short body step against a 2 km control; seconds a year and the watch-fors | a table and pictures for the owner; nothing to `main` | medium |
| 4 | **The short body** (one law change): drop what costs and cannot matter at this grain; keep the coat, `KEEP_T`, `COOL_T`, `frost_k`, the one take law | the gate; the books; kinds alive at year 300 reported | medium |
| 5 | **Ids by cell serial**: each cell has a serial that never changes; ids of bodies and parts come from it (clean-up) | the gate; bit for bit | low |
| 6 | **One cell table with a remap**: the per-cell arrays behind one table that can remap old cells to new ones (clean-up) | the gate; bit for bit | medium |
| 7 | **Tape frames for a changing mesh**: a raster at the finest grain of a window, and frames of a watched region every month | the gate; `tape/tests/tape.rs` | low |
| 8 | **A finer static region** (if WP3 says yes): cells finer than 2 km inside a region fixed at the start | the gate; `tape/tests/scale.rs` extended to the region and to animals | medium |
| 9 | **A region with its own steps**: a region runs its Flow steps inside the month on its view; what crosses its edge lands at the month's end | the gate; the books; threads change nothing; climate and kinds with and without the region within noise | high |
| 10 | **Refine and collapse during a run**, at a month's start, by a rule, recorded; potential energy booked by a `relief` term; bodies by `nodes::Lot` and `rejoin` | the gate; a round trip keeps every element and the energy within 1e-9; a resumed run equals an unbroken one | high |
| 11 | **The live viewer**: the core in a web worker; the page steps a small island | the gate; a 22 x 22 island steps in a browser; its speed reported | low to medium |
| 12 | **`analyze/`**: three detectors (lasting clusters, lasting lanes, repeated structures), fact cards | the gate; its tests read WP0's tapes | low |
| 13 | The planet as the coarse level with refined regions; then god mode; then the world search | later rounds | high |

WP11 runs beside WP2 to WP4. WP5 and WP6 run beside WP2 and WP4 where they touch other files.
